//! 供应商路由器模块
//!
//! 负责选择和管理代理目标供应商，实现智能故障转移

use crate::database::Database;
use crate::error::AppError;
use crate::provider::Provider;
use crate::proxy::circuit_breaker::{AllowResult, CircuitBreaker, CircuitBreakerConfig};
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
}

impl ProviderRouter {
    const CONNECTIVITY_TIMEOUT: Duration = Duration::from_secs(5);
    const CONNECTIVITY_PENALTY_MS: u64 = 30_000;

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
            // 故障转移开启：按层级逐级推进（只在一个层级内工作，层级整体不可用时才进入下一层级）
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

            let mut selected_priority: Option<usize> = None;
            let mut selected_chain: Vec<Provider> = Vec::new();

            for (priority, providers_in_level) in priority_groups.iter() {
                // 在当前层级内按供应商 -> URL -> providers 分组
                let mut supplier_urls: HashMap<String, HashMap<String, Vec<Provider>>> =
                    HashMap::new();

                for provider in providers_in_level {
                    let supplier = Self::supplier_name(provider);
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
                    if self.is_supplier_in_cooldown(app_type, *priority, supplier).await {
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

                                // “有效”判断：优先使用已有缓存；否则做一次快速连通性探测
                                let cache_key = Self::url_latency_key(app_type, *priority, supplier, purl);
                                let cached_latency = {
                                    let latencies = self.url_latencies.read().await;
                                    latencies.get(&cache_key).map(|l| l.latency_ms)
                                };

                                let connect_ms = if cached_latency.is_none() {
                                    self.connectivity_latency(purl).await.ok()
                                } else {
                                    None
                                };

                                let ok = match cached_latency {
                                    Some(l) if l != u64::MAX => true,
                                    _ => connect_ms.is_some(),
                                };

                                if ok {
                                    // 若无缓存，写入一个带 penalty 的可用延迟（避免之后重复探测）
                                    if let Some(connect_ms) = connect_ms {
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

                                    selected_url = Some(purl.clone());
                                    self.set_supplier_current_url(app_type, *priority, supplier, purl)
                                        .await;
                                    break;
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

                            // 若存在 URL 优先级配置，则在“可用 URL 列表”内应用优先级（不改变可用性，只改变选择顺序）
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
                                filtered_urls = Self::apply_url_priority(filtered_urls, &preferred);
                            }

                            if let Some(url) = filtered_urls.first() {
                                selected_url = Some(url.clone());
                                self.set_supplier_current_url(app_type, *priority, supplier, url)
                                    .await;
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

                selected_priority = Some(*priority);
                selected_chain = candidates;
                break;
            }

            let Some(target_priority) = selected_priority else {
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
                "[{}] Selected priority {} with {} key(s) (model={})",
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
    pub async fn test_url_latency(
        &self,
        provider: &Provider,
        app_type: &str,
        request_model: &str,
    ) -> Result<u64, String> {
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
                .map_err(|e| format!("请求失败: {}", e))?
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
                .header("x-stainless-package-version", env!("CARGO_PKG_VERSION"))
                .header("X-API-Key", api_key)
                .header("x-target-base-url", base_url)
                .json(&test_payload)
                .send()
                .await
                .map_err(|e| format!("请求失败: {}", e))?
        } else {
            // Gemini（或其他）：暂无稳定的“全链路问答”探测格式，这里仅进行基础连通性探测。
            client
                .get(base_url)
                .send()
                .await
                .map_err(|e| format!("请求失败: {}", e))?
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
            Err(format!("HTTP错误: {}", status))
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
        #[derive(Debug)]
        enum ProbeSummary {
            FullChainOk { latency_ms: u64 },
            FallbackOk {
                connect_ms: u64,
                total_ms: u64,
                reason: String,
            },
            Failed { reason: String },
        }

        let mut results = Vec::new();
        let mut summaries: Vec<(String, ProbeSummary)> = Vec::new();
        let mut full_ok_count: usize = 0;
        let mut fallback_ok_count: usize = 0;
        let mut fail_count: usize = 0;

        for (url, providers) in url_groups {
            // 尽量模拟真实环境：同一 URL 下可能有多个 key，真实使用会在同一 URL 上轮询 key。
            // 因此测速不能“只测第一个 key 就判死”；这里按 key 去重并尝试少量 key，避免测速过重。
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

            let mut full_chain_ok: Option<u64> = None;
            let mut full_chain_errs: Vec<String> = Vec::new();

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
                        full_chain_ok = Some(latency);
                        break;
                    }
                    Err(e) => full_chain_errs.push(e),
                }
            }

            if let Some(latency) = full_chain_ok {
                results.push((url.clone(), latency));
                full_ok_count += 1;
                summaries.push(
                    (
                        url.clone(),
                        ProbeSummary::FullChainOk { latency_ms: latency },
                    )
                );

                let cache_key = Self::url_latency_key(app_type, priority, supplier, url);
                let mut latencies = self.url_latencies.write().await;
                latencies.insert(
                    cache_key,
                    UrlLatency {
                        latency_ms: latency,
                        tested_at: std::time::Instant::now(),
                    },
                );
            } else {
                let err_summary = if full_chain_errs.is_empty() {
                    "未知错误".to_string()
                } else {
                    full_chain_errs.join("; ")
                };
                let err_short = Self::shorten_for_log(&err_summary, 80);

                // 回退到简单连通性测试，避免“全链路测试形态不一致”导致把可用URL判死
                match self.connectivity_latency(url).await {
                    Ok(connect_latency) => {
                        let latency =
                            connect_latency.saturating_add(Self::CONNECTIVITY_PENALTY_MS);
                        fallback_ok_count += 1;
                        summaries.push(
                            (
                                url.clone(),
                                ProbeSummary::FallbackOk {
                                    connect_ms: connect_latency,
                                    total_ms: latency,
                                    reason: err_short,
                                },
                            )
                        );
                        results.push((url.clone(), latency));

                        // 缓存回退结果（带 penalty 的延迟），避免每次请求都因“无有效测速结果”而重复测速刷屏
                        let cache_key = Self::url_latency_key(app_type, priority, supplier, url);
                        let mut latencies = self.url_latencies.write().await;
                        latencies.insert(
                            cache_key,
                            UrlLatency {
                                latency_ms: latency,
                                tested_at: std::time::Instant::now(),
                            },
                        );
                    }
                    Err(connect_err) => {
                        fail_count += 1;
                        summaries.push(
                            (
                                url.clone(),
                                ProbeSummary::Failed {
                                    reason: format!(
                                        "全链路失败={}; 连通性失败={}",
                                        err_short,
                                        Self::shorten_for_log(&connect_err, 80)
                                    ),
                                },
                            )
                        );
                        results.push((url.clone(), u64::MAX));
                    }
                }
            }
        }

        // 按延迟排序
        results.sort_by_key(|(_, latency)| *latency);

        // 计算“最终选择”：排除 MAX 与 suspect 后的最低延迟 URL（与 select_providers 的过滤规则保持一致）
        let mut selected_url: Option<String> = None;
        let mut selected_latency: Option<u64> = None;
        for (u, l) in results.iter() {
            if *l == u64::MAX {
                continue;
            }
            if self.is_url_suspect(app_type, supplier, u).await {
                continue;
            }
            selected_url = Some(u.clone());
            selected_latency = Some(*l);
            break;
        }

        // 组装简洁摘要（INFO）：只在测速结束时输出，不输出过程性冗余内容
        summaries.sort_by(|a, b| a.0.cmp(&b.0));
        let detail = summaries
            .iter()
            .map(|(u, s)| match s {
                ProbeSummary::FullChainOk { latency_ms } => format!("{u}=OK({latency_ms}ms)"),
                ProbeSummary::FallbackOk {
                    connect_ms,
                    total_ms,
                    reason,
                } => format!("{u}=FB({connect_ms}ms→{total_ms}ms, {reason})"),
                ProbeSummary::Failed { reason } => format!("{u}=FAIL({reason})"),
            })
            .collect::<Vec<String>>()
            .join("; ");

        let selected_text = match (&selected_url, selected_latency) {
            (Some(u), Some(l)) => format!("{u} ({l}ms)"),
            _ => "N/A".to_string(),
        };

        // 若全失败，用 WARN 提醒；否则用 INFO 给出清晰结论。
        if full_ok_count == 0 && fallback_ok_count == 0 {
            log::warn!(
                "[{}:{}] 测速结束 supplier={} model={} 结果: 全失败(full_ok=0 fallback_ok=0 fail={}) 选用={} 详情: {}",
                app_type,
                priority,
                supplier,
                request_model,
                fail_count,
                selected_text,
                detail
            );
        } else {
            log::info!(
                "[{}:{}] 测速结束 supplier={} model={} 结果: full_ok={} fallback_ok={} fail={} 选用={} 详情: {}",
                app_type,
                priority,
                supplier,
                request_model,
                full_ok_count,
                fallback_ok_count,
                fail_count,
                selected_text,
                detail
            );
        }

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

        // 按层级逐级推进：priority=1 可用时，不会进入 priority=2
        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].id, "b");
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
