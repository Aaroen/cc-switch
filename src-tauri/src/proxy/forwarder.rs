//! 请求转发器
//!
//! 负责将请求转发到上游Provider，支持重试和故障转移

use super::{
    error::*,
    failover_switch::FailoverSwitchManager,
    provider_router::ProviderRouter,
    providers::{get_adapter, ProviderAdapter},
    types::ProxyStatus,
    ProxyError,
};
use crate::{app_config::AppType, provider::Provider};
use reqwest::{Client, Response};
use serde_json::Value;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

pub struct ForwardResult {
    pub response: Response,
    pub provider: Provider,
}

pub struct ForwardError {
    pub error: ProxyError,
    pub provider: Option<Provider>,
}

pub struct RequestForwarder {
    client: Client,
    /// 共享的 ProviderRouter（持有熔断器状态）
    router: Arc<ProviderRouter>,
    /// 重试次数（语义取决于场景）：
    /// - 单 Provider：同一 Provider 内重试次数（指数退避）
    /// - 多 Provider（故障转移/分层）：每个“层级”（sort_index）内最多尝试次数（错误即切换到下一个轮询目标）
    max_retries: u8,
    status: Arc<RwLock<ProxyStatus>>,
    current_providers: Arc<RwLock<std::collections::HashMap<String, (String, String)>>>,
    /// 故障转移切换管理器
    failover_manager: Arc<FailoverSwitchManager>,
    /// AppHandle，用于发射事件和更新托盘
    app_handle: Option<tauri::AppHandle>,
    /// 请求开始时的"当前供应商 ID"（用于判断是否需要同步 UI/托盘）
    current_provider_id_at_start: String,
}

impl RequestForwarder {
    fn extract_model_from_body(body: &Value) -> Option<String> {
        body.get("model")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.to_string())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        router: Arc<ProviderRouter>,
        non_streaming_timeout: u64,
        max_retries: u8,
        status: Arc<RwLock<ProxyStatus>>,
        current_providers: Arc<RwLock<std::collections::HashMap<String, (String, String)>>>,
        failover_manager: Arc<FailoverSwitchManager>,
        app_handle: Option<tauri::AppHandle>,
        current_provider_id_at_start: String,
        _streaming_first_byte_timeout: u64,
        _streaming_idle_timeout: u64,
    ) -> Self {
        // 全局超时设置为 1800 秒（30 分钟），确保业务层超时配置能正常工作
        // 参考 Claude Code Hub 的 undici 全局超时设计
        const GLOBAL_TIMEOUT_SECS: u64 = 1800;

        let mut client_builder = Client::builder();
        if non_streaming_timeout > 0 {
            // 使用配置的非流式超时
            client_builder = client_builder.timeout(Duration::from_secs(non_streaming_timeout));
        } else {
            // 禁用超时时使用全局超时作为保底
            client_builder = client_builder.timeout(Duration::from_secs(GLOBAL_TIMEOUT_SECS));
        }

        let client = client_builder
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            router,
            max_retries,
            status,
            current_providers,
            failover_manager,
            app_handle,
            current_provider_id_at_start,
        }
    }

    /// 对单个 Provider 执行请求（带重试）
    ///
    /// 在同一个 Provider 上最多重试 max_retries 次，使用指数退避
    async fn forward_with_provider_retry(
        &self,
        provider: &Provider,
        endpoint: &str,
        body: &Value,
        headers: &axum::http::HeaderMap,
        adapter: &dyn ProviderAdapter,
    ) -> Result<Response, ProxyError> {
        let mut last_error = None;

        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                // 指数退避：100ms, 200ms, 400ms, ...
                let delay_ms = 100 * 2u64.pow(attempt as u32 - 1);
                log::debug!(
                    "[{}] 重试第 {}/{} 次（等待 {}ms）",
                    adapter.name(),
                    attempt,
                    self.max_retries,
                    delay_ms
                );
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            }

            match self
                .forward(provider, endpoint, body, headers, adapter)
                .await
            {
                Ok(response) => return Ok(response),
                Err(e) => {
                    // 只有“同一 Provider 内可重试”的错误才继续重试
                    if !self.should_retry_same_provider(&e) {
                        return Err(e);
                    }

                    log::debug!(
                        "[{}] Provider {} 第 {} 次请求失败: {}",
                        adapter.name(),
                        provider.name,
                        attempt + 1,
                        e
                    );
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or(ProxyError::MaxRetriesExceeded))
    }

    fn max_attempts_per_priority(&self) -> usize {
        // 与用户配置保持一致：0 表示不额外重试，但仍会有 1 次尝试
        std::cmp::max(1, self.max_retries as usize)
    }

    /// 转发请求（带故障转移）
    ///
    /// # Arguments
    /// * `app_type` - 应用类型
    /// * `endpoint` - API 端点
    /// * `body` - 请求体
    /// * `headers` - 请求头
    /// * `providers` - 已选择的 Provider 列表（由 RequestContext 提供，避免重复调用 select_providers）
    pub async fn forward_with_retry(
        &self,
        app_type: &AppType,
        endpoint: &str,
        body: Value,
        headers: axum::http::HeaderMap,
        providers: Vec<Provider>,
    ) -> Result<ForwardResult, ForwardError> {
        // 获取适配器
        let adapter = get_adapter(app_type);
        let app_type_str = app_type.as_str();
        let request_model = Self::extract_model_from_body(&body);

        if providers.is_empty() {
            return Err(ForwardError {
                error: ProxyError::NoAvailableProvider,
                provider: None,
            });
        }

        let total_provider_count = providers.len();

        log::debug!(
            "[{}] 故障转移链: {} 个可用供应商",
            app_type_str,
            total_provider_count
        );

        let mut last_error = None;
        let mut last_provider = None;
        let mut attempted_providers = 0usize;

        // 单 Provider 场景下跳过熔断器检查（故障转移关闭时）
        let bypass_circuit_breaker = providers.len() == 1;

        if bypass_circuit_breaker {
            // 故障转移关闭：保留“同一 Provider 内重试”
            let provider = providers
                .first()
                .expect("bypass_circuit_breaker implies non-empty providers");

            // 更新状态中的当前Provider信息
            {
                let mut status = self.status.write().await;
                status.current_provider = Some(provider.name.clone());
                status.current_provider_id = Some(provider.id.clone());
                status.total_requests += 1;
                status.last_request_at = Some(chrono::Utc::now().to_rfc3339());
            }

            let start = Instant::now();

            match self
                .forward_with_provider_retry(provider, endpoint, &body, &headers, adapter.as_ref())
                .await
            {
                Ok(response) => {
                    let latency = start.elapsed().as_millis() as u64;

                    // 成功：记录成功并更新熔断器
                    if let Err(e) = self
                        .router
                        .record_result(
                            &provider.id,
                            app_type_str,
                            false,
                            true,
                            None,
                        )
                        .await
                    {
                        log::warn!("Failed to record success: {e}");
                    }

                    // 更新当前应用类型使用的 provider
                    {
                        let mut current_providers = self.current_providers.write().await;
                        current_providers.insert(
                            app_type_str.to_string(),
                            (provider.id.clone(), provider.name.clone()),
                        );
                    }

                    // 更新成功统计
                    {
                        let mut status = self.status.write().await;
                        status.success_requests += 1;
                        status.last_error = None;
                        let should_switch =
                            self.current_provider_id_at_start.as_str() != provider.id.as_str();
                        if should_switch {
                            status.failover_count += 1;

                            // 异步触发供应商切换，更新 UI/托盘，并把"当前供应商"同步为实际使用的 provider
                            let fm = self.failover_manager.clone();
                            let ah = self.app_handle.clone();
                            let pid = provider.id.clone();
                            let pname = provider.name.clone();
                            let at = app_type_str.to_string();

                            tokio::spawn(async move {
                                if let Err(e) = fm.try_switch(ah.as_ref(), &at, &pid, &pname).await
                                {
                                    log::error!("[Failover] 切换供应商失败: {e}");
                                }
                            });
                        }
                        // 重新计算成功率
                        if status.total_requests > 0 {
                            status.success_rate = (status.success_requests as f32
                                / status.total_requests as f32)
                                * 100.0;
                        }
                    }

                    // 统一日志：一次请求仅记录一条
                    log::info!(
                        "正常 200 - {} (耗时: {}ms)",
                        provider.name,
                        latency
                    );

                    // startup 测试覆盖：记录这一次真实请求的结果（用于 csc t startup）
                    self.router
                        .maybe_record_startup_test_from_forwarder(
                            app_type_str,
                            &provider,
                            request_model.as_deref(),
                            latency,
                            Some(response.status().as_u16()),
                            None,
                        )
                        .await;

                    return Ok(ForwardResult {
                        response,
                        provider: provider.clone(),
                    });
                }
                Err(e) => {
                    let latency = start.elapsed().as_millis() as u64;
                    let is_startup_test = self.router.has_active_test_override(app_type_str).await;
                    let e_text = e.to_string();

                    // 失败：记录失败并更新熔断器
                    if let Err(record_err) = self
                        .router
                        .record_result(
                            &provider.id,
                            app_type_str,
                            false,
                            false,
                            Some(e_text.clone()),
                        )
                        .await
                    {
                        log::warn!("Failed to record failure: {record_err}");
                    }

                    // 分类错误
                    let category = self.categorize_proxy_error(&e);

                    match category {
                        ErrorCategory::Retryable => {
                            // 可重试：更新错误信息，继续尝试下一个供应商
                            {
                                let mut status = self.status.write().await;
                                status.last_error =
                                    Some(format!("Provider {} 失败: {}", provider.name, e));
                            }

                            log::debug!(
                                "[{}] Provider {} 失败（可重试）: {} - {}ms",
                                app_type_str,
                                provider.name,
                                e,
                                latency
                            );

                            last_error = Some(e);
                            last_provider = Some(provider.clone());
                            // 单 Provider 场景：此处已经做过同 Provider 重试，直接返回最终错误
                            {
                                let mut status = self.status.write().await;
                                status.failed_requests += 1;
                                if status.total_requests > 0 {
                                    status.success_rate = (status.success_requests as f32
                                        / status.total_requests as f32)
                                        * 100.0;
                                }
                            }
                            // startup 测试覆盖：即使被归类为可重试，也需要立刻回传具体错误，避免 CLI 超时
                            if is_startup_test {
                                self.router
                                    .maybe_record_startup_test_from_forwarder(
                                        app_type_str,
                                        &provider,
                                        request_model.as_deref(),
                                        latency,
                                        None,
                                        Some(e_text.clone()),
                                    )
                                    .await;
                            }
                            return Err(ForwardError {
                                error: last_error.unwrap_or(ProxyError::MaxRetriesExceeded),
                                provider: last_provider,
                            });
                        }
                        ErrorCategory::NonRetryable | ErrorCategory::ClientAbort => {
                            // 不可重试：直接返回错误
                            {
                                let mut status = self.status.write().await;
                                status.failed_requests += 1;
                                status.last_error = Some(e.to_string());
                                if status.total_requests > 0 {
                                    status.success_rate = (status.success_requests as f32
                                        / status.total_requests as f32)
                                        * 100.0;
                                }
                            }
                            // startup 测试覆盖：回传具体错误到 CLI（用于终端展示）
                            if is_startup_test {
                                self.router
                                    .maybe_record_startup_test_from_forwarder(
                                        app_type_str,
                                        &provider,
                                        request_model.as_deref(),
                                        latency,
                                        None,
                                        Some(e_text.clone()),
                                    )
                                    .await;
                            }
                            log::error!(
                                "[{}] Provider {} 失败（不可重试）: {}",
                                app_type_str,
                                provider.name,
                                e
                            );
                            return Err(ForwardError {
                                error: e,
                                provider: Some(provider.clone()),
                            });
                        }
                    }
                }
            }
        } else {
            // 故障转移开启：按 sort_index（层级）分组；
            // 在一个层级内最多“全量轮询”N 轮（N=self.max_retries，至少 1 轮），每次报错立刻切换到下一个轮询目标；
            // 当本层级 N 轮全部失败后，才进入下一层级。
            let mut by_priority: std::collections::BTreeMap<usize, Vec<Provider>> =
                std::collections::BTreeMap::new();
            for p in providers.into_iter() {
                let priority = p.sort_index.unwrap_or(999999);
                by_priority.entry(priority).or_default().push(p);
            }

            let rounds_per_priority = self.max_attempts_per_priority();

            for (priority, providers_in_level) in by_priority.into_iter() {
                if providers_in_level.is_empty() {
                    continue;
                }

                let mut attempts_executed = 0usize;

                for round in 0..rounds_per_priority {
                    let mut skipped_by_circuit = 0usize;

                    for provider in providers_in_level.iter() {
                        // 发起请求前先获取熔断器放行许可（HalfOpen 会占用探测名额）
                        let permit = self
                            .router
                            .allow_provider_request(&provider.id, app_type_str)
                            .await;

                        if !permit.allowed {
                            skipped_by_circuit += 1;
                            continue;
                        }

                        attempted_providers += 1;
                        attempts_executed += 1;

                        log::debug!(
                            "[{}] 层级 {} 第 {}/{} 轮 - 使用Provider: {}",
                            app_type_str,
                            priority,
                            round + 1,
                            rounds_per_priority,
                            provider.name
                        );

                        // 更新状态中的当前Provider信息
                        {
                            let mut status = self.status.write().await;
                            status.current_provider = Some(provider.name.clone());
                            status.current_provider_id = Some(provider.id.clone());
                            status.total_requests += 1;
                            status.last_request_at = Some(chrono::Utc::now().to_rfc3339());
                        }

                        let start = Instant::now();

                        // 多 Provider：错误即切换，不做“同 Provider 内重试”
                        match self
                            .forward(provider, endpoint, &body, &headers, adapter.as_ref())
                            .await
                        {
                            Ok(response) => {
                                let latency = start.elapsed().as_millis() as u64;

                                if let Err(e) = self
                                    .router
                                    .record_result(
                                        &provider.id,
                                        app_type_str,
                                        permit.used_half_open_permit,
                                        true,
                                        None,
                                    )
                                    .await
                                {
                                    log::warn!("Failed to record success: {e}");
                                }

                                // 更新当前应用类型使用的 provider
                                {
                                    let mut current_providers = self.current_providers.write().await;
                                    current_providers.insert(
                                        app_type_str.to_string(),
                                        (provider.id.clone(), provider.name.clone()),
                                    );
                                }

                                // 更新成功统计
                                {
                                    let mut status = self.status.write().await;
                                    status.success_requests += 1;
                                    status.last_error = None;
                                    let should_switch = self.current_provider_id_at_start.as_str()
                                        != provider.id.as_str();
                                    if should_switch {
                                        status.failover_count += 1;

                                        let fm = self.failover_manager.clone();
                                        let ah = self.app_handle.clone();
                                        let pid = provider.id.clone();
                                        let pname = provider.name.clone();
                                        let at = app_type_str.to_string();

                                        tokio::spawn(async move {
                                            if let Err(e) =
                                                fm.try_switch(ah.as_ref(), &at, &pid, &pname).await
                                            {
                                                log::error!("[Failover] 切换供应商失败: {e}");
                                            }
                                        });
                                    }
                                    if status.total_requests > 0 {
                                        status.success_rate = (status.success_requests as f32
                                            / status.total_requests as f32)
                                            * 100.0;
                                    }
                                }

                                log::info!(
                                    "正常 200 - {} (耗时: {}ms)",
                                    provider.name,
                                    latency
                                );

                                // startup 测试覆盖：记录这一次真实请求的结果（用于 csc t startup）
                                self.router
                                    .maybe_record_startup_test_from_forwarder(
                                        app_type_str,
                                        &provider,
                                        request_model.as_deref(),
                                        latency,
                                        Some(response.status().as_u16()),
                                        None,
                                    )
                                    .await;

                                return Ok(ForwardResult {
                                    response,
                                    provider: provider.clone(),
                                });
                            },
                            Err(e) => {
                                let latency = start.elapsed().as_millis() as u64;

                                if let Err(record_err) = self
                                    .router
                                    .record_result(
                                        &provider.id,
                                        app_type_str,
                                        permit.used_half_open_permit,
                                        false,
                                        Some(e.to_string()),
                                    )
                                    .await
                                {
                                    log::warn!("Failed to record failure: {record_err}");
                                }

                                let category = self.categorize_proxy_error(&e);

                                match category {
                                    ErrorCategory::Retryable => {
                                        // startup 测试覆盖：首错即停（避免日志刷屏/重复尝试），并把具体错误回传给 CLI
                                        if self.router.has_active_test_override(app_type_str).await {
                                            self.router
                                                .maybe_record_startup_test_from_forwarder(
                                                    app_type_str,
                                                    &provider,
                                                    request_model.as_deref(),
                                                    latency,
                                                    None,
                                                    Some(e.to_string()),
                                                )
                                                .await;
                                            return Err(ForwardError {
                                                error: e,
                                                provider: Some(provider.clone()),
                                            });
                                        }
                                        {
                                            let mut status = self.status.write().await;
                                            status.last_error = Some(format!(
                                                "Provider {} 失败: {}",
                                                provider.name, e
                                            ));
                                        }

                                        log::debug!(
                                            "[{}] Provider {} 失败（可重试，切换下一个轮询目标）: {} - {}ms",
                                            app_type_str,
                                            provider.name,
                                            e,
                                            latency
                                        );

                                        last_error = Some(e);
                                        last_provider = Some(provider.clone());
                                        continue;
                                    }
                                    ErrorCategory::NonRetryable | ErrorCategory::ClientAbort => {
                                        {
                                            let mut status = self.status.write().await;
                                            status.failed_requests += 1;
                                            status.last_error = Some(e.to_string());
                                            if status.total_requests > 0 {
                                                status.success_rate = (status.success_requests as f32
                                                    / status.total_requests as f32)
                                                    * 100.0;
                                            }
                                        }
                                        // startup 测试覆盖：仅在“最终失败”时记录（避免 key 内重试导致误判）
                                        self.router
                                            .maybe_record_startup_test_from_forwarder(
                                                app_type_str,
                                                &provider,
                                                request_model.as_deref(),
                                                latency,
                                                None,
                                                Some(e.to_string()),
                                            )
                                            .await;
                                        log::error!(
                                            "[{}] Provider {} 失败（不可重试）: {}",
                                            app_type_str,
                                            provider.name,
                                            e
                                        );
                                        return Err(ForwardError {
                                            error: e,
                                            provider: Some(provider.clone()),
                                        });
                                    }
                                }
                            }
                        }

                    }

                    // 防止“整个层级都被熔断器拒绝”时空转
                    if skipped_by_circuit >= providers_in_level.len() {
                        break;
                    }
                }

                if attempts_executed == 0 {
                    log::debug!(
                        "[{}] 层级 {} 无可用供应商（可能被熔断器限制），尝试下一层级",
                        app_type_str,
                        priority
                    );
                } else {
                    log::warn!(
                        "[{}] 层级 {} 已用尽尝试轮次（{} 轮），切换到下一层级",
                        app_type_str,
                        priority,
                        rounds_per_priority
                    );
                }
            }
        }

        if attempted_providers == 0 {
            // providers 列表非空，但全部被熔断器拒绝（典型：HalfOpen 探测名额被占用）
            {
                let mut status = self.status.write().await;
                status.failed_requests += 1;
                status.last_error = Some("所有供应商暂时不可用（熔断器限制）".to_string());
                if status.total_requests > 0 {
                    status.success_rate =
                        (status.success_requests as f32 / status.total_requests as f32) * 100.0;
                }
            }
            return Err(ForwardError {
                error: ProxyError::NoAvailableProvider,
                provider: None,
            });
        }

        // 所有供应商都失败了
        {
            let mut status = self.status.write().await;
            status.failed_requests += 1;
            status.last_error = Some("所有供应商都失败".to_string());
            if status.total_requests > 0 {
                status.success_rate =
                    (status.success_requests as f32 / status.total_requests as f32) * 100.0;
            }
        }

        log::error!(
            "[{}] 所有 {} 个供应商都失败了",
            app_type_str,
            total_provider_count
        );

        Err(ForwardError {
            error: last_error.unwrap_or(ProxyError::MaxRetriesExceeded),
            provider: last_provider,
        })
    }

    /// 转发单个请求（使用适配器）
    async fn forward(
        &self,
        provider: &Provider,
        endpoint: &str,
        body: &Value,
        headers: &axum::http::HeaderMap,
        adapter: &dyn ProviderAdapter,
    ) -> Result<Response, ProxyError> {
        // 提取 API Key
        let auth = adapter.extract_auth(provider).ok_or_else(|| {
            ProxyError::AuthError(format!("Provider {} 缺少认证信息", provider.id))
        })?;

        // 根据adapter类型选择转发目标（并保留 base_url 便于错误日志定位）
        let (url, target_description, upstream_base_url) = if adapter.name() == "Codex" {
            // Codex直接转发到目标URL
            let base_url = adapter.extract_base_url(provider)?;
            let full_url = adapter.build_url(&base_url, endpoint);
            (full_url, format!("{}", base_url), Some(base_url))
        } else {
            // Claude通过Python代理
            let url = format!("http://127.0.0.1:15722{}", endpoint);
            let base_url = provider
                .settings_config
                .get("env")
                .and_then(|env| env.get("ANTHROPIC_BASE_URL"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            (url, "Python代理(15722)".to_string(), base_url)
        };

        // 构建请求
        let mut request = self.client.post(&url);

        // 只透传必要的 Headers（白名单模式）
        let allowed_headers = [
            "accept",
            "user-agent",
            "x-request-id",
            "x-stainless-arch",
            "x-stainless-lang",
            "x-stainless-os",
            "x-stainless-package-version",
            "x-stainless-runtime",
            "x-stainless-runtime-version",
        ];

        for (key, value) in headers {
            let key_str = key.as_str().to_lowercase();
            if allowed_headers.contains(&key_str.as_str()) {
                request = request.header(key, value);
            }
        }

        // 确保 Content-Type 是 json
        request = request.header("Content-Type", "application/json");

        // 根据转发目标添加不同的认证头部
        if adapter.name() == "Codex" {
            // Codex使用标准OpenAI格式：Authorization: Bearer <token>
            request = request.header("Authorization", format!("Bearer {}", auth.api_key));
        } else {
            // Claude通过Python代理，需要添加X-API-Key和x-target-base-url
            let base_url = provider
                .settings_config
                .get("env")
                .and_then(|env| env.get("ANTHROPIC_BASE_URL"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    ProxyError::ConfigError(format!(
                        "Provider {} 缺少ANTHROPIC_BASE_URL配置",
                        provider.id
                    ))
                })?;

            request = request.header("X-API-Key", &auth.api_key);
            request = request.header("x-target-base-url", base_url);
        }

        // 发送请求
        let response = request.json(body).send().await.map_err(|e| {
            log::error!(
                "错误 - {} - target={} base_url={} - 详情: 请求失败 {}",
                provider.name,
                target_description,
                upstream_base_url.as_deref().unwrap_or("-"),
                e
            );
            if e.is_timeout() {
                ProxyError::Timeout(format!("请求超时: {e}"))
            } else if e.is_connect() {
                ProxyError::ForwardFailed(format!("连接失败: {e}"))
            } else {
                ProxyError::ForwardFailed(e.to_string())
            }
        })?;

        // 检查响应状态
        let status = response.status();

        if status.is_success() {
            Ok(response)
        } else {
            let status_code = status.as_u16();
            let body_text = response.text().await.ok();
            log::error!(
                "错误 {} - {} - base_url={} - 详情: {:?}",
                status_code,
                provider.name,
                upstream_base_url.as_deref().unwrap_or("-"),
                body_text
            );

            Err(ProxyError::UpstreamError {
                status: status_code,
                body: body_text,
            })
        }
    }

    /// 分类ProxyError
    ///
    /// 决定哪些错误应该触发故障转移到下一个 Provider
    ///
    /// 设计原则：既然用户配置了多个供应商，就应该让所有供应商都尝试一遍。
    /// 只有明确是客户端中断的情况才不重试。
    fn should_retry_same_provider(&self, error: &ProxyError) -> bool {
        match error {
            // 网络类错误：短暂抖动时同一 Provider 内重试有意义
            ProxyError::Timeout(_) => true,
            ProxyError::ForwardFailed(_) => true,
            // 上游 HTTP 错误：只对“可能瞬态”的状态码做同 Provider 重试（其余交给 failover）
            ProxyError::UpstreamError { status, .. } => {
                *status == 408 || *status == 429 || *status >= 500
            }
            _ => false,
        }
    }

    fn categorize_proxy_error(&self, error: &ProxyError) -> ErrorCategory {
        match error {
            // 网络和上游错误：都应该尝试下一个供应商
            ProxyError::Timeout(_) => ErrorCategory::Retryable,
            ProxyError::ForwardFailed(_) => ErrorCategory::Retryable,
            ProxyError::ProviderUnhealthy(_) => ErrorCategory::Retryable,
            // 上游 HTTP 错误：无论状态码如何，都尝试下一个供应商
            // 原因：不同供应商有不同的限制和认证，一个供应商的 4xx 错误
            // 不代表其他供应商也会失败
            ProxyError::UpstreamError { .. } => ErrorCategory::Retryable,
            // Provider 级配置/转换问题：换一个 Provider 可能就能成功
            ProxyError::ConfigError(_) => ErrorCategory::Retryable,
            ProxyError::TransformError(_) => ErrorCategory::Retryable,
            ProxyError::AuthError(_) => ErrorCategory::Retryable,
            ProxyError::StreamIdleTimeout(_) => ErrorCategory::Retryable,
            ProxyError::MaxRetriesExceeded => ErrorCategory::Retryable,
            // 无可用供应商：所有供应商都试过了，无法重试
            ProxyError::NoAvailableProvider => ErrorCategory::NonRetryable,
            // 其他错误（数据库/内部错误等）：不是换供应商能解决的问题
            _ => ErrorCategory::NonRetryable,
        }
    }
}
