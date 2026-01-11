//! 模型映射模块
//!
//! 在请求转发前，根据 Provider 配置替换请求中的模型名称

use crate::provider::Provider;
use crate::proxy::model_catalog::{detect_model_family, is_same_family, ModelFamily};
use serde_json::Value;

/// 模型映射配置
pub struct ModelMapping {
    pub haiku_model: Option<String>,
    pub sonnet_model: Option<String>,
    pub opus_model: Option<String>,
    pub default_model: Option<String>,
    pub reasoning_model: Option<String>,
}

impl ModelMapping {
    /// 从 Provider 配置中提取模型映射
    pub fn from_provider(provider: &Provider) -> Self {
        let env = provider.settings_config.get("env");

        Self {
            haiku_model: env
                .and_then(|e| e.get("ANTHROPIC_DEFAULT_HAIKU_MODEL"))
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(String::from),
            sonnet_model: env
                .and_then(|e| e.get("ANTHROPIC_DEFAULT_SONNET_MODEL"))
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(String::from),
            opus_model: env
                .and_then(|e| e.get("ANTHROPIC_DEFAULT_OPUS_MODEL"))
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(String::from),
            default_model: env
                .and_then(|e| e.get("ANTHROPIC_MODEL"))
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(String::from),
            reasoning_model: env
                .and_then(|e| e.get("ANTHROPIC_REASONING_MODEL"))
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(String::from),
        }
    }

    /// 检查是否配置了任何模型映射
    pub fn has_mapping(&self) -> bool {
        self.haiku_model.is_some()
            || self.sonnet_model.is_some()
            || self.opus_model.is_some()
            || self.default_model.is_some()
            || self.reasoning_model.is_some()
    }

    /// 根据原始模型名称获取映射后的模型
    pub fn map_model(&self, original_model: &str, has_thinking: bool) -> String {
        let model_lower = original_model.to_lowercase();

        fn claude_family(lower: &str) -> Option<&'static str> {
            if lower.contains("haiku") {
                return Some("haiku");
            }
            if lower.contains("sonnet") {
                return Some("sonnet");
            }
            if lower.contains("opus") {
                return Some("opus");
            }
            None
        }

        // 约束：家族守护（Claude/GPT/Gemini/Llama 等）
        // - 若原始模型能识别家族，则映射目标必须仍在同家族内（严禁跨到 GLM/GPT 等）。
        // - Claude 额外要求：尽量保持 haiku/sonnet/opus 一致（避免“性能断崖”）。
        let original_family = detect_model_family(original_model);
        let original_claude_family = claude_family(&model_lower);

        let is_acceptable_mapping = |mapped: &str| -> bool {
            let mapped_lower = mapped.to_lowercase();
            // 1) 家族锚定：必须同家族（保守：只有请求可识别时才强制）
            if !is_same_family(original_model, mapped) {
                return false;
            }

            // 2) Claude 的子家族守护：尽量保持 haiku/sonnet/opus 一致
            if original_family == ModelFamily::Claude {
                // 映射值本身缺少 haiku/sonnet/opus 关键词时放行（交给后续智能解析兜底）
                if let Some(f) = original_claude_family {
                    let mapped_family = claude_family(&mapped_lower);
                    return mapped_family.is_none() || mapped_lower.contains(f);
                }
            }
            true
        };

        // 1. thinking 模式优先使用推理模型
        if has_thinking {
            if let Some(ref m) = self.reasoning_model {
                if is_acceptable_mapping(m) {
                    return m.clone();
                }
            }
        }

        // 2. 按模型类型匹配
        if model_lower.contains("haiku") {
            if let Some(ref m) = self.haiku_model {
                if is_acceptable_mapping(m) {
                    return m.clone();
                }
            }
        }
        if model_lower.contains("opus") {
            if let Some(ref m) = self.opus_model {
                if is_acceptable_mapping(m) {
                    return m.clone();
                }
            }
        }
        if model_lower.contains("sonnet") {
            if let Some(ref m) = self.sonnet_model {
                if is_acceptable_mapping(m) {
                    return m.clone();
                }
            }
        }

        // 3. 默认模型
        if let Some(ref m) = self.default_model {
            if is_acceptable_mapping(m) {
                return m.clone();
            }
        }

        // 4. 无映射，保持原样
        original_model.to_string()
    }
}

/// 检测请求是否启用了 thinking 模式
pub fn has_thinking_enabled(body: &Value) -> bool {
    body.get("thinking")
        .and_then(|v| v.as_object())
        .and_then(|o| o.get("type"))
        .and_then(|t| t.as_str())
        == Some("enabled")
}

/// 对请求体应用模型映射
///
/// 返回 (映射后的请求体, 原始模型名, 映射后模型名)
pub fn apply_model_mapping(
    mut body: Value,
    provider: &Provider,
) -> (Value, Option<String>, Option<String>) {
    let mapping = ModelMapping::from_provider(provider);

    // 如果没有配置映射，直接返回
    if !mapping.has_mapping() {
        let original = body.get("model").and_then(|m| m.as_str()).map(String::from);
        return (body, original, None);
    }

    // 提取原始模型名
    let original_model = body.get("model").and_then(|m| m.as_str()).map(String::from);

    if let Some(ref original) = original_model {
        let has_thinking = has_thinking_enabled(&body);
        let mapped = mapping.map_model(original, has_thinking);

        if mapped != *original {
            body["model"] = serde_json::json!(mapped);
            return (body, Some(original.clone()), Some(mapped));
        }
    }

    (body, original_model, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_provider_with_mapping() -> Provider {
        Provider {
            id: "test".to_string(),
            name: "Test".to_string(),
            settings_config: json!({
                    "env": {
                        "ANTHROPIC_MODEL": "cursor2-claude-4.5-sonnet",
                    "ANTHROPIC_DEFAULT_HAIKU_MODEL": "claude-haiku-4-5-2cc",
                    "ANTHROPIC_DEFAULT_SONNET_MODEL": "cursor2-claude-4.5-sonnet",
                    "ANTHROPIC_DEFAULT_OPUS_MODEL": "claude-opus-4-5-2cc",
                    "ANTHROPIC_REASONING_MODEL": "claude-sonnet-4-5-thinking"
                }
            }),
            website_url: None,
            category: None,
            created_at: None,
            sort_index: None,
            notes: None,
            meta: None,
            icon: None,
            icon_color: None,
            in_failover_queue: false,
        }
    }

    fn create_provider_without_mapping() -> Provider {
        Provider {
            id: "test".to_string(),
            name: "Test".to_string(),
            settings_config: json!({}),
            website_url: None,
            category: None,
            created_at: None,
            sort_index: None,
            notes: None,
            meta: None,
            icon: None,
            icon_color: None,
            in_failover_queue: false,
        }
    }

    fn create_provider_with_reasoning_only() -> Provider {
        Provider {
            id: "test".to_string(),
            name: "Test".to_string(),
            settings_config: json!({
                "env": {
                    "ANTHROPIC_REASONING_MODEL": "claude-sonnet-4-5-thinking"
                }
            }),
            website_url: None,
            category: None,
            created_at: None,
            sort_index: None,
            notes: None,
            meta: None,
            icon: None,
            icon_color: None,
            in_failover_queue: false,
        }
    }

    #[test]
    fn test_sonnet_mapping() {
        let provider = create_provider_with_mapping();
        let body = json!({"model": "claude-sonnet-4-5-20250929"});
        let (result, original, mapped) = apply_model_mapping(body, &provider);
        assert_eq!(result["model"], "cursor2-claude-4.5-sonnet");
        assert_eq!(original, Some("claude-sonnet-4-5-20250929".to_string()));
        assert_eq!(mapped, Some("cursor2-claude-4.5-sonnet".to_string()));
    }

    #[test]
    fn test_haiku_mapping() {
        let provider = create_provider_with_mapping();
        let body = json!({"model": "claude-haiku-4-5"});
        let (result, _, mapped) = apply_model_mapping(body, &provider);
        assert_eq!(result["model"], "claude-haiku-4-5-2cc");
        assert_eq!(mapped, Some("claude-haiku-4-5-2cc".to_string()));
    }

    #[test]
    fn test_opus_mapping() {
        let provider = create_provider_with_mapping();
        let body = json!({"model": "claude-opus-4-5"});
        let (result, _, mapped) = apply_model_mapping(body, &provider);
        assert_eq!(result["model"], "claude-opus-4-5-2cc");
        assert_eq!(mapped, Some("claude-opus-4-5-2cc".to_string()));
    }

    #[test]
    fn test_thinking_mode() {
        let provider = create_provider_with_mapping();
        let body = json!({
            "model": "claude-sonnet-4-5",
            "thinking": {"type": "enabled"}
        });
        let (result, _, mapped) = apply_model_mapping(body, &provider);
        assert_eq!(result["model"], "claude-sonnet-4-5-thinking");
        assert_eq!(mapped, Some("claude-sonnet-4-5-thinking".to_string()));
    }

    #[test]
    fn test_reasoning_only_mapping_in_thinking_mode() {
        let provider = create_provider_with_reasoning_only();
        let body = json!({
            "model": "claude-sonnet-4-5",
            "thinking": {"type": "enabled"}
        });
        let (result, _, mapped) = apply_model_mapping(body, &provider);
        assert_eq!(result["model"], "claude-sonnet-4-5-thinking");
        assert_eq!(mapped, Some("claude-sonnet-4-5-thinking".to_string()));
    }

    #[test]
    fn test_reasoning_only_mapping_does_not_affect_non_thinking() {
        let provider = create_provider_with_reasoning_only();
        let body = json!({
            "model": "claude-sonnet-4-5",
            "thinking": {"type": "disabled"}
        });
        let (result, original, mapped) = apply_model_mapping(body, &provider);
        assert_eq!(result["model"], "claude-sonnet-4-5");
        assert_eq!(original, Some("claude-sonnet-4-5".to_string()));
        assert!(mapped.is_none());
    }

    #[test]
    fn test_thinking_disabled() {
        let provider = create_provider_with_mapping();
        let body = json!({
            "model": "claude-sonnet-4-5",
            "thinking": {"type": "disabled"}
        });
        let (result, _, mapped) = apply_model_mapping(body, &provider);
        assert_eq!(result["model"], "cursor2-claude-4.5-sonnet");
        assert_eq!(mapped, Some("cursor2-claude-4.5-sonnet".to_string()));
    }

    #[test]
    fn test_unknown_model_uses_default() {
        let provider = create_provider_with_mapping();
        let body = json!({"model": "some-unknown-model"});
        let (result, _, mapped) = apply_model_mapping(body, &provider);
        assert_eq!(result["model"], "cursor2-claude-4.5-sonnet");
        assert_eq!(mapped, Some("cursor2-claude-4.5-sonnet".to_string()));
    }

    #[test]
    fn test_no_mapping_configured() {
        let provider = create_provider_without_mapping();
        let body = json!({"model": "claude-sonnet-4-5"});
        let (result, original, mapped) = apply_model_mapping(body, &provider);
        assert_eq!(result["model"], "claude-sonnet-4-5");
        assert_eq!(original, Some("claude-sonnet-4-5".to_string()));
        assert!(mapped.is_none());
    }

    #[test]
    fn test_case_insensitive() {
        let provider = create_provider_with_mapping();
        let body = json!({"model": "Claude-SONNET-4-5"});
        let (result, _, mapped) = apply_model_mapping(body, &provider);
        assert_eq!(result["model"], "cursor2-claude-4.5-sonnet");
        assert_eq!(mapped, Some("cursor2-claude-4.5-sonnet".to_string()));
    }

    #[test]
    fn test_claude_mapping_rejects_cross_family() {
        let provider = Provider {
            id: "test".to_string(),
            name: "Test".to_string(),
            settings_config: json!({
                "env": {
                    "ANTHROPIC_DEFAULT_HAIKU_MODEL": "zai-org/GLM-4.5"
                }
            }),
            website_url: None,
            category: None,
            created_at: None,
            sort_index: None,
            notes: None,
            meta: None,
            icon: None,
            icon_color: None,
            in_failover_queue: false,
        };

        let body = json!({"model": "claude-haiku-4-5"});
        let (result, original, mapped) = apply_model_mapping(body, &provider);
        assert_eq!(result["model"], "claude-haiku-4-5");
        assert_eq!(original, Some("claude-haiku-4-5".to_string()));
        assert!(mapped.is_none());
    }
}
