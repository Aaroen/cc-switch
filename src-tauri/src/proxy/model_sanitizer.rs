//! 模型名称清洗（用于避免“伪模型名”污染请求/日志/缓存）
//!
//! 当前重点：OpenAI/Codex 的 `gpt-*` 模型不允许携带日期后缀（例如 `gpt-5.2-2025-12-11`）。

use serde_json::Value;

/// 清洗 OpenAI/Codex 的 GPT 模型名：
/// - `gpt-5.2-2025-12-11` -> `gpt-5.2`
/// - `gpt-5.2-20251211`   -> `gpt-5.2`
/// - `gpt-4-0613`         -> `gpt-4`
/// - `gpt-4-1106-preview` -> `gpt-4`
/// - 任意包含 `-202` 的 gpt-* -> 截断到 `-202` 之前
pub fn sanitize_gpt_model_name(model: &str) -> String {
    let trimmed = model.trim();
    if trimmed.is_empty() {
        return trimmed.to_string();
    }
    let lower = trimmed.to_lowercase();
    if !lower.starts_with("gpt-") {
        return trimmed.to_string();
    }

    fn is_all_digits(s: &str) -> bool {
        !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
    }

    fn is_year(s: &str) -> bool {
        if s.len() != 4 || !is_all_digits(s) {
            return false;
        }
        matches!(s.parse::<u32>(), Ok(y) if (2000..=2099).contains(&y))
    }

    fn is_mm_or_dd(s: &str) -> bool {
        s.len() == 2 && is_all_digits(s)
    }

    fn is_yyyymmdd(s: &str) -> bool {
        s.len() == 8 && is_all_digits(s) && s.starts_with("20")
    }

    fn is_mmdd_code(s: &str) -> bool {
        // OpenAI 历史别名常见：gpt-4-0613 / gpt-4-1106 / gpt-4-0314 等
        if s.len() != 4 || !is_all_digits(s) {
            return false;
        }
        let mm = s[0..2].parse::<u32>().ok();
        let dd = s[2..4].parse::<u32>().ok();
        matches!((mm, dd), (Some(m), Some(d)) if (1..=12).contains(&m) && (1..=31).contains(&d))
    }

    let parts: Vec<&str> = trimmed.split('-').collect();
    for i in 0..parts.len() {
        if is_yyyymmdd(parts[i]) {
            return parts[..i].join("-");
        }
        if is_year(parts[i]) {
            if i + 2 < parts.len() && is_mm_or_dd(parts[i + 1]) && is_mm_or_dd(parts[i + 2]) {
                return parts[..i].join("-");
            }
            return parts[..i].join("-");
        }
        if is_mmdd_code(parts[i]) {
            return parts[..i].join("-");
        }
    }

    // 宽松兜底：只要包含 -202 就截断（避免奇形怪状日期）
    if let Some(idx) = lower.find("-202") {
        return trimmed[..idx].to_string();
    }

    trimmed.to_string()
}

/// 若 body 中存在 `model` 字段且需要清洗，则原地替换并返回 (from,to)
pub fn sanitize_openai_model_in_body(body: &mut Value) -> Option<(String, String)> {
    let m = body.get("model")?.as_str()?.to_string();
    let sanitized = sanitize_gpt_model_name(&m);
    if sanitized == m {
        return None;
    }
    body["model"] = Value::String(sanitized.clone());
    Some((m, sanitized))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sanitize_gpt_strips_dash_date() {
        assert_eq!(
            sanitize_gpt_model_name("gpt-5.2-2025-12-11"),
            "gpt-5.2"
        );
    }

    #[test]
    fn sanitize_gpt_strips_compact_date() {
        assert_eq!(sanitize_gpt_model_name("gpt-5.2-20251211"), "gpt-5.2");
    }

    #[test]
    fn sanitize_gpt_strips_legacy_mmdd() {
        assert_eq!(sanitize_gpt_model_name("gpt-4-0613"), "gpt-4");
        assert_eq!(sanitize_gpt_model_name("gpt-4-1106-preview"), "gpt-4");
        assert_eq!(sanitize_gpt_model_name("gpt-4-32k-0613"), "gpt-4-32k");
    }

    #[test]
    fn sanitize_gpt_strips_dash_date_for_4o() {
        assert_eq!(sanitize_gpt_model_name("gpt-4o-2024-08-06"), "gpt-4o");
        assert_eq!(
            sanitize_gpt_model_name("gpt-4o-mini-2024-08-06"),
            "gpt-4o-mini"
        );
    }

    #[test]
    fn sanitize_non_gpt_unchanged() {
        assert_eq!(
            sanitize_gpt_model_name("claude-sonnet-4-5-20250929"),
            "claude-sonnet-4-5-20250929"
        );
    }

    #[test]
    fn sanitize_body_rewrites_model() {
        let mut body = json!({"model":"gpt-5.2-2025-12-11"});
        let changed = sanitize_openai_model_in_body(&mut body);
        assert_eq!(
            changed,
            Some(("gpt-5.2-2025-12-11".to_string(), "gpt-5.2".to_string()))
        );
        assert_eq!(body["model"], "gpt-5.2");
    }
}
