//! 请求上下文模块
//!
//! 提供请求生命周期的上下文管理，封装通用初始化逻辑

use crate::app_config::AppType;
use crate::provider::Provider;
use crate::proxy::{
    forwarder::RequestForwarder, router::ProviderRouter, server::ProxyState, types::ProxyConfig,
    ProxyError,
};
use std::time::Instant;

/// 请求上下文
///
/// 贯穿整个请求生命周期，包含：
/// - 计时信息
/// - 代理配置
/// - 选中的 Provider
/// - 请求模型名称
/// - 日志标签
pub struct RequestContext {
    /// 请求开始时间
    pub start_time: Instant,
    /// 代理配置快照
    pub config: ProxyConfig,
    /// 选中的 Provider
    pub provider: Provider,
    /// 请求中的模型名称
    pub request_model: String,
    /// 日志标签（如 "Claude"、"Codex"、"Gemini"）
    pub tag: &'static str,
    /// 应用类型字符串（如 "claude"、"codex"、"gemini"）
    pub app_type_str: &'static str,
    /// 应用类型（预留，目前通过 app_type_str 使用）
    #[allow(dead_code)]
    pub app_type: AppType,
}

impl RequestContext {
    /// 创建请求上下文
    ///
    /// # Arguments
    /// * `state` - 代理服务器状态
    /// * `body` - 请求体 JSON
    /// * `app_type` - 应用类型
    /// * `tag` - 日志标签
    /// * `app_type_str` - 应用类型字符串
    ///
    /// # Errors
    /// 返回 `ProxyError` 如果 Provider 选择失败
    pub async fn new(
        state: &ProxyState,
        body: &serde_json::Value,
        app_type: AppType,
        tag: &'static str,
        app_type_str: &'static str,
    ) -> Result<Self, ProxyError> {
        let start_time = Instant::now();
        let config = state.config.read().await.clone();

        // 从请求体提取模型名称
        let request_model = body
            .get("model")
            .and_then(|m| m.as_str())
            .unwrap_or("unknown")
            .to_string();

        // Provider 选择
        let router = ProviderRouter::new(state.db.clone());
        let provider = router.select_provider(&app_type, &[]).await?;

        log::info!(
            "[{}] Provider: {}, model: {}",
            tag,
            provider.name,
            request_model
        );

        Ok(Self {
            start_time,
            config,
            provider,
            request_model,
            tag,
            app_type_str,
            app_type,
        })
    }

    /// 从 URI 提取模型名称（Gemini 专用）
    ///
    /// Gemini API 的模型名称在 URI 中，格式如：
    /// `/v1beta/models/gemini-pro:generateContent`
    pub fn with_model_from_uri(mut self, uri: &axum::http::Uri) -> Self {
        let endpoint = uri
            .path_and_query()
            .map(|pq| pq.as_str())
            .unwrap_or(uri.path());

        self.request_model = endpoint
            .split('/')
            .find(|s| s.starts_with("models/"))
            .and_then(|s| s.strip_prefix("models/"))
            .map(|s| s.split(':').next().unwrap_or(s))
            .unwrap_or("unknown")
            .to_string();

        log::info!("[{}] 从 URI 提取模型: {}", self.tag, self.request_model);
        self
    }

    /// 创建 RequestForwarder
    pub fn create_forwarder(&self, state: &ProxyState) -> RequestForwarder {
        RequestForwarder::new(
            state.db.clone(),
            self.config.request_timeout,
            self.config.max_retries,
            state.status.clone(),
            state.current_providers.clone(),
        )
    }

    /// 计算请求延迟（毫秒）
    #[inline]
    pub fn latency_ms(&self) -> u64 {
        self.start_time.elapsed().as_millis() as u64
    }
}
