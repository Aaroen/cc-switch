//! 模型家族识别（用于“家族锚定”，避免跨家族映射导致体验断崖）
//!
//! 说明：
//! - 该模块不做联网；家族关键词来源于主流模型命名习惯（并参考 artificialanalysis.ai/models 的常见族群）。
//! - 目标是“保守识别”：能识别则严格同家族，否则返回 Other（不做限制）。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelFamily {
    Claude,
    OpenAi, // gpt-* / o* / chatgpt 等
    Gemini,
    Llama,
    Qwen,
    Mistral,
    DeepSeek,
    Grok,
    Phi,
    Gemma,
    Glm,
    Kimi,
    Yi,
    Command, // Cohere Command
    Jamba,
    Other,
}

fn normalize(s: &str) -> String {
    s.trim().to_lowercase()
}

pub fn detect_model_family(model_id: &str) -> ModelFamily {
    let s = normalize(model_id);
    if s.is_empty() {
        return ModelFamily::Other;
    }

    // provider 前缀常见：anthropic/claude-*, openai/gpt-*, google/gemini-*
    let s = s.split('/').last().unwrap_or(s.as_str()).to_string();

    // Claude
    if s.contains("claude") {
        return ModelFamily::Claude;
    }

    // OpenAI: gpt-* / o1 / o3-mini / chatgpt / gpt-oss-*
    if s.starts_with("gpt-")
        || s == "chatgpt"
        || s.starts_with("o1")
        || s.starts_with("o3")
        || s.starts_with("o4")
        || s.starts_with("o5")
    {
        return ModelFamily::OpenAi;
    }

    // Gemini（本轮不改其智能逻辑，但允许家族识别）
    if s.contains("gemini") {
        return ModelFamily::Gemini;
    }

    // Llama
    if s.contains("llama") || s.contains("meta-llama") {
        return ModelFamily::Llama;
    }

    // Qwen
    if s.contains("qwen") {
        return ModelFamily::Qwen;
    }

    // Mistral / Mixtral
    if s.contains("mistral") || s.contains("mixtral") {
        return ModelFamily::Mistral;
    }

    // DeepSeek
    if s.contains("deepseek") {
        return ModelFamily::DeepSeek;
    }

    // Grok (xAI)
    if s.contains("grok") {
        return ModelFamily::Grok;
    }

    // Phi
    if s.contains("phi-") || s.starts_with("phi") {
        return ModelFamily::Phi;
    }

    // Gemma
    if s.contains("gemma") {
        return ModelFamily::Gemma;
    }

    // GLM
    if s.contains("glm") {
        return ModelFamily::Glm;
    }

    // Kimi / Moonshot
    if s.contains("kimi") || s.contains("moonshot") {
        return ModelFamily::Kimi;
    }

    // Yi
    if s.contains("yi-") || s.starts_with("yi-") || s.contains("01-ai") {
        return ModelFamily::Yi;
    }

    // Cohere Command
    if s.contains("command") {
        return ModelFamily::Command;
    }

    // AI21 Jamba
    if s.contains("jamba") {
        return ModelFamily::Jamba;
    }

    ModelFamily::Other
}

pub fn is_same_family(request_model: &str, candidate_model: &str) -> bool {
    let a = detect_model_family(request_model);
    let b = detect_model_family(candidate_model);
    // 保守：只有当请求能识别家族时才强制；否则一律放行
    if a == ModelFamily::Other {
        return true;
    }
    a == b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_common_families() {
        assert_eq!(detect_model_family("claude-sonnet-4-5"), ModelFamily::Claude);
        assert_eq!(detect_model_family("anthropic/claude-haiku-4.5"), ModelFamily::Claude);
        assert_eq!(detect_model_family("gpt-5.2"), ModelFamily::OpenAi);
        assert_eq!(detect_model_family("openai/gpt-4o"), ModelFamily::OpenAi);
        assert_eq!(detect_model_family("o3-mini"), ModelFamily::OpenAi);
        assert_eq!(detect_model_family("gemini-2.5-pro"), ModelFamily::Gemini);
        assert_eq!(detect_model_family("meta-llama/llama-3.1-70b"), ModelFamily::Llama);
        assert_eq!(detect_model_family("qwen2.5-72b"), ModelFamily::Qwen);
        assert_eq!(detect_model_family("mistral-large"), ModelFamily::Mistral);
        assert_eq!(detect_model_family("deepseek-r1"), ModelFamily::DeepSeek);
        assert_eq!(detect_model_family("grok-2"), ModelFamily::Grok);
        assert_eq!(detect_model_family("phi-4"), ModelFamily::Phi);
        assert_eq!(detect_model_family("gemma-2"), ModelFamily::Gemma);
        assert_eq!(detect_model_family("glm-4.5"), ModelFamily::Glm);
        assert_eq!(detect_model_family("kimi-k2"), ModelFamily::Kimi);
        assert_eq!(detect_model_family("yi-34b"), ModelFamily::Yi);
        assert_eq!(detect_model_family("command-r"), ModelFamily::Command);
        assert_eq!(detect_model_family("jamba-1.5"), ModelFamily::Jamba);
    }

    #[test]
    fn same_family_guard_is_conservative() {
        assert!(is_same_family("unknown-model", "glm-4.5"));
        assert!(!is_same_family("claude-sonnet-4-5", "glm-4.5"));
        assert!(!is_same_family("gpt-5.2", "deepseek-r1"));
    }
}

