//! 供应商路由器模块
//!
//! 负责选择和管理代理目标供应商，实现智能故障转移

use crate::database::Database;
use crate::error::AppError;
use crate::provider::Provider;
use crate::proxy::circuit_breaker::{AllowResult, CircuitBreaker, CircuitBreakerConfig};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::sync::RwLock;

/// URL延迟测试结果
#[derive(Debug, Clone)]
struct UrlLatency {
    /// 延迟时间(毫秒)
    latency_ms: u64,
    /// 测试时间戳
    tested_at: std::time::Instant,
}

#[derive(Debug, Clone)]
pub struct UrlProbeDetail {
    pub url: String,
    pub kind: UrlProbeKind,
}

#[derive(Debug, Clone)]
pub enum UrlProbeKind {
    FullOk { latency_ms: u64 },
    Overloaded { latency_ms: u64, message: String },
    FallbackOk {
        connect_ms: u64,
        penalty_ms: u64,
        reason: String,
    },
    Failed { reason: String },
}

#[derive(Debug, Clone)]
struct UrlProbeError {
    latency_ms: u64,
    kind: UrlProbeErrorKind,
}

#[derive(Debug, Clone)]
enum UrlProbeErrorKind {
    Overloaded { message: String },
    Http { status: u16, body: Option<String> },
    Network { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkUrlResult {
    pub url: String,
    /// OK / OV / FB / FAIL
    pub kind: String,
    pub latency_ms: Option<u64>,
    pub penalty_ms: Option<u64>,
    pub message: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSupplierResult {
    pub priority: usize,
    pub supplier: String,
    pub chosen_url: Option<String>,
    /// OK / OV / FB / FAIL / COOLDOWN
    pub chosen_kind: String,
    pub metric_ms: Option<u64>,
    pub urls: Vec<BenchmarkUrlResult>,
}

/// 供应商路由器
pub struct ProviderRouter {
    /// 数据库连接
    db: Arc<Database>,
    /// 熔断器管理器 - key 格式: "app_type:provider_id"
    circuit_breakers: Arc<RwLock<HashMap<String, Arc<CircuitBreaker>>>>,
    /// URL内轮询计数器 - key 格式: "app_type:priority:层级", value: 当前索引
    round_robin_counters: Arc<RwLock<HashMap<String, usize>>>,
    /// 当前激活层级 - key 格式: "app_type", value: 当前使用的优先级层级
    active_priority_level: Arc<RwLock<HashMap<String, usize>>>,
    /// 供应商URL已测试标记 - key 格式: "app_type:priority:supplier", value: 是否已测试过URL延迟
    priority_level_tested: Arc<RwLock<HashMap<String, bool>>>,
    /// URL延迟缓存 - key 格式: "app_type:priority:supplier:base_url", value: 延迟测试结果
    url_latencies: Arc<RwLock<HashMap<String, UrlLatency>>>,
    /// 供应商冷静期 - key 格式: "app_type:priority:supplier", value: 冷静期结束时间
    supplier_cooldowns: Arc<RwLock<HashMap<String, std::time::Instant>>>,
    /// URL 疑似失效标记 - key 格式: "app_type:supplier:base_url", value: 解除时间
    suspect_urls: Arc<RwLock<HashMap<String, std::time::Instant>>>,
    /// 每个供应商当前选中的 URL（同一时刻只使用一个“最快 URL”）
    /// key 格式: "app_type:priority:supplier", value: base_url
    supplier_current_url: Arc<RwLock<HashMap<String, String>>>,
    /// 供应商测速锁（避免并发请求触发重复测速）
    /// key 格式: "app_type:priority:supplier"
    supplier_benchmark_locks: Arc<RwLock<HashMap<String, Arc<Mutex<()>>>>>,
    /// 启动即测速（保底）模式下的测试覆盖：用于将下一次（或短时间内）请求强制路由到指定 supplier
    test_override: Arc<RwLock<Option<TestOverride>>>,
    /// 测试结果（run_id -> result），供 CLI 轮询读取
    test_results: Arc<RwLock<HashMap<String, BenchmarkSupplierResult>>>,
}

#[derive(Debug, Clone)]
struct TestOverride {
    app_type: String,
    priority: usize,
    supplier: String,
    run_id: String,
    expires_at: std::time::Instant,
}

impl ProviderRouter {
    const CONNECTIVITY_TIMEOUT: Duration = Duration::from_secs(5);
    const CONNECTIVITY_PENALTY_MS: u64 = 30_000;
    const DEFAULT_BENCHMARK_SUMMARY_INFO_ENV: &'static str = "CC_SWITCH_BENCHMARK_SUMMARY";

    /// 创建新的供应商路由器
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db,
            circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
            round_robin_counters: Arc::new(RwLock::new(HashMap::new())),
            active_priority_level: Arc::new(RwLock::new(HashMap::new())),
            priority_level_tested: Arc::new(RwLock::new(HashMap::new())),
            url_latencies: Arc::new(RwLock::new(HashMap::new())),
            supplier_cooldowns: Arc::new(RwLock::new(HashMap::new())),
            suspect_urls: Arc::new(RwLock::new(HashMap::new())),
            supplier_current_url: Arc::new(RwLock::new(HashMap::new())),
            supplier_benchmark_locks: Arc::new(RwLock::new(HashMap::new())),
            test_override: Arc::new(RwLock::new(None)),
            test_results: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    #[inline]
    fn supplier_key(app_type: &str, priority: usize, supplier: &str) -> String {
        format!("{app_type}:{priority}:{supplier}")
    }

    #[inline]
    fn url_latency_key(app_type: &str, priority: usize, supplier: &str, url: &str) -> String {
        format!("{app_type}:{priority}:{supplier}:{url}")
    }

    async fn get_supplier_current_url(
        &self,
        app_type: &str,
        priority: usize,
        supplier: &str,
    ) -> Option<String> {
        let key = Self::supplier_key(app_type, priority, supplier);
        let map = self.supplier_current_url.read().await;
        map.get(&key).cloned()
    }

    async fn set_supplier_current_url(
        &self,
        app_type: &str,
        priority: usize,
        supplier: &str,
        url: &str,
    ) {
        let key = Self::supplier_key(app_type, priority, supplier);
        let mut map = self.supplier_current_url.write().await;
        map.insert(key, url.to_string());
    }

    async fn clear_supplier_current_url(&self, app_type: &str, priority: usize, supplier: &str) {
        let key = Self::supplier_key(app_type, priority, supplier);
        let mut map = self.supplier_current_url.write().await;
        map.remove(&key);
    }

    async fn get_active_test_override(&self, app_type: &str) -> Option<TestOverride> {
        let mut guard = self.test_override.write().await;
        if let Some(o) = guard.as_ref() {
            if o.app_type == app_type && std::time::Instant::now() < o.expires_at {
                return Some(o.clone());
            }
        }
        // 过期清理
        *guard = None;
        None
    }

    pub async fn set_test_override(
        &self,
        app_type: &str,
        priority: usize,
        supplier: &str,
        run_id: &str,
        ttl_secs: u64,
    ) {
        // 为了保证触发 benchmark：清空该 supplier 的 “已测试” 与 “current_url” 状态
        self.clear_supplier_current_url(app_type, priority, supplier).await;
        {
            let key = Self::supplier_key(app_type, priority, supplier);
            let mut tested_map = self.priority_level_tested.write().await;
            tested_map.remove(&key);
        }

        {
            let mut results = self.test_results.write().await;
            results.remove(run_id);
        }

        let override_state = TestOverride {
            app_type: app_type.to_string(),
            priority,
            supplier: supplier.to_string(),
            run_id: run_id.to_string(),
            expires_at: std::time::Instant::now() + std::time::Duration::from_secs(ttl_secs),
        };
        *self.test_override.write().await = Some(override_state);
    }

    pub async fn get_test_result(&self, run_id: &str) -> Option<BenchmarkSupplierResult> {
        let map = self.test_results.read().await;
        map.get(run_id).cloned()
    }

    fn details_to_benchmark_url_results(details: &[UrlProbeDetail]) -> Vec<BenchmarkUrlResult> {
        details
            .iter()
            .map(|d| {
                let (kind, latency_ms, penalty_ms, message, reason) = match &d.kind {
                    UrlProbeKind::FullOk { latency_ms } => (
                        "OK".to_string(),
                        Some(*latency_ms),
                        None,
                        None,
                        None,
                    ),
                    UrlProbeKind::Overloaded { latency_ms, message } => (
                        "OV".to_string(),
                        Some(*latency_ms),
                        Some(Self::CONNECTIVITY_PENALTY_MS),
                        Some(message.clone()),
                        None,
                    ),
                    UrlProbeKind::FallbackOk {
                        connect_ms,
                        penalty_ms,
                        reason,
                    } => (
                        "FB".to_string(),
                        Some(*connect_ms),
                        Some(*penalty_ms),
                        None,
                        Some(reason.clone()),
                    ),
                    UrlProbeKind::Failed { reason } => (
                        "FAIL".to_string(),
                        None,
                        None,
                        None,
                        Some(reason.clone()),
                    ),
                };

                BenchmarkUrlResult {
                    url: d.url.clone(),
                    kind,
                    latency_ms,
                    penalty_ms,
                    message,
                    reason,
                }
            })
            .collect()
    }

    async fn get_supplier_benchmark_lock(
        &self,
        app_type: &str,
        priority: usize,
        supplier: &str,
    ) -> Arc<Mutex<()>> {
        let key = Self::supplier_key(app_type, priority, supplier);

        {
            let map = self.supplier_benchmark_locks.read().await;
            if let Some(lock) = map.get(&key) {
                return lock.clone();
            }
        }

        let mut map = self.supplier_benchmark_locks.write().await;
        map.entry(key).or_insert_with(|| Arc::new(Mutex::new(()))).clone()
    }

    fn is_likely_network_error(err: &str) -> bool {
        err.contains("超时")
            || err.contains("连接失败")
            || err.contains("Connection refused")
            || err.contains("connection refused")
            || err.contains("dns")
            || err.contains("DNS")
            || err.contains("timed out")
            || err.contains("error sending request")
            || err.contains("connection closed")
            || err.contains("Upstream request failed")
            || err.contains("请求转发失败: error")
            || err.contains("请求转发失败: timed out")
            || err.contains("请求转发失败: Connection refused")
    }

    fn default_url_priority_for_supplier(supplier: &str) -> Vec<&'static str> {
        match supplier.to_lowercase().as_str() {
            // 用户需求：anyrouter 的 https://anyrouter.top 可用时优先使用
            "anyrouter" => vec!["https://anyrouter.top"],
            _ => Vec::new(),
        }
    }

    fn parse_url_priority_from_provider(provider: &Provider) -> Vec<String> {
        // 支持两种配置方式：
        // 1) settingsConfig.root: baseUrlPriority / base_url_priority (array 或 string)
        // 2) settingsConfig.env: BASE_URL_PRIORITY（逗号分隔）
        let mut out: Vec<String> = Vec::new();

        let from_root = provider
            .settings_config
            .get("baseUrlPriority")
            .or_else(|| provider.settings_config.get("base_url_priority"));

        if let Some(v) = from_root {
            if let Some(arr) = v.as_array() {
                for item in arr {
                    if let Some(s) = item.as_str() {
                        let s = s.trim();
                        if !s.is_empty() {
                            out.push(s.to_string());
                        }
                    }
                }
            } else if let Some(s) = v.as_str() {
                for part in s.split(',') {
                    let p = part.trim();
                    if !p.is_empty() {
                        out.push(p.to_string());
                    }
                }
            }
        }

        if let Some(s) = provider
            .settings_config
            .get("env")
            .and_then(|env| env.get("BASE_URL_PRIORITY"))
            .and_then(|v| v.as_str())
        {
            for part in s.split(',') {
                let p = part.trim();
                if !p.is_empty() {
                    out.push(p.to_string());
                }
            }
        }

        // 去重，保留顺序
        let mut seen = std::collections::HashMap::<String, ()>::new();
        out.retain(|u| seen.insert(u.to_string(), ()).is_none());
        out
    }

    fn apply_url_priority(mut urls: Vec<String>, priority: &[String]) -> Vec<String> {
        if priority.is_empty() || urls.is_empty() {
            return urls;
        }
        let mut picked = Vec::with_capacity(urls.len());
        for p in priority {
            if let Some(pos) = urls.iter().position(|u| u == p) {
                picked.push(urls.remove(pos));
            }
        }
        picked.extend(urls);
        picked
    }

    fn shorten_for_log(text: &str, max_chars: usize) -> String {
        if max_chars == 0 {
            return String::new();
        }
        let mut out = String::new();
        for (i, ch) in text.chars().enumerate() {
            if i >= max_chars {
                out.push_str("…");
                break;
            }
            out.push(ch);
        }
        out
    }

    fn should_log_benchmark_summary_info() -> bool {
        std::env::var(Self::DEFAULT_BENCHMARK_SUMMARY_INFO_ENV)
            .ok()
            .as_deref()
            == Some("1")
    }

    fn is_overloaded_error_text(text: &str) -> bool {
        // 常见“可达但不可用”的提示（满载/限流/暂不可用）
        text.contains("负载已经达到上限")
            || text.contains("满载")
            || text.contains("rate limit")
            || text.contains("Rate limit")
            || text.contains("Too Many Requests")
            || text.contains("temporarily unavailable")
    }

    fn extract_error_message_from_body(body: &str) -> Option<String> {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(body) else {
            return None;
        };

        // 兼容多种错误结构
        if let Some(msg) = v
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
        {
            return Some(msg.to_string());
        }

        if let Some(msg) = v.get("message").and_then(|m| m.as_str()) {
            return Some(msg.to_string());
        }

        None
    }


    fn supplier_name(provider: &Provider) -> String {
        provider
            .name
            .split('-')
            .next()
            .unwrap_or(&provider.name)
            .to_string()
    }

    fn extract_base_url(provider: &Provider, app_type: &str) -> Option<String> {
        match app_type {
            "claude" => provider
                .settings_config
                .get("env")
                .and_then(|env| env.get("ANTHROPIC_BASE_URL"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            "gemini" => provider
                .settings_config
                .get("env")
                .and_then(|env| env.get("GOOGLE_GEMINI_BASE_URL"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            "codex" => provider
                .settings_config
                .get("base_url")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            _ => None,
        }
    }

    fn extract_api_key_value(provider: &Provider, app_type: &str) -> Option<String> {
        match app_type {
            "claude" => provider
                .settings_config
                .get("env")
                .and_then(|env| {
                    env.get("ANTHROPIC_API_KEY")
                        .or_else(|| env.get("ANTHROPIC_AUTH_TOKEN"))
                })
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            "gemini" => provider
                .settings_config
                .get("env")
                .and_then(|env| env.get("GOOGLE_API_KEY"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            "codex" => provider
                .settings_config
                .get("env")
                .and_then(|env| env.get("OPENAI_API_KEY"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            _ => None,
        }
    }

    async fn is_url_suspect(&self, app_type: &str, supplier: &str, url: &str) -> bool {
        let now = std::time::Instant::now();
        let key = format!("{app_type}:{supplier}:{url}");
        let mut map = self.suspect_urls.write().await;

        match map.get(&key).copied() {
            Some(until) if until > now => true,
            Some(_) => {
                map.remove(&key);
                false
            }
            None => false,
        }
    }

    async fn set_url_suspect(&self, app_type: &str, supplier: &str, url: &str, seconds: u64) {
        let key = format!("{app_type}:{supplier}:{url}");
        let until = std::time::Instant::now() + std::time::Duration::from_secs(seconds);
        let mut map = self.suspect_urls.write().await;
        map.insert(key, until);
    }

    async fn is_supplier_in_cooldown(&self, app_type: &str, priority: usize, supplier: &str) -> bool {
        let now = std::time::Instant::now();
        let key = format!("{app_type}:{priority}:{supplier}");
        let mut map = self.supplier_cooldowns.write().await;
        match map.get(&key).copied() {
            Some(until) if until > now => true,
            Some(_) => {
                map.remove(&key);
                false
            }
            None => false,
        }
    }

    async fn set_supplier_cooldown(&self, app_type: &str, priority: usize, supplier: &str, seconds: u64) {
        let key = format!("{app_type}:{priority}:{supplier}");
        let until = std::time::Instant::now() + std::time::Duration::from_secs(seconds);
        let mut map = self.supplier_cooldowns.write().await;
        map.insert(key, until);
    }

    /// 选择可用的供应商（支持故障转移）
    ///
    /// 返回按优先级排序的可用供应商列表：
    /// - 故障转移关闭时：仅返回当前供应商
    /// - 故障转移开启时：完全按照故障转移队列顺序返回，忽略当前供应商设置
    pub fn select_providers<'a>(
        &'a self,
        app_type: &'a str,
        request_model: Option<&'a str>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<Provider>, AppError>> + 'a + Send>>
    {
        Box::pin(async move {
            self.select_providers_impl(app_type, request_model).await
        })
    }

    async fn select_providers_impl(
        &self,
        app_type: &str,
        request_model: Option<&str>,
    ) -> Result<Vec<Provider>, AppError> {
        let request_model = request_model.unwrap_or("unknown");

        // 检查该应用的自动故障转移开关是否开启（从 proxy_config 表读取）
        let auto_failover_enabled = match self.db.get_proxy_config_for_app(app_type).await {
            Ok(config) => {
                let enabled = config.auto_failover_enabled;
                log::debug!("[{app_type}] Failover enabled from proxy_config: {enabled}");
                enabled
            }
            Err(e) => {
                log::error!(
                    "[{app_type}] Failed to read proxy_config for auto_failover_enabled: {e}, defaulting to disabled"
                );
                false
            }
        };

        if auto_failover_enabled {
            // 故障转移开启：按层级生成候选链（由转发器按“层级内轮询重试 -> 进入下一层级”执行）
            // 轮询单位为“不同的 key 值”（相同 key 不重复计权），且每个供应商同一时刻仅使用其“当前最快 URL”。
            let failover_providers = self.db.get_failover_providers(app_type)?;

            log::debug!(
                "[{}] Failover enabled, {} providers in queue",
                app_type,
                failover_providers.len()
            );

            // 按层级分组（sort_index 作为层级）
            let mut priority_groups: std::collections::BTreeMap<usize, Vec<Provider>> =
                std::collections::BTreeMap::new();
            for provider in failover_providers {
                let priority = provider.sort_index.unwrap_or(999999);
                priority_groups
                    .entry(priority)
                    .or_insert_with(Vec::new)
                    .push(provider);
            }

            let mut first_priority: Option<usize> = None;
            let mut selected_chain: Vec<Provider> = Vec::new();

            let test_override = self.get_active_test_override(app_type).await;

            for (priority, providers_in_level) in priority_groups.iter() {
                if let Some(o) = test_override.as_ref() {
                    if *priority != o.priority {
                        continue;
                    }
                }
                // 在当前层级内按供应商 -> URL -> providers 分组
                let mut supplier_urls: HashMap<String, HashMap<String, Vec<Provider>>> =
                    HashMap::new();

                for provider in providers_in_level {
                    let supplier = Self::supplier_name(provider);
                    if let Some(o) = test_override.as_ref() {
                        if supplier != o.supplier {
                            continue;
                        }
                    }
                    let Some(base_url) = Self::extract_base_url(provider, app_type) else {
                        continue;
                    };
                    supplier_urls
                        .entry(supplier)
                        .or_insert_with(HashMap::new)
                        .entry(base_url)
                        .or_insert_with(Vec::new)
                        .push(provider.clone());
                }

                if supplier_urls.is_empty() {
                    continue;
                }

                let mut candidates: Vec<Provider> = Vec::new();

                for (supplier, url_map) in supplier_urls.iter() {
                    if test_override.is_none()
                        && self
                            .is_supplier_in_cooldown(app_type, *priority, supplier)
                            .await
                    {
                        continue;
                    }

                    // 正常请求不应反复测速：
                    // - 启动时为每个 supplier 选一次最快 URL；
                    // - 仅当该 URL 被标记 suspect（链路失效）时，才清空并重新测速/切换。
                    let mut selected_url: Option<String> = None;

                    if url_map.len() == 1 {
                        if let Some(url) = url_map.keys().next() {
                            if !self.is_url_suspect(app_type, supplier, url).await {
                                selected_url = Some(url.clone());
                                self.set_supplier_current_url(app_type, *priority, supplier, url)
                                    .await;
                            }
                        }
                    } else if let Some(current_url) =
                        self.get_supplier_current_url(app_type, *priority, supplier).await
                    {
                        if url_map.contains_key(&current_url)
                            && !self.is_url_suspect(app_type, supplier, &current_url).await
                        {
                            selected_url = Some(current_url);
                        } else {
                            self.clear_supplier_current_url(app_type, *priority, supplier).await;
                        }
                    }

                    if selected_url.is_none() {
                        // 使用锁避免并发请求导致重复测速
                        let lock = self
                            .get_supplier_benchmark_lock(app_type, *priority, supplier)
                            .await;
                        let _guard = lock.lock().await;

                        // 二次检查：可能在等待锁期间已有其它任务选出了 current_url
                        if let Some(current_url) =
                            self.get_supplier_current_url(app_type, *priority, supplier).await
                        {
                            if url_map.contains_key(&current_url)
                                && !self.is_url_suspect(app_type, supplier, &current_url).await
                            {
                                selected_url = Some(current_url);
                            } else {
                                self.clear_supplier_current_url(app_type, *priority, supplier)
                                    .await;
                            }
                        }

                        if selected_url.is_none() {
                            // URL 优先级：当指定 URL 可用时优先使用（例如 anyrouter.top）
                            // 优先级来源：默认规则 + provider.settingsConfig/baseUrlPriority + env.BASE_URL_PRIORITY
                            let mut preferred: Vec<String> = Self::default_url_priority_for_supplier(supplier)
                                .into_iter()
                                .map(|s| s.to_string())
                                .collect();
                            if let Some(p) = url_map.values().flat_map(|v| v.first()).next() {
                                preferred.extend(Self::parse_url_priority_from_provider(p));
                            }
                            // 去重（保留顺序）
                            {
                                let mut seen = std::collections::HashMap::<String, ()>::new();
                                preferred.retain(|u| seen.insert(u.to_string(), ()).is_none());
                            }

                            for purl in preferred.iter() {
                                if !url_map.contains_key(purl) {
                                    continue;
                                }
                                if self.is_url_suspect(app_type, supplier, purl).await {
                                    continue;
                                }

                                // “有效”判断（更保守）：
                                // - 仅当已有“全链路 OK”缓存时才直接命中优先级；
                                // - 仅连通性 OK（FB/penalty）不应强行锁定优先级 URL，否则会长期卡在网关可连通但业务不可用的 URL 上。
                                let cache_key = Self::url_latency_key(app_type, *priority, supplier, purl);
                                let cached_latency = {
                                    let latencies = self.url_latencies.read().await;
                                    latencies.get(&cache_key).map(|l| l.latency_ms)
                                };

                                if let Some(l) = cached_latency {
                                    // 仅当“明显不是回退结果（penalty）”时，才认为可直接命中优先 URL
                                    if l != u64::MAX && l < Self::CONNECTIVITY_PENALTY_MS {
                                        selected_url = Some(purl.clone());
                                        self.set_supplier_current_url(app_type, *priority, supplier, purl)
                                            .await;
                                        log::info!(
                                            "[{}:{}] URL优先级命中 supplier={} 选用={} (cached_latency_ms={:?})",
                                            app_type,
                                            priority,
                                            supplier,
                                            purl,
                                            cached_latency
                                        );
                                        break;
                                    }
                                } else if let Ok(connect_ms) = self.connectivity_latency(purl).await {
                                    // 仅用于缓存（避免重复探测刷屏），不作为“优先级直接命中”的依据
                                    let latency =
                                        connect_ms.saturating_add(Self::CONNECTIVITY_PENALTY_MS);
                                    let mut latencies = self.url_latencies.write().await;
                                    latencies.insert(
                                        cache_key,
                                        UrlLatency {
                                            latency_ms: latency,
                                            tested_at: std::time::Instant::now(),
                                        },
                                    );
                                }
                            }

                            if selected_url.is_some() {
                                // 已按优先级选出 URL，跳过后续测速/排序逻辑
                            } else {
                            // 生成该供应商的 URL 有序列表（优先使用缓存；缓存缺失/URL失效时才测速）
                            let tested_key = Self::supplier_key(app_type, *priority, supplier);
                            let mut should_benchmark = false;
                            {
                                let tested_map = self.priority_level_tested.read().await;
                                if tested_map.get(&tested_key).copied().unwrap_or(false) == false {
                                    should_benchmark = true;
                                }
                            }

                            let mut urls_with_latency: Vec<(String, u64)> = Vec::new();
                            if !should_benchmark {
                                let latencies = self.url_latencies.read().await;
                                urls_with_latency = url_map
                                    .keys()
                                    .map(|url| {
                                        let cache_key =
                                            Self::url_latency_key(app_type, *priority, supplier, url);
                                        let latency = latencies
                                            .get(&cache_key)
                                            .map(|l| l.latency_ms)
                                            .unwrap_or(u64::MAX);
                                        (url.clone(), latency)
                                    })
                                    .collect();
                                urls_with_latency.sort_by_key(|(_, latency)| *latency);

                                // 缓存完全缺失：需要测速一次选出最快 URL
                                let has_any_latency =
                                    urls_with_latency.iter().any(|(_, l)| *l != u64::MAX);
                                if !has_any_latency {
                                    should_benchmark = true;
                                }
                            }

                            // 过滤掉 suspect URL
                            let mut filtered_urls = Vec::new();
                            for (url, latency) in urls_with_latency.iter() {
                                if *latency == u64::MAX {
                                    continue;
                                }
                                if self.is_url_suspect(app_type, supplier, url).await {
                                    continue;
                                }
                                filtered_urls.push(url.clone());
                            }

                            if should_benchmark || filtered_urls.is_empty() {
                                let benchmark_results = self
                                    .benchmark_urls(
                                        app_type,
                                        *priority,
                                        request_model,
                                        supplier,
                                        url_map,
                                    )
                                    .await;

                                {
                                    let mut tested_map = self.priority_level_tested.write().await;
                                    tested_map.insert(tested_key.clone(), true);
                                }

                                let mut ok = Vec::new();
                                for (url, latency) in benchmark_results {
                                    if latency == u64::MAX {
                                        continue;
                                    }
                                    if self.is_url_suspect(app_type, supplier, &url).await {
                                        continue;
                                    }
                                    ok.push(url);
                                }
                                filtered_urls = ok;
                            }

                            // 若存在 URL 优先级配置，则优先挑选“全链路 OK”的优先 URL；
                            // 若不存在“全链路 OK”，仍按原有策略仅做顺序调整（FB 结果不会强制锁定优先 URL）。
                            if filtered_urls.len() > 1 {
                                let mut preferred: Vec<String> = Self::default_url_priority_for_supplier(supplier)
                                    .into_iter()
                                    .map(|s| s.to_string())
                                    .collect();
                                if let Some(p) = url_map.values().flat_map(|v| v.first()).next() {
                                    preferred.extend(Self::parse_url_priority_from_provider(p));
                                }
                                let mut seen = std::collections::HashMap::<String, ()>::new();
                                preferred.retain(|u| seen.insert(u.to_string(), ()).is_none());

                                // 先尝试命中“优先 URL 且全链路 OK”
                                for purl in preferred.iter() {
                                    if !filtered_urls.iter().any(|u| u == purl) {
                                        continue;
                                    }
                                    if self.is_url_suspect(app_type, supplier, purl).await {
                                        continue;
                                    }
                                    let cache_key =
                                        Self::url_latency_key(app_type, *priority, supplier, purl);
                                    let cached_latency = {
                                        let latencies = self.url_latencies.read().await;
                                        latencies.get(&cache_key).map(|l| l.latency_ms)
                                    };
                                    if let Some(l) = cached_latency {
                                        if l != u64::MAX && l < Self::CONNECTIVITY_PENALTY_MS {
                                            selected_url = Some(purl.clone());
                                            self.set_supplier_current_url(
                                                app_type,
                                                *priority,
                                                supplier,
                                                purl,
                                            )
                                            .await;
                                            break;
                                        }
                                    }
                                }

                                // 未命中全链路 OK 的优先 URL，则仅按优先级调整顺序
                                if selected_url.is_none() {
                                    filtered_urls = Self::apply_url_priority(filtered_urls, &preferred);
                                }
                            }

                            if selected_url.is_none() {
                                if let Some(url) = filtered_urls.first() {
                                    selected_url = Some(url.clone());
                                    self.set_supplier_current_url(app_type, *priority, supplier, url).await;
                                }
                            }
                            }
                        }
                    }

                    let Some(selected_url) = selected_url else {
                        // 该供应商当前无可用 URL：进入短暂冷静期
                        self.set_supplier_cooldown(app_type, *priority, supplier, 20).await;
                        continue;
                    };

                    let Some(providers_at_url) = url_map.get(&selected_url) else {
                        continue;
                    };

                    // 在该 URL 上按“不同 key 值”去重，保证轮询均分
                    let mut unique_by_key: HashMap<String, Provider> = HashMap::new();
                    for provider in providers_at_url {
                        let Some(key_value) = Self::extract_api_key_value(provider, app_type) else {
                            continue;
                        };
                        unique_by_key.entry(key_value).or_insert_with(|| provider.clone());
                    }

                    // 熔断器过滤：只保留当前可用的 key
                    for provider in unique_by_key.values() {
                        let circuit_key = format!("{}:{}", app_type, provider.id);
                        let breaker = self.get_or_create_circuit_breaker(&circuit_key).await;
                        if breaker.is_available().await {
                            candidates.push(provider.clone());
                        }
                    }
                }

                if candidates.is_empty() {
                    continue;
                }

                // 层级命中：应用“key均分”轮询（在所有 key 上 round-robin）
                candidates.sort_by(|a, b| a.id.cmp(&b.id));

                let counter_key = format!("{app_type}:priority:{priority}:key-rr");
                let rotate_count = {
                    let mut counters = self.round_robin_counters.write().await;
                    let counter = counters.entry(counter_key.clone()).or_insert(0);
                    let count = *counter % candidates.len();
                    *counter = (*counter + 1) % candidates.len();
                    count
                };
                candidates.rotate_left(rotate_count);

                if first_priority.is_none() {
                    first_priority = Some(*priority);
                }
                // 追加该层级的候选 key；后续层级继续追加，由 forwarder 在失败后推进到下一层级
                selected_chain.extend(candidates);
            }

            let Some(target_priority) = first_priority else {
                return Err(AppError::Config(format!(
                    "No available providers for {app_type} (all priorities unavailable)"
                )));
            };

            // 记录当前激活层级
            {
                let mut active_levels = self.active_priority_level.write().await;
                active_levels.insert(app_type.to_string(), target_priority);
            }

            log::debug!(
                "[{}] Selected priority {} with {} key(s) across priorities (model={})",
                app_type,
                target_priority,
                selected_chain.len(),
                request_model
            );

            return Ok(selected_chain);
        } else {
            // 故障转移关闭：仅使用当前供应商，跳过熔断器检查
            // 原因：单 Provider 场景下，熔断器打开会导致所有请求失败，用户体验差
            log::info!("[{app_type}] Failover disabled, using current provider only (circuit breaker bypassed)");

            if let Some(current_id) = self.db.get_current_provider(app_type)? {
                if let Some(current) = self.db.get_provider_by_id(&current_id, app_type)? {
                    log::debug!(
                        "[{}] Current provider: {} ({})",
                        app_type,
                        current.name,
                        current.id
                    );
                    return Ok(vec![current]);
                }
            }
        }

        Err(AppError::Config(format!(
            "No available provider for {app_type} (failover disabled but current provider missing)"
        )))
    }

    /// 请求执行前获取熔断器“放行许可”
    ///
    /// - Closed：直接放行
    /// - Open：超时到达后切到 HalfOpen 并放行一次探测
    /// - HalfOpen：按限流规则放行探测
    ///
    /// 注意：调用方必须在请求结束后通过 `record_result()` 释放 HalfOpen 名额，
    /// 否则会导致该 Provider 长时间无法进入探测状态。
    pub async fn allow_provider_request(&self, provider_id: &str, app_type: &str) -> AllowResult {
        let circuit_key = format!("{app_type}:{provider_id}");
        let breaker = self.get_or_create_circuit_breaker(&circuit_key).await;
        breaker.allow_request().await
    }

    /// 记录供应商请求结果
    pub async fn record_result(
        &self,
        provider_id: &str,
        app_type: &str,
        used_half_open_permit: bool,
        success: bool,
        error_msg: Option<String>,
    ) -> Result<(), AppError> {
        // 1. 按应用独立获取熔断器配置（用于更新健康状态和判断是否禁用）
        let failure_threshold = match self.db.get_proxy_config_for_app(app_type).await {
            Ok(app_config) => app_config.circuit_failure_threshold,
            Err(e) => {
                log::warn!(
                    "Failed to load circuit config for {app_type}, using default threshold: {e}"
                );
                5 // 默认值
            }
        };

        // 2. 更新熔断器状态
        let circuit_key = format!("{app_type}:{provider_id}");
        let breaker = self.get_or_create_circuit_breaker(&circuit_key).await;

        if success {
            breaker.record_success(used_half_open_permit).await;
            log::debug!("Provider {provider_id} request succeeded");
        } else {
            breaker.record_failure(used_half_open_permit).await;
            log::debug!(
                "Provider {} request failed: {}",
                provider_id,
                error_msg.as_deref().unwrap_or("Unknown error")
            );
        }

        // 2.5 失败时：只有在“明显链路错误”时才标记 URL suspect（避免因上游满载/策略/5xx 误判导致反复测速刷屏）
        // 促使下次选择时在同供应商内切换到其它 URL 并重新测速。
        if !success {
            if let Some(err) = error_msg.as_deref() {
                if Self::is_likely_network_error(err) {
                    let seconds = 60;
                    if let Some(provider) = self.db.get_provider_by_id(provider_id, app_type)? {
                        let supplier = Self::supplier_name(&provider);
                        if let Some(url) = Self::extract_base_url(&provider, app_type) {
                            self.set_url_suspect(app_type, &supplier, &url, seconds).await;
                            let priority = provider.sort_index.unwrap_or(999999);
                            self.clear_supplier_current_url(app_type, priority, &supplier)
                                .await;
                        }
                    }
                }
            }
        } else {
            // 成功时尝试移除 suspect（如果有的话）
            if let Some(provider) = self.db.get_provider_by_id(provider_id, app_type)? {
                let supplier = Self::supplier_name(&provider);
                if let Some(url) = Self::extract_base_url(&provider, app_type) {
                    let key = format!("{app_type}:{supplier}:{url}");
                    let mut map = self.suspect_urls.write().await;
                    map.remove(&key);
                }
            }
        }

        // 3. 更新数据库健康状态（使用配置的阈值）
        self.db
            .update_provider_health_with_threshold(
                provider_id,
                app_type,
                success,
                error_msg.clone(),
                failure_threshold,
            )
            .await?;

        Ok(())
    }

    /// 重置熔断器（手动恢复）
    pub async fn reset_circuit_breaker(&self, circuit_key: &str) {
        let breakers = self.circuit_breakers.read().await;
        if let Some(breaker) = breakers.get(circuit_key) {
            log::info!("Manually resetting circuit breaker for {circuit_key}");
            breaker.reset().await;
        }
    }

    /// 重置指定供应商的熔断器
    pub async fn reset_provider_breaker(&self, provider_id: &str, app_type: &str) {
        let circuit_key = format!("{app_type}:{provider_id}");
        self.reset_circuit_breaker(&circuit_key).await;
    }

    /// 更新所有熔断器的配置（热更新）
    ///
    /// 当用户在 UI 中修改熔断器配置后调用此方法，
    /// 所有现有的熔断器会立即使用新配置
    pub async fn update_all_configs(&self, config: CircuitBreakerConfig) {
        let breakers = self.circuit_breakers.read().await;
        let count = breakers.len();

        for breaker in breakers.values() {
            breaker.update_config(config.clone()).await;
        }

        log::info!("已更新 {count} 个熔断器的配置");
    }

    /// 获取熔断器状态
    #[allow(dead_code)]
    pub async fn get_circuit_breaker_stats(
        &self,
        provider_id: &str,
        app_type: &str,
    ) -> Option<crate::proxy::circuit_breaker::CircuitBreakerStats> {
        let circuit_key = format!("{app_type}:{provider_id}");
        let breakers = self.circuit_breakers.read().await;

        if let Some(breaker) = breakers.get(&circuit_key) {
            Some(breaker.get_stats().await)
        } else {
            None
        }
    }

    /// 获取或创建熔断器
    async fn get_or_create_circuit_breaker(&self, key: &str) -> Arc<CircuitBreaker> {
        // 先尝试读锁获取
        {
            let breakers = self.circuit_breakers.read().await;
            if let Some(breaker) = breakers.get(key) {
                return breaker.clone();
            }
        }

        // 如果不存在，获取写锁创建
        let mut breakers = self.circuit_breakers.write().await;

        // 双重检查，防止竞争条件
        if let Some(breaker) = breakers.get(key) {
            return breaker.clone();
        }

        // 从 key 中提取 app_type (格式: "app_type:provider_id")
        let app_type = key.split(':').next().unwrap_or("claude");

        // 按应用独立读取熔断器配置
        let config = match self.db.get_proxy_config_for_app(app_type).await {
            Ok(app_config) => {
                log::debug!(
                    "Loading circuit breaker config for {key} (app={app_type}): \
                    failure_threshold={}, success_threshold={}, timeout={}s",
                    app_config.circuit_failure_threshold,
                    app_config.circuit_success_threshold,
                    app_config.circuit_timeout_seconds
                );
                crate::proxy::circuit_breaker::CircuitBreakerConfig {
                    failure_threshold: app_config.circuit_failure_threshold,
                    success_threshold: app_config.circuit_success_threshold,
                    timeout_seconds: app_config.circuit_timeout_seconds as u64,
                    error_rate_threshold: app_config.circuit_error_rate_threshold,
                    min_requests: app_config.circuit_min_requests,
                }
            }
            Err(e) => {
                log::warn!(
                    "Failed to load circuit breaker config for {key} (app={app_type}): {e}, using default"
                );
                crate::proxy::circuit_breaker::CircuitBreakerConfig::default()
            }
        };

        log::debug!("Creating new circuit breaker for {key} with config: {config:?}");

        let breaker = Arc::new(CircuitBreaker::new(config));
        breakers.insert(key.to_string(), breaker.clone());

        breaker
    }

    /// 测试URL的全链路延迟
    ///
    /// 发送简单问答请求，测量完整延迟
    /// - Claude: Rust -> Python -> 目标URL -> Python -> Rust
    /// - Codex: Rust -> 目标URL -> Rust
    async fn test_url_latency(
        &self,
        provider: &Provider,
        app_type: &str,
        request_model: &str,
    ) -> Result<u64, UrlProbeError> {
        let config_err = |message: String| UrlProbeError {
            latency_ms: 0,
            kind: UrlProbeErrorKind::Network { message },
        };

        // 根据app_type提取base_url
        let base_url = match app_type {
            "claude" => provider
                .settings_config
                .get("env")
                .and_then(|env| env.get("ANTHROPIC_BASE_URL"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| config_err("Provider缺少ANTHROPIC_BASE_URL配置".to_string()))?,
            "gemini" => provider
                .settings_config
                .get("env")
                .and_then(|env| env.get("GOOGLE_GEMINI_BASE_URL"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| config_err("Provider缺少GOOGLE_GEMINI_BASE_URL配置".to_string()))?,
            "codex" => {
                // Codex的base_url直接在settingsConfig根级别
                provider
                    .settings_config
                    .get("base_url")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| config_err("Provider缺少base_url配置".to_string()))?
            }
            _ => {
                return Err(config_err(format!("不支持的app_type: {}", app_type)));
            }
        };

        // 根据app_type提取API key
        let api_key = match app_type {
            "claude" => provider
                .settings_config
                .get("env")
                .and_then(|env| {
                    env.get("ANTHROPIC_API_KEY")
                        .or_else(|| env.get("ANTHROPIC_AUTH_TOKEN"))
                })
                .and_then(|v| v.as_str())
                .ok_or_else(|| config_err("Provider缺少API key配置".to_string()))?,
            "gemini" => provider
                .settings_config
                .get("env")
                .and_then(|env| env.get("GOOGLE_API_KEY"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| config_err("Provider缺少GOOGLE_API_KEY配置".to_string()))?,
            "codex" => provider
                .settings_config
                .get("env")
                .and_then(|env| env.get("OPENAI_API_KEY"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| config_err("Provider缺少OPENAI_API_KEY配置".to_string()))?,
            _ => {
                return Err(config_err(format!("不支持的app_type: {}", app_type)));
            }
        };

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| UrlProbeError {
                latency_ms: 0,
                kind: UrlProbeErrorKind::Network {
                    message: format!("创建HTTP客户端失败: {e}"),
                },
            })?;

        let start = std::time::Instant::now();

        let response = if app_type == "codex" {
            // Codex: 直接测试目标URL，使用OpenAI格式
            let test_payload = serde_json::json!({
                "model": request_model,
                "max_tokens": 100,
                "temperature": 0.7,
                "stream": false,
                "messages": [{
                    "role": "user",
                    "content": "请简短回答：什么是人工智能？"
                }]
            });

            let target_url = format!("{}/v1/chat/completions", base_url);

            client
                .post(&target_url)
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", api_key))
                .json(&test_payload)
                .send()
                .await
                .map_err(|e| UrlProbeError {
                    latency_ms: start.elapsed().as_millis() as u64,
                    kind: UrlProbeErrorKind::Network {
                        message: format!("请求失败: {e}"),
                    },
                })?
        } else if app_type == "claude" {
            // Claude: 通过Python代理测试，使用Claude格式
            // 关键：测试请求必须尽量贴近真实 CLI 环境，否则会出现“测速不可用但真实可用”的误判。
            let test_payload = serde_json::json!({
                "model": request_model,
                "max_tokens": 100,
                "temperature": 1.0,
                "stream": false,
                "messages": [{
                    "role": "user",
                    "content": "请用一句话简短介绍你自己。"
                }]
            });

            client
                .post("http://127.0.0.1:15722/v1/messages")
                .header("Content-Type", "application/json")
                .header("Accept", "application/json")
                .header("User-Agent", "claude-cli/2.0.8 (external, cli)")
                .header("x-request-id", format!("cc-switch-probe-{}", uuid::Uuid::new_v4()))
                .header("x-stainless-os", std::env::consts::OS)
                .header("x-stainless-arch", std::env::consts::ARCH)
                .header("x-stainless-lang", "rust")
                .header("x-stainless-runtime", "cc-switch")
                .header("x-stainless-runtime-version", env!("CARGO_PKG_VERSION"))
                .header("x-stainless-package-version", env!("CARGO_PKG_VERSION"))
                .header("X-API-Key", api_key)
                .header("x-target-base-url", base_url)
                .json(&test_payload)
                .send()
                .await
                .map_err(|e| UrlProbeError {
                    latency_ms: start.elapsed().as_millis() as u64,
                    kind: UrlProbeErrorKind::Network {
                        message: format!("请求失败: {e}"),
                    },
                })?
        } else {
            // Gemini（或其他）：暂无稳定的“全链路问答”探测格式，这里仅进行基础连通性探测。
            client
                .get(base_url)
                .send()
                .await
                .map_err(|e| UrlProbeError {
                    latency_ms: start.elapsed().as_millis() as u64,
                    kind: UrlProbeErrorKind::Network {
                        message: format!("请求失败: {e}"),
                    },
                })?
        };

        let status = response.status();
        let latency = start.elapsed().as_millis() as u64;

        // 简化测试：仅检查HTTP状态码和连通性
        if status.is_success() {
            // HTTP 200-299，连接成功
            log::debug!("探测正常 {} - {}", status.as_u16(), provider.name);
            Ok(latency)
        } else {
            // 非200状态码，记录详细错误
            log::debug!("探测失败 {} - {} - 详情: {}", status.as_u16(), provider.name, status);
            let status_code = status.as_u16();
            let body_text = response.text().await.ok();
            let msg = body_text
                .as_deref()
                .and_then(Self::extract_error_message_from_body)
                .unwrap_or_default();

            if !msg.is_empty() && Self::is_overloaded_error_text(&msg) {
                Err(UrlProbeError {
                    latency_ms: latency,
                    kind: UrlProbeErrorKind::Overloaded { message: msg },
                })
            } else {
                Err(UrlProbeError {
                    latency_ms: latency,
                    kind: UrlProbeErrorKind::Http {
                        status: status_code,
                        body: body_text.map(|t| Self::shorten_for_log(&t, 200)),
                    },
                })
            }
        }
    }

    async fn connectivity_latency(&self, base_url: &str) -> Result<u64, String> {
        let url = format!("{}/", base_url.trim_end_matches('/'));
        let client = reqwest::Client::builder()
            .timeout(Self::CONNECTIVITY_TIMEOUT)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| format!("创建HTTP客户端失败: {e}"))?;

        let start = std::time::Instant::now();
        let resp = client
            .head(&url)
            .send()
            .await
            .map_err(|e| format!("连通性探测失败: {e}"))?;

        // 只要能拿到响应，就认为“可连通”；不要求 2xx
        let _ = resp.status();
        Ok(start.elapsed().as_millis() as u64)
    }

    /// 测试所有URL并返回延迟排序结果
    ///
    /// 返回: Vec<(url, latency_ms)> 按延迟从低到高排序
    pub async fn benchmark_urls(
        &self,
        app_type: &str,
        priority: usize,
        request_model: &str,
        supplier: &str,
        url_groups: &HashMap<String, Vec<Provider>>,
    ) -> Vec<(String, u64)> {
        let details = self
            .benchmark_urls_detailed(app_type, priority, request_model, supplier, url_groups)
            .await;

        let mut results: Vec<(String, u64)> = Vec::with_capacity(details.len());
        for d in details.iter() {
            let latency = match &d.kind {
                UrlProbeKind::FullOk { latency_ms } => *latency_ms,
                UrlProbeKind::Overloaded { latency_ms, .. } => {
                    latency_ms.saturating_add(Self::CONNECTIVITY_PENALTY_MS)
                }
                UrlProbeKind::FallbackOk {
                    connect_ms,
                    penalty_ms,
                    ..
                } => connect_ms.saturating_add(*penalty_ms),
                UrlProbeKind::Failed { .. } => u64::MAX,
            };
            results.push((d.url.clone(), latency));
        }

        results
    }

    /// 详细测速：与真实启动探测同构，但保留“满载/限流”等可达状态，
    /// 用于 CLI `csc t` 输出与诊断。
    pub async fn benchmark_urls_detailed(
        &self,
        app_type: &str,
        priority: usize,
        request_model: &str,
        supplier: &str,
        url_groups: &HashMap<String, Vec<Provider>>,
    ) -> Vec<UrlProbeDetail> {
        log::debug!(
            "[{}:{}] 开始URL延迟测试，共{}个URL (supplier={}, model={})",
            app_type,
            priority,
            url_groups.len(),
            supplier,
            request_model
        );

        let mut details: Vec<UrlProbeDetail> = Vec::new();

        let mut full_ok_count: usize = 0;
        let mut overloaded_count: usize = 0;
        let mut fallback_ok_count: usize = 0;
        let mut fail_count: usize = 0;

        for (url, providers) in url_groups {
            // 同一 URL 下按 key 去重并尝试少量 key，避免“只测第一个 key 就判死”
            const MAX_KEYS_PER_URL: usize = 2;

            let mut unique_by_key: HashMap<String, Provider> = HashMap::new();
            for p in providers {
                let Some(key_value) = Self::extract_api_key_value(p, app_type) else {
                    continue;
                };
                unique_by_key.entry(key_value).or_insert_with(|| p.clone());
            }

            let mut tested_providers: Vec<Provider> = unique_by_key.into_values().collect();
            tested_providers.truncate(MAX_KEYS_PER_URL);

            let mut full_ok: Option<u64> = None;
            let mut overloaded: Option<(u64, String)> = None;
            let mut err_summaries: Vec<String> = Vec::new();

            for provider in tested_providers.iter() {
                log::debug!(
                    "[{}:{}] 测试URL: {} (使用provider: {})",
                    app_type,
                    priority,
                    url,
                    provider.name
                );

                match self.test_url_latency(provider, app_type, request_model).await {
                    Ok(latency) => {
                        full_ok = Some(latency);
                        break;
                    }
                    Err(e) => match e.kind {
                        UrlProbeErrorKind::Overloaded { message } => {
                            overloaded = Some((e.latency_ms, message));
                            // Overloaded 可能与 key 相关，继续尝试下一个 key
                            continue;
                        }
                        UrlProbeErrorKind::Http { status, body } => {
                            let b = body.unwrap_or_default();
                            let reason = if b.is_empty() {
                                format!("HTTP {status}")
                            } else {
                                format!("HTTP {status}: {b}")
                            };
                            err_summaries.push(Self::shorten_for_log(&reason, 120));
                        }
                        UrlProbeErrorKind::Network { message } => {
                            err_summaries.push(Self::shorten_for_log(&message, 120));
                        }
                    },
                }
            }

            if let Some(latency) = full_ok {
                full_ok_count += 1;

                // 缓存全链路延迟（用于后续选择最快 URL）
                let cache_key = Self::url_latency_key(app_type, priority, supplier, url);
                let mut latencies = self.url_latencies.write().await;
                latencies.insert(
                    cache_key,
                    UrlLatency {
                        latency_ms: latency,
                        tested_at: std::time::Instant::now(),
                    },
                );

                details.push(UrlProbeDetail {
                    url: url.clone(),
                    kind: UrlProbeKind::FullOk { latency_ms: latency },
                });
                continue;
            }

            if let Some((latency_ms, message)) = overloaded.clone() {
                overloaded_count += 1;
                details.push(UrlProbeDetail {
                    url: url.clone(),
                    kind: UrlProbeKind::Overloaded {
                        latency_ms,
                        message: Self::shorten_for_log(&message, 120),
                    },
                });
                continue;
            }

            let err_short = if err_summaries.is_empty() {
                "未知错误".to_string()
            } else {
                err_summaries.join("; ")
            };

            // 回退到简单连通性测试（仅作为“可达性”保底）
            match self.connectivity_latency(url).await {
                Ok(connect_ms) => {
                    fallback_ok_count += 1;

                    let penalty_ms = Self::CONNECTIVITY_PENALTY_MS;
                    let total_ms = connect_ms.saturating_add(penalty_ms);

                    // 缓存回退结果（避免重复测速刷屏）
                    let cache_key = Self::url_latency_key(app_type, priority, supplier, url);
                    let mut latencies = self.url_latencies.write().await;
                    latencies.insert(
                        cache_key,
                        UrlLatency {
                            latency_ms: total_ms,
                            tested_at: std::time::Instant::now(),
                        },
                    );

                    details.push(UrlProbeDetail {
                        url: url.clone(),
                        kind: UrlProbeKind::FallbackOk {
                            connect_ms,
                            penalty_ms,
                            reason: err_short,
                        },
                    });
                }
                Err(connect_err) => {
                    fail_count += 1;
                    details.push(UrlProbeDetail {
                        url: url.clone(),
                        kind: UrlProbeKind::Failed {
                            reason: format!(
                                "全链路失败={}; 连通性失败={}",
                                err_short,
                                Self::shorten_for_log(&connect_err, 120)
                            ),
                        },
                    });
                }
            }
        }

        // 排序：OK 最优，其次 OVERLOADED，再次 FB，最后 FAIL
        details.sort_by_key(|d| match &d.kind {
            UrlProbeKind::FullOk { latency_ms } => (0u8, *latency_ms),
            UrlProbeKind::Overloaded { latency_ms, .. } => (1u8, latency_ms.saturating_add(30_000)),
            UrlProbeKind::FallbackOk { connect_ms, penalty_ms, .. } => (2u8, connect_ms.saturating_add(*penalty_ms)),
            UrlProbeKind::Failed { .. } => (3u8, u64::MAX),
        });

        // 选用策略：对齐真实路由（优先 URL 且全链路 OK > OK 最快 > OV > FB）
        let mut preferred: Vec<String> = Self::default_url_priority_for_supplier(supplier)
            .into_iter()
            .map(|s| s.to_string())
            .collect();
        if let Some(p) = url_groups.values().flat_map(|v| v.first()).next() {
            preferred.extend(Self::parse_url_priority_from_provider(p));
        }
        {
            let mut seen = std::collections::HashMap::<String, ()>::new();
            preferred.retain(|u| seen.insert(u.to_string(), ()).is_none());
        }

        let preferred_ok = preferred.iter().find_map(|u| {
            details
                .iter()
                .find(|d| d.url == *u && matches!(d.kind, UrlProbeKind::FullOk { .. }))
        });

        let selected = preferred_ok
            .or_else(|| {
                details
                    .iter()
                    .find(|d| matches!(d.kind, UrlProbeKind::FullOk { .. }))
            })
            .or_else(|| {
                details
                    .iter()
                    .find(|d| matches!(d.kind, UrlProbeKind::Overloaded { .. }))
            })
            .or_else(|| {
                details
                    .iter()
                    .find(|d| matches!(d.kind, UrlProbeKind::FallbackOk { .. }))
            });
        let selected_text = selected
            .map(|d| match &d.kind {
                UrlProbeKind::FullOk { latency_ms } => format!("{} (OK {}ms)", d.url, latency_ms),
                UrlProbeKind::Overloaded { latency_ms, .. } => {
                    format!("{} (OV {}ms)", d.url, latency_ms)
                }
                UrlProbeKind::FallbackOk {
                    connect_ms,
                    penalty_ms,
                    ..
                } => format!(
                    "{} (FB {}ms +{}ms)",
                    d.url, connect_ms, penalty_ms
                ),
                UrlProbeKind::Failed { .. } => "N/A".to_string(),
            })
            .unwrap_or_else(|| "N/A".to_string());

        let detail_text = details
            .iter()
            .map(|d| match &d.kind {
                UrlProbeKind::FullOk { latency_ms } => format!("{}=OK({}ms)", d.url, latency_ms),
                UrlProbeKind::Overloaded { latency_ms, message } => {
                    format!("{}=OV({}ms, {})", d.url, latency_ms, message)
                }
                UrlProbeKind::FallbackOk { connect_ms, penalty_ms, .. } => {
                    format!("{}=FB({}ms+{}ms)", d.url, connect_ms, penalty_ms)
                }
                UrlProbeKind::Failed { .. } => format!("{}=FAIL", d.url),
            })
            .collect::<Vec<_>>()
            .join("; ");

        let summary_info = Self::should_log_benchmark_summary_info();

        if full_ok_count == 0 && overloaded_count == 0 && fallback_ok_count == 0 {
            // 全失败时始终 WARN（便于排障）
            log::warn!(
                "[{}:{}] 测速结束 supplier={} model={} 结果: 全失败(ok=0 ov=0 fb=0 fail={}) 选用={} 详情: {}",
                app_type,
                priority,
                supplier,
                request_model,
                fail_count,
                selected_text,
                detail_text
            );
        } else {
            // 默认不刷屏：摘要降为 DEBUG；需要时可通过 CC_SWITCH_BENCHMARK_SUMMARY=1 提升到 INFO
            if summary_info {
                log::info!(
                    "[{}:{}] 测速结束 supplier={} model={} 结果: ok={} ov={} fb={} fail={} 选用={} 详情: {}",
                    app_type,
                    priority,
                    supplier,
                    request_model,
                    full_ok_count,
                    overloaded_count,
                    fallback_ok_count,
                    fail_count,
                    selected_text,
                    detail_text
                );
            } else {
                log::debug!(
                    "[{}:{}] 测速结束 supplier={} model={} 结果: ok={} ov={} fb={} fail={} 选用={} 详情: {}",
                    app_type,
                    priority,
                    supplier,
                    request_model,
                    full_ok_count,
                    overloaded_count,
                    fallback_ok_count,
                    fail_count,
                    selected_text,
                    detail_text
                );
            }
        }

        // startup 测速模式：若存在测试覆盖并匹配当前 supplier，则记录结果供 CLI 轮询读取
        if let Some(o) = self.get_active_test_override(app_type).await {
            if o.priority == priority && o.supplier == supplier {
                let urls = Self::details_to_benchmark_url_results(&details);

                let (chosen_url, chosen_kind, metric_ms) = if let Some(p) = selected {
                    match &p.kind {
                        UrlProbeKind::FullOk { latency_ms } => {
                            (Some(p.url.clone()), "OK".to_string(), Some(*latency_ms))
                        }
                        UrlProbeKind::Overloaded { latency_ms, .. } => (
                            Some(p.url.clone()),
                            "OV".to_string(),
                            Some(latency_ms.saturating_add(Self::CONNECTIVITY_PENALTY_MS)),
                        ),
                        UrlProbeKind::FallbackOk {
                            connect_ms,
                            penalty_ms,
                            ..
                        } => (
                            Some(p.url.clone()),
                            "FB".to_string(),
                            Some(connect_ms.saturating_add(*penalty_ms)),
                        ),
                        UrlProbeKind::Failed { .. } => (None, "FAIL".to_string(), None),
                    }
                } else {
                    (None, "FAIL".to_string(), None)
                };

                let result = BenchmarkSupplierResult {
                    priority,
                    supplier: supplier.to_string(),
                    chosen_url: chosen_url.clone(),
                    chosen_kind,
                    metric_ms,
                    urls,
                };

                {
                    let mut map = self.test_results.write().await;
                    map.insert(o.run_id.clone(), result);
                }

                // 防止测试覆盖影响后续正常请求
                *self.test_override.write().await = None;
            }
        }

        details
    }

    pub async fn benchmark_all_suppliers(
        &self,
        app_type: &str,
        request_model: &str,
        only_priority: Option<usize>,
        only_supplier: Option<&str>,
    ) -> Result<Vec<BenchmarkSupplierResult>, AppError> {
        let providers = self.db.get_failover_providers(app_type)?;
        if providers.is_empty() {
            return Ok(Vec::new());
        }

        let mut priority_groups: std::collections::BTreeMap<usize, Vec<Provider>> =
            std::collections::BTreeMap::new();
        for provider in providers {
            let priority = provider.sort_index.unwrap_or(999999);
            if let Some(p) = only_priority {
                if priority != p {
                    continue;
                }
            }
            priority_groups.entry(priority).or_default().push(provider);
        }

        let mut out: Vec<BenchmarkSupplierResult> = Vec::new();

        for (priority, providers_in_level) in priority_groups.into_iter() {
            let mut supplier_urls: HashMap<String, HashMap<String, Vec<Provider>>> = HashMap::new();

            for provider in providers_in_level {
                let supplier = Self::supplier_name(&provider);
                if let Some(s) = only_supplier {
                    if supplier != s {
                        continue;
                    }
                }
                let Some(base_url) = Self::extract_base_url(&provider, app_type) else {
                    continue;
                };
                supplier_urls
                    .entry(supplier)
                    .or_default()
                    .entry(base_url)
                    .or_default()
                    .push(provider);
            }

            for (supplier, url_groups) in supplier_urls.into_iter() {
                if self.is_supplier_in_cooldown(app_type, priority, &supplier).await {
                    out.push(BenchmarkSupplierResult {
                        priority,
                        supplier,
                        chosen_url: None,
                        chosen_kind: "COOLDOWN".to_string(),
                        metric_ms: None,
                        urls: Vec::new(),
                    });
                    continue;
                }

                let details = self
                    .benchmark_urls_detailed(app_type, priority, request_model, &supplier, &url_groups)
                    .await;

                let mut urls: Vec<BenchmarkUrlResult> = Vec::with_capacity(details.len());
                for d in details.iter() {
                    let (kind, latency_ms, penalty_ms, message, reason) = match &d.kind {
                        UrlProbeKind::FullOk { latency_ms } => (
                            "OK".to_string(),
                            Some(*latency_ms),
                            None,
                            None,
                            None,
                        ),
                        UrlProbeKind::Overloaded { latency_ms, message } => (
                            "OV".to_string(),
                            Some(*latency_ms),
                            Some(Self::CONNECTIVITY_PENALTY_MS),
                            Some(message.clone()),
                            None,
                        ),
                        UrlProbeKind::FallbackOk {
                            connect_ms,
                            penalty_ms,
                            reason,
                        } => (
                            "FB".to_string(),
                            Some(*connect_ms),
                            Some(*penalty_ms),
                            None,
                            Some(reason.clone()),
                        ),
                        UrlProbeKind::Failed { reason } => (
                            "FAIL".to_string(),
                            None,
                            None,
                            None,
                            Some(reason.clone()),
                        ),
                    };

                    urls.push(BenchmarkUrlResult {
                        url: d.url.clone(),
                        kind,
                        latency_ms,
                        penalty_ms,
                        message,
                        reason,
                    });
                }

                let mut preferred: Vec<String> = Self::default_url_priority_for_supplier(&supplier)
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect();
                if let Some(p) = url_groups.values().flat_map(|v| v.first()).next() {
                    preferred.extend(Self::parse_url_priority_from_provider(p));
                }
                {
                    let mut seen = std::collections::HashMap::<String, ()>::new();
                    preferred.retain(|u| seen.insert(u.to_string(), ()).is_none());
                }

                let preferred_ok = preferred.iter().find_map(|u| {
                    details
                        .iter()
                        .find(|d| d.url == *u && matches!(d.kind, UrlProbeKind::FullOk { .. }))
                });

                let pick = preferred_ok
                    .or_else(|| {
                        details
                            .iter()
                            .find(|d| matches!(d.kind, UrlProbeKind::FullOk { .. }))
                    })
                    .or_else(|| {
                        details
                            .iter()
                            .find(|d| matches!(d.kind, UrlProbeKind::Overloaded { .. }))
                    })
                    .or_else(|| {
                        details
                            .iter()
                            .find(|d| matches!(d.kind, UrlProbeKind::FallbackOk { .. }))
                    });

                let (chosen_url, chosen_kind, metric_ms) = if let Some(p) = pick {
                    match &p.kind {
                        UrlProbeKind::FullOk { latency_ms } => {
                            (Some(p.url.clone()), "OK".to_string(), Some(*latency_ms))
                        }
                        UrlProbeKind::Overloaded { latency_ms, .. } => (
                            Some(p.url.clone()),
                            "OV".to_string(),
                            Some(latency_ms.saturating_add(Self::CONNECTIVITY_PENALTY_MS)),
                        ),
                        UrlProbeKind::FallbackOk {
                            connect_ms,
                            penalty_ms,
                            ..
                        } => (
                            Some(p.url.clone()),
                            "FB".to_string(),
                            Some(connect_ms.saturating_add(*penalty_ms)),
                        ),
                        UrlProbeKind::Failed { .. } => (None, "FAIL".to_string(), None),
                    }
                } else {
                    (None, "FAIL".to_string(), None)
                };

                if let Some(url) = chosen_url.as_deref() {
                    self.set_supplier_current_url(app_type, priority, &supplier, url)
                        .await;
                    let mut tested_map = self.priority_level_tested.write().await;
                    tested_map.insert(Self::supplier_key(app_type, priority, &supplier), true);
                }

                out.push(BenchmarkSupplierResult {
                    priority,
                    supplier,
                    chosen_url,
                    chosen_kind,
                    metric_ms,
                    urls,
                });
            }
        }

        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use serde_json::json;

    #[tokio::test]
    async fn test_provider_router_creation() {
        let db = Arc::new(Database::memory().unwrap());
        let router = ProviderRouter::new(db);

        let breaker = router.get_or_create_circuit_breaker("claude:test").await;
        assert!(breaker.allow_request().await.allowed);
    }

    #[tokio::test]
    async fn test_failover_disabled_uses_current_provider() {
        let db = Arc::new(Database::memory().unwrap());

        let provider_a =
            Provider::with_id("a".to_string(), "Provider A".to_string(), json!({}), None);
        let provider_b =
            Provider::with_id("b".to_string(), "Provider B".to_string(), json!({}), None);

        db.save_provider("claude", &provider_a).unwrap();
        db.save_provider("claude", &provider_b).unwrap();
        db.set_current_provider("claude", "a").unwrap();
        db.add_to_failover_queue("claude", "b").unwrap();

        let router = ProviderRouter::new(db.clone());
        // 单元测试不跑真实网络测速：手动标记该供应商已测试过URL
        {
            let mut tested = router.priority_level_tested.write().await;
            tested.insert("claude:1:anyrouter".to_string(), true);
        }
        let providers = router.select_providers("claude", None).await.unwrap();

        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].id, "a");
    }

    #[tokio::test]
    async fn test_failover_enabled_uses_queue_order() {
        let db = Arc::new(Database::memory().unwrap());

        // 设置 sort_index 来控制顺序：b=1, a=2
        let mut provider_a =
            Provider::with_id(
                "a".to_string(),
                "anyrouter-key-a".to_string(),
                json!({
                    "env": {
                        "ANTHROPIC_API_KEY": "sk-a",
                        "ANTHROPIC_BASE_URL": "https://anyrouter.top"
                    }
                }),
                None,
            );
        provider_a.sort_index = Some(2);
        let mut provider_b =
            Provider::with_id(
                "b".to_string(),
                "anyrouter-key-b".to_string(),
                json!({
                    "env": {
                        "ANTHROPIC_API_KEY": "sk-b",
                        "ANTHROPIC_BASE_URL": "https://anyrouter.top"
                    }
                }),
                None,
            );
        provider_b.sort_index = Some(1);

        db.save_provider("claude", &provider_a).unwrap();
        db.save_provider("claude", &provider_b).unwrap();
        db.set_current_provider("claude", "a").unwrap();

        db.add_to_failover_queue("claude", "b").unwrap();
        db.add_to_failover_queue("claude", "a").unwrap();

        // 启用自动故障转移（使用新的 proxy_config API）
        let mut config = db.get_proxy_config_for_app("claude").await.unwrap();
        config.auto_failover_enabled = true;
        db.update_proxy_config_for_app(config).await.unwrap();

        let router = ProviderRouter::new(db.clone());
        // 单元测试不跑真实网络测速：手动标记该供应商已测试过URL
        {
            let mut tested = router.priority_level_tested.write().await;
            tested.insert("claude:1:anyrouter".to_string(), true);
        }
        let providers = router.select_providers("claude", None).await.unwrap();

        // 返回“多层级候选链”：先给出 priority=1，再追加 priority=2（由 forwarder 在失败后推进到下一层级）
        assert_eq!(providers.len(), 2);
        assert_eq!(providers[0].id, "b");
        assert_eq!(providers[1].id, "a");
    }

    #[tokio::test]
    async fn test_select_providers_does_not_consume_half_open_permit() {
        let db = Arc::new(Database::memory().unwrap());

        db.update_circuit_breaker_config(&CircuitBreakerConfig {
            failure_threshold: 1,
            timeout_seconds: 0,
            ..Default::default()
        })
        .await
        .unwrap();

        let provider_a =
            Provider::with_id(
                "a".to_string(),
                "anyrouter-key-a".to_string(),
                json!({
                    "env": {
                        "ANTHROPIC_API_KEY": "sk-a",
                        "ANTHROPIC_BASE_URL": "https://anyrouter.top"
                    }
                }),
                None,
            );
        let provider_b =
            Provider::with_id(
                "b".to_string(),
                "anyrouter-key-b".to_string(),
                json!({
                    "env": {
                        "ANTHROPIC_API_KEY": "sk-b",
                        "ANTHROPIC_BASE_URL": "https://anyrouter.top"
                    }
                }),
                None,
            );

        db.save_provider("claude", &provider_a).unwrap();
        db.save_provider("claude", &provider_b).unwrap();

        db.add_to_failover_queue("claude", "a").unwrap();
        db.add_to_failover_queue("claude", "b").unwrap();

        // 启用自动故障转移（使用新的 proxy_config API）
        let mut config = db.get_proxy_config_for_app("claude").await.unwrap();
        config.auto_failover_enabled = true;
        db.update_proxy_config_for_app(config).await.unwrap();

        let router = ProviderRouter::new(db.clone());

        router
            .record_result("b", "claude", false, false, Some("fail".to_string()))
            .await
            .unwrap();

        // 单元测试不跑真实网络测速：手动标记该供应商已测试过URL
        {
            let mut tested = router.priority_level_tested.write().await;
            tested.insert("claude:999999:anyrouter".to_string(), true);
        }
        let providers = router.select_providers("claude", None).await.unwrap();
        assert_eq!(providers.len(), 2);

        assert!(router.allow_provider_request("b", "claude").await.allowed);
    }
}
