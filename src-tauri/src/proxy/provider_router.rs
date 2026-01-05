//! 供应商路由器模块
//!
//! 负责选择和管理代理目标供应商，实现智能故障转移

use crate::database::Database;
use crate::error::AppError;
use crate::provider::Provider;
use crate::proxy::circuit_breaker::{AllowResult, CircuitBreaker, CircuitBreakerConfig};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// URL延迟测试结果
#[derive(Debug, Clone)]
struct UrlLatency {
    /// 延迟时间(毫秒)
    latency_ms: u64,
    /// 测试时间戳
    tested_at: std::time::Instant,
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
    /// 层级URL已测试标记 - key 格式: "app_type:priority", value: 是否已测试过URL延迟
    priority_level_tested: Arc<RwLock<HashMap<String, bool>>>,
    /// URL延迟缓存 - key 格式: "app_type:priority:base_url", value: 延迟测试结果
    url_latencies: Arc<RwLock<HashMap<String, UrlLatency>>>,
}

impl ProviderRouter {
    /// 创建新的供应商路由器
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db,
            circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
            round_robin_counters: Arc::new(RwLock::new(HashMap::new())),
            active_priority_level: Arc::new(RwLock::new(HashMap::new())),
            priority_level_tested: Arc::new(RwLock::new(HashMap::new())),
            url_latencies: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 选择可用的供应商（支持故障转移）
    ///
    /// 返回按优先级排序的可用供应商列表：
    /// - 故障转移关闭时：仅返回当前供应商
    /// - 故障转移开启时：完全按照故障转移队列顺序返回，忽略当前供应商设置
    pub fn select_providers<'a>(
        &'a self,
        app_type: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<Provider>, AppError>> + 'a + Send>>
    {
        Box::pin(async move {
            self.select_providers_impl(app_type).await
        })
    }

    async fn select_providers_impl(&self, app_type: &str) -> Result<Vec<Provider>, AppError> {
        let mut result = Vec::new();

        // 检查该应用的自动故障转移开关是否开启（从 proxy_config 表读取）
        let auto_failover_enabled = match self.db.get_proxy_config_for_app(app_type).await {
            Ok(config) => {
                let enabled = config.auto_failover_enabled;
                log::info!("[{app_type}] Failover enabled from proxy_config: {enabled}");
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
            // 故障转移开启：使用 in_failover_queue 标记的供应商，按 sort_index 排序
            let failover_providers = self.db.get_failover_providers(app_type)?;
            log::info!(
                "[{}] Failover enabled, {} providers in queue",
                app_type,
                failover_providers.len()
            );

            // 按优先级分组providers
            let mut priority_groups: std::collections::BTreeMap<usize, Vec<Provider>> = std::collections::BTreeMap::new();
            for provider in failover_providers {
                let priority = provider.sort_index.unwrap_or(999999);
                priority_groups
                    .entry(priority)
                    .or_insert_with(Vec::new)
                    .push(provider);
            }

            log::info!(
                "[{}] Grouped into {} priority levels: {:?}",
                app_type,
                priority_groups.len(),
                priority_groups.keys().collect::<Vec<_>>()
            );

            // 找到第一个有可用providers的层级（用于确定需要测试哪个层级的URL）
            let mut first_available_priority: Option<usize> = None;
            for (priority, providers_in_level) in priority_groups.iter() {
                // 检查该层级是否有任何可用的provider（熔断器未打开）
                let mut has_available = false;
                for provider in providers_in_level {
                    let circuit_key = format!("{}:{}", app_type, provider.id);
                    let breaker = self.get_or_create_circuit_breaker(&circuit_key).await;
                    if breaker.is_available().await {
                        has_available = true;
                        break;
                    }
                }

                if has_available {
                    first_available_priority = Some(*priority);
                    log::info!(
                        "[{}] First available priority level: {} ({} providers)",
                        app_type,
                        priority,
                        providers_in_level.len()
                    );
                    break;
                }
            }

            // 如果没有任何可用的层级，返回错误
            let target_priority = match first_available_priority {
                Some(p) => p,
                None => {
                    log::error!("[{}] 所有优先级层级的providers都不可用（熔断器全部打开）", app_type);
                    return Err(AppError::Config(format!(
                        "No available providers for {app_type} (all circuit breakers open)"
                    )));
                }
            };

            // 检查当前激活层级，判断是否发生了层级切换
            let mut active_levels = self.active_priority_level.write().await;
            let previous_priority = active_levels.get(app_type).copied();
            let priority_changed = previous_priority != Some(target_priority);

            if priority_changed {
                log::info!(
                    "[{}] Priority level switch detected: {:?} -> {}",
                    app_type,
                    previous_priority,
                    target_priority
                );
                // 更新当前层级
                active_levels.insert(app_type.to_string(), target_priority);
            }
            drop(active_levels);

            // 存储所有层级的providers（按优先级顺序）
            let mut all_providers_ordered = Vec::new();

            // 处理每个优先级层级
            for (priority, providers_in_level) in priority_groups.iter() {
                log::info!(
                    "[{}] Processing priority level {}: {} providers",
                    app_type,
                    priority,
                    providers_in_level.len()
                );

                // 在当前层级内按URL分组
                let mut url_groups_in_level: HashMap<String, Vec<Provider>> = HashMap::new();
                for provider in providers_in_level {
                    // Debug: 输出provider配置
                    log::debug!(
                        "[{}] Provider {} settingsConfig: {:?}",
                        app_type,
                        provider.name,
                        provider.settings_config
                    );

                    // 根据app_type选择正确的环境变量名
                    let base_url = match app_type {
                        "claude" => provider
                            .settings_config
                            .get("env")
                            .and_then(|env| env.get("ANTHROPIC_BASE_URL"))
                            .and_then(|v| v.as_str()),
                        "gemini" => provider
                            .settings_config
                            .get("env")
                            .and_then(|env| env.get("GOOGLE_GEMINI_BASE_URL"))
                            .and_then(|v| v.as_str()),
                        "codex" => {
                            // Codex的base_url直接在settingsConfig根级别
                            provider
                                .settings_config
                                .get("base_url")
                                .and_then(|v| v.as_str())
                        }
                        _ => None,
                    };

                    if let Some(url) = base_url {
                        url_groups_in_level
                            .entry(url.to_string())
                            .or_insert_with(Vec::new)
                            .push(provider.clone());
                    }
                }

                // 获取当前层级的URL顺序
                let ordered_urls_in_level = if *priority == target_priority {
                    // 只对首次可用的层级进行延迟测试（启动时或层级切换时）
                    let test_key = format!("{}:{}", app_type, priority);
                    let mut tested_map = self.priority_level_tested.write().await;
                    let already_tested = tested_map.get(&test_key).copied().unwrap_or(false);

                    if !already_tested || priority_changed {
                        log::info!(
                            "[{}] Priority {} - Performing latency test for {} URLs (first use or level switch)",
                            app_type,
                            priority,
                            url_groups_in_level.len()
                        );

                        let benchmark_results = self.benchmark_urls(app_type, *priority, &url_groups_in_level).await;
                        tested_map.insert(test_key, true);
                        drop(tested_map);

                        benchmark_results
                            .into_iter()
                            .filter(|(_, latency)| *latency != u64::MAX)
                            .map(|(url, latency)| {
                                log::info!("[{}] Priority {} URL {}: {}ms", app_type, priority, url, latency);
                                url
                            })
                            .collect::<Vec<_>>()
                    } else {
                        drop(tested_map);
                        // 使用缓存的延迟结果排序
                        let latencies = self.url_latencies.read().await;
                        let mut urls_with_latency: Vec<(String, u64)> = url_groups_in_level
                            .keys()
                            .map(|url| {
                                let cache_key = format!("{}:{}:{}", app_type, priority, url);
                                let latency = latencies
                                    .get(&cache_key)
                                    .map(|l| l.latency_ms)
                                    .unwrap_or(u64::MAX);
                                (url.clone(), latency)
                            })
                            .collect();
                        urls_with_latency.sort_by_key(|(_, latency)| *latency);
                        urls_with_latency.into_iter().map(|(url, _)| url).collect()
                    }
                } else {
                    // 其他层级不进行延迟测试，使用字母序
                    let mut urls: Vec<String> = url_groups_in_level.keys().cloned().collect();
                    urls.sort();
                    log::debug!(
                        "[{}] Priority {}: Using default order for {} URLs (not target level)",
                        app_type,
                        priority,
                        urls.len()
                    );
                    urls
                };

                // 收集当前层级所有可用的providers
                let mut providers_in_this_level = Vec::new();
                for url in ordered_urls_in_level {
                    if let Some(providers_in_url) = url_groups_in_level.get(&url) {
                        for provider in providers_in_url {
                            // 熔断器检查
                            let circuit_key = format!("{}:{}", app_type, provider.id);
                            let breaker = self.get_or_create_circuit_breaker(&circuit_key).await;

                            if breaker.is_available().await {
                                providers_in_this_level.push(provider.clone());
                            } else {
                                log::debug!(
                                    "[{}] Provider {} (priority {}) circuit breaker open, skipping",
                                    app_type,
                                    provider.name,
                                    priority
                                );
                            }
                        }
                    }
                }

                // 在当前层级内应用round-robin轮询
                if !providers_in_this_level.is_empty() {
                    let counter_key = format!("{}:priority:{}", app_type, priority);
                    let rotate_count = {
                        let mut counters = self.round_robin_counters.write().await;
                        let counter = counters.entry(counter_key.clone()).or_insert(0);
                        let count = *counter % providers_in_this_level.len();
                        // 递增计数器，下次请求会从下一个provider开始
                        *counter = (*counter + 1) % providers_in_this_level.len();
                        count
                    };

                    // 旋转列表，使轮询选中的provider移到层级首位
                    providers_in_this_level.rotate_left(rotate_count);

                    log::info!(
                        "[{}] Priority {} round-robin: starting with {} (rotation {}/{})",
                        app_type,
                        priority,
                        providers_in_this_level[0].name,
                        rotate_count,
                        providers_in_this_level.len()
                    );

                    // 将轮询后的providers添加到总列表
                    all_providers_ordered.extend(providers_in_this_level);
                }
            }

            if all_providers_ordered.is_empty() {
                log::error!("[{}] 没有可用的providers（所有providers都被熔断器阻止或测试失败）", app_type);
                return Err(AppError::Config(format!(
                    "No available providers for {app_type}"
                )));
            }

            log::info!(
                "[{}] Provider chain ready: {} providers across {} priority levels",
                app_type,
                all_providers_ordered.len(),
                priority_groups.len()
            );

            result = all_providers_ordered;
        } else {
            // 故障转移关闭：仅使用当前供应商，跳过熔断器检查
            // 原因：单 Provider 场景下，熔断器打开会导致所有请求失败，用户体验差
            log::info!("[{app_type}] Failover disabled, using current provider only (circuit breaker bypassed)");

            if let Some(current_id) = self.db.get_current_provider(app_type)? {
                if let Some(current) = self.db.get_provider_by_id(&current_id, app_type)? {
                    log::info!(
                        "[{}] Current provider: {} ({})",
                        app_type,
                        current.name,
                        current.id
                    );
                    result.push(current);
                }
            }
        }

        if result.is_empty() {
            return Err(AppError::Config(format!(
                "No available provider for {app_type} (all circuit breakers open or no providers configured)"
            )));
        }

        log::info!(
            "[{}] Provider chain: {} provider(s) available",
            app_type,
            result.len()
        );

        Ok(result)
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
            log::warn!(
                "Provider {} request failed: {}",
                provider_id,
                error_msg.as_deref().unwrap_or("Unknown error")
            );
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
    pub async fn test_url_latency(&self, provider: &Provider, app_type: &str) -> Result<u64, String> {
        // 根据app_type提取base_url
        let base_url = match app_type {
            "claude" => provider
                .settings_config
                .get("env")
                .and_then(|env| env.get("ANTHROPIC_BASE_URL"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Provider缺少ANTHROPIC_BASE_URL配置".to_string())?,
            "gemini" => provider
                .settings_config
                .get("env")
                .and_then(|env| env.get("GOOGLE_GEMINI_BASE_URL"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Provider缺少GOOGLE_GEMINI_BASE_URL配置".to_string())?,
            "codex" => {
                // Codex的base_url直接在settingsConfig根级别
                provider
                    .settings_config
                    .get("base_url")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Provider缺少base_url配置".to_string())?
            }
            _ => return Err(format!("不支持的app_type: {}", app_type)),
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
                .ok_or_else(|| "Provider缺少API key配置".to_string())?,
            "gemini" => provider
                .settings_config
                .get("env")
                .and_then(|env| env.get("GOOGLE_API_KEY"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Provider缺少GOOGLE_API_KEY配置".to_string())?,
            "codex" => provider
                .settings_config
                .get("env")
                .and_then(|env| env.get("OPENAI_API_KEY"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Provider缺少OPENAI_API_KEY配置".to_string())?,
            _ => return Err(format!("不支持的app_type: {}", app_type)),
        };

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| format!("创建HTTP客户端失败: {}", e))?;

        let start = std::time::Instant::now();

        let response = if app_type == "codex" {
            // Codex: 直接测试目标URL，使用OpenAI格式
            let test_payload = serde_json::json!({
                "model": "gpt-3.5-turbo",
                "max_tokens": 10,
                "messages": [{
                    "role": "user",
                    "content": "1+1=?"
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
                .map_err(|e| format!("请求失败: {}", e))?
        } else {
            // Claude: 通过Python代理测试，使用Claude格式
            let test_payload = serde_json::json!({
                "model": "claude-haiku-4-5-20251001",
                "max_tokens": 10,
                "messages": [{
                    "role": "user",
                    "content": "1+1=?"
                }]
            });

            client
                .post("http://127.0.0.1:15722/v1/messages")
                .header("Content-Type", "application/json")
                .header("anthropic-version", "2023-06-01")
                .header("x-api-key", api_key)
                .header("x-target-base-url", base_url)
                .json(&test_payload)
                .send()
                .await
                .map_err(|e| format!("请求失败: {}", e))?
        };

        let status = response.status();
        if !status.is_success() {
            return Err(format!("HTTP错误: {}", status));
        }

        // 等待响应体完成（确保测量完整响应时间）
        let _ = response.bytes().await.map_err(|e| format!("读取响应失败: {}", e))?;

        let latency = start.elapsed().as_millis() as u64;
        Ok(latency)
    }

    /// 测试所有URL并返回延迟排序结果
    ///
    /// 返回: Vec<(url, latency_ms)> 按延迟从低到高排序
    pub async fn benchmark_urls(
        &self,
        app_type: &str,
        priority: usize,
        url_groups: &HashMap<String, Vec<Provider>>,
    ) -> Vec<(String, u64)> {
        log::info!("[{}:{}] 开始URL延迟测试，共{}个URL", app_type, priority, url_groups.len());

        let mut results = Vec::new();

        for (url, providers) in url_groups {
            // 选择该URL的第一个provider进行测试
            if let Some(provider) = providers.first() {
                log::info!("[{}:{}] 测试URL: {} (使用provider: {})", app_type, priority, url, provider.name);

                match self.test_url_latency(provider, app_type).await {
                    Ok(latency) => {
                        log::info!("[{}:{}] URL {} 延迟: {}ms", app_type, priority, url, latency);
                        results.push((url.clone(), latency));

                        // 缓存测试结果（包含优先级信息）
                        let cache_key = format!("{}:{}:{}", app_type, priority, url);
                        let mut latencies = self.url_latencies.write().await;
                        latencies.insert(
                            cache_key,
                            UrlLatency {
                                latency_ms: latency,
                                tested_at: std::time::Instant::now(),
                            },
                        );
                    }
                    Err(e) => {
                        log::warn!("[{}:{}] URL {} 测试失败: {}", app_type, priority, url, e);
                        // 失败的URL设置高延迟值，排序时会靠后
                        results.push((url.clone(), u64::MAX));
                    }
                }
            }
        }

        // 按延迟排序
        results.sort_by_key(|(_, latency)| *latency);

        log::info!(
            "[{}:{}] URL延迟测试完成，最快: {} ({}ms)",
            app_type,
            priority,
            results.first().map(|(u, _)| u.as_str()).unwrap_or("N/A"),
            results.first().map(|(_, l)| *l).unwrap_or(0)
        );

        results
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
        let providers = router.select_providers("claude").await.unwrap();

        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].id, "a");
    }

    #[tokio::test]
    async fn test_failover_enabled_uses_queue_order() {
        let db = Arc::new(Database::memory().unwrap());

        // 设置 sort_index 来控制顺序：b=1, a=2
        let mut provider_a =
            Provider::with_id("a".to_string(), "Provider A".to_string(), json!({}), None);
        provider_a.sort_index = Some(2);
        let mut provider_b =
            Provider::with_id("b".to_string(), "Provider B".to_string(), json!({}), None);
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
        let providers = router.select_providers("claude").await.unwrap();

        assert_eq!(providers.len(), 2);
        // 按 sort_index 排序：b(1) 在前，a(2) 在后
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
            Provider::with_id("a".to_string(), "Provider A".to_string(), json!({}), None);
        let provider_b =
            Provider::with_id("b".to_string(), "Provider B".to_string(), json!({}), None);

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

        let providers = router.select_providers("claude").await.unwrap();
        assert_eq!(providers.len(), 2);

        assert!(router.allow_provider_request("b", "claude").await.allowed);
    }
}
