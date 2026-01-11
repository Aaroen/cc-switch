//! Claude 模型名称智能解析与写回
//!
//! 目标：当不同服务商对同一 Claude 模型使用不同命名时，自动匹配到该服务商实际支持的模型 ID，
//! 并在首次成功后写回 Provider 配置，避免后续重复匹配。

use crate::provider::Provider;
use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::Client;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::{Duration, Instant};

const MODELS_ENDPOINT: &str = "/v1/models";
const PYTHON_PROXY_BASE: &str = "http://127.0.0.1:15722";

const MODEL_LIST_TTL: Duration = Duration::from_secs(6 * 60 * 60); // 6h
const MODEL_LIST_FAILURE_COOLDOWN: Duration = Duration::from_secs(30 * 60); // 30m
const MODELS_FETCH_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
pub struct ModelWriteback {
    pub env_key: &'static str,
    pub value: String,
    pub from_model: String,
    pub to_model: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
struct ModelListKey {
    provider_id: String,
    base_url: String,
}

#[derive(Debug, Clone)]
struct CachedModelList {
    fetched_at: Instant,
    models: Vec<String>,
}

static MODEL_LIST_CACHE: Lazy<Mutex<HashMap<ModelListKey, CachedModelList>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static MODEL_LIST_FAILURES: Lazy<Mutex<HashMap<ModelListKey, Instant>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Family {
    Haiku,
    Sonnet,
    Opus,
}

#[derive(Debug, Clone, Default)]
struct ModelFeatures {
    family: Option<Family>,
    major: Option<u32>,
    minor: Option<u32>,
    thinking: bool,
    date: Option<String>,
}

fn normalize_token(s: &str) -> String {
    s.trim().to_lowercase()
}

fn extract_anthropic_base_url(provider: &Provider) -> Option<String> {
    provider
        .settings_config
        .get("env")
        .and_then(|env| env.get("ANTHROPIC_BASE_URL"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim().trim_end_matches('/').to_string())
}

fn detect_family(s: &str) -> Option<Family> {
    let sl = s.to_lowercase();
    if sl.contains("haiku") {
        return Some(Family::Haiku);
    }
    if sl.contains("sonnet") {
        return Some(Family::Sonnet);
    }
    if sl.contains("opus") {
        return Some(Family::Opus);
    }
    None
}

fn detect_thinking(s: &str) -> bool {
    let sl = s.to_lowercase();
    sl.contains("thinking") || sl.contains("reasoning") || sl.contains(":extended")
}

fn extract_major_minor(s: &str) -> (Option<u32>, Option<u32>) {
    static RE_DOT: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(?i)(\d+)\.(\d+)").expect("regex"));
    static RE_DASH: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(?i)(\d+)[-_](\d+)").expect("regex"));

    // 优先匹配 4.5 这种形式，其次 4-5 / 4_5
    if let Some(c) = RE_DOT.captures(s) {
        let major = c.get(1).and_then(|m| m.as_str().parse::<u32>().ok());
        let minor = c.get(2).and_then(|m| m.as_str().parse::<u32>().ok());
        return (major, minor);
    }
    if let Some(c) = RE_DASH.captures(s) {
        let major = c.get(1).and_then(|m| m.as_str().parse::<u32>().ok());
        let minor = c.get(2).and_then(|m| m.as_str().parse::<u32>().ok());
        return (major, minor);
    }
    (None, None)
}

fn extract_date_yyyymmdd(s: &str) -> Option<String> {
    static RE_DATE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)\b(\d{8})\b").expect("regex"));
    RE_DATE
        .captures(s)
        .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
}

fn parse_features(model: &str, thinking_from_body: bool) -> ModelFeatures {
    let mut f = ModelFeatures::default();
    f.family = detect_family(model);
    let (major, minor) = extract_major_minor(model);
    f.major = major;
    f.minor = minor;
    f.thinking = thinking_from_body || detect_thinking(model);
    f.date = extract_date_yyyymmdd(model);
    f
}

fn score_candidate(request: &ModelFeatures, candidate: &ModelFeatures) -> i32 {
    let mut score = 0i32;

    // family（最高优先级）
    if let Some(req_f) = request.family {
        match candidate.family {
            Some(c_f) if c_f == req_f => score += 100,
            Some(_) => score -= 1000,
            None => score -= 50,
        }
    }

    // major/minor（第二优先级）
    if let (Some(req_major), Some(req_minor)) = (request.major, request.minor) {
        match (candidate.major, candidate.minor) {
            (Some(c_major), Some(c_minor)) if c_major == req_major && c_minor == req_minor => {
                score += 30
            }
            (Some(c_major), _) if c_major == req_major => score += 10,
            _ => score -= 50,
        }
    } else if let Some(req_major) = request.major {
        if candidate.major == Some(req_major) {
            score += 10;
        }
    }

    // thinking（第三优先级）
    if request.thinking {
        score += if candidate.thinking { 5 } else { -5 };
    } else if candidate.thinking {
        score -= 2;
    }

    // date：不在优先级内，仅作为轻微 tie-breaker
    if let (Some(req_date), Some(c_date)) = (request.date.as_deref(), candidate.date.as_deref()) {
        if req_date == c_date {
            score += 2;
        }
    }

    score
}

fn choose_best_model_with_avoid(
    request_model: &str,
    thinking_from_body: bool,
    candidates: &[String],
    avoid_norm: &HashSet<String>,
) -> Option<String> {
    let req_norm = normalize_token(request_model);
    let request = parse_features(request_model, thinking_from_body);
    let request_is_claude =
        crate::proxy::model_catalog::detect_model_family(request_model)
            == crate::proxy::model_catalog::ModelFamily::Claude;

    // 若存在至少一个同 family 候选，则强制在同 family 内选择（“优先 family”，但允许无同 family 时降级）
    let has_family_match = request.family.is_some()
        && candidates.iter().any(|c| {
            if avoid_norm.contains(&normalize_token(c)) {
                return false;
            }
            if request_is_claude && !normalize_token(c).contains("claude") {
                return false;
            }
            parse_features(c, false).family == request.family
        });

    // 1) 精确命中：如果上游本来就支持该 model，则优先不改（除非被显式 avoid）
    for c in candidates {
        let cn = normalize_token(c);
        if avoid_norm.contains(&cn) {
            continue;
        }
        // 家族锚定：Claude 请求严禁映射到非 Claude（例如 GLM/GPT）
        if request_is_claude && !cn.contains("claude") {
            continue;
        }
        if cn == req_norm {
            return Some(c.clone());
        }
    }

    // 2) 打分选择（同分时用稳定的 tie-breaker，避免“分数接近就放弃”导致 model_not_found）
    let mut best: Option<(i32, String)> = None;

    for c in candidates {
        let cn = normalize_token(c);
        if avoid_norm.contains(&cn) {
            continue;
        }
        // 家族锚定：Claude 请求严禁映射到非 Claude（例如 GLM/GPT）
        if request_is_claude && !cn.contains("claude") {
            continue;
        }

        let cf = parse_features(c, false);
        if has_family_match && cf.family != request.family {
            continue;
        }

        let s = score_candidate(&request, &cf);
        match &best {
            None => best = Some((s, c.clone())),
            Some((best_s, best_c)) => {
                let replace = s > *best_s
                    || (s == *best_s
                        && (c.len() < best_c.len()
                            || (c.len() == best_c.len() && c < best_c)));
                if replace {
                    best = Some((s, c.clone()));
                }
            }
        }
    }

    best.map(|(_, m)| m)
}

#[cfg(test)]
fn choose_best_model(
    request_model: &str,
    thinking_from_body: bool,
    candidates: &[String],
) -> Option<String> {
    choose_best_model_with_avoid(request_model, thinking_from_body, candidates, &HashSet::new())
}

fn determine_writeback_key(original_request_model: &str, thinking_from_body: bool) -> &'static str {
    if thinking_from_body || detect_thinking(original_request_model) {
        return "ANTHROPIC_REASONING_MODEL";
    }
    match detect_family(original_request_model) {
        Some(Family::Haiku) => "ANTHROPIC_DEFAULT_HAIKU_MODEL",
        Some(Family::Sonnet) => "ANTHROPIC_DEFAULT_SONNET_MODEL",
        Some(Family::Opus) => "ANTHROPIC_DEFAULT_OPUS_MODEL",
        None => "ANTHROPIC_MODEL",
    }
}

fn read_env_model(provider: &Provider, key: &str) -> Option<String> {
    provider
        .settings_config
        .get("env")
        .and_then(|env| env.get(key))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn is_model_in_list(model: &str, candidates: &[String]) -> bool {
    let m = normalize_token(model);
    candidates.iter().any(|c| normalize_token(c) == m)
}

async fn fetch_models_via_python_proxy(
    client: &Client,
    base_url: &str,
    api_key: &str,
) -> Result<Vec<String>, String> {
    let url = format!("{PYTHON_PROXY_BASE}{MODELS_ENDPOINT}");

    async fn do_fetch(
        client: &Client,
        url: &str,
        base_url: &str,
        api_key_value: &str,
    ) -> Result<Value, String> {
        let resp = client
            .get(url)
            .timeout(MODELS_FETCH_TIMEOUT)
            // Python 代理会把 X-API-Key 注入为 x-api-key 或 authorization（取决于 value 前缀）
            .header("X-API-Key", api_key_value)
            .header("x-target-base-url", base_url)
            // 一些 Anthropic 兼容网关会要求该头存在；对于 OpenAI 风格网关一般会忽略
            .header("anthropic-version", "2023-06-01")
            .send()
            .await
            .map_err(|e| format!("请求 /v1/models 失败: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("请求 /v1/models 返回非 2xx: {status} body={}", text));
        }

        resp.json::<Value>()
            .await
            .map_err(|e| format!("解析 /v1/models JSON 失败: {e}"))
    }

    // 兼容：部分 NewAPI/聚合服务对 /v1/messages 接受 x-api-key，但 /v1/models 只接受 Authorization: Bearer。
    // Python 代理的规则：当传入的 X-API-Key value 以 "Bearer " 开头时，会注入 authorization 头。
    let v = match do_fetch(client, &url, base_url, api_key).await {
        Ok(v) => v,
        Err(e1) => {
            // 对 Anthropic 官方 key（sk-ant-*）不再尝试 Bearer；避免误用导致额外失败日志
            if api_key.trim_start().starts_with("sk-ant-") || api_key.trim_start().starts_with("Bearer ") {
                return Err(e1);
            }
            let bearer = format!("Bearer {}", api_key.trim());
            match do_fetch(client, &url, base_url, &bearer).await {
                Ok(v) => v,
                Err(e2) => {
                    return Err(format!("{e1}; fallback_bearer={e2}"));
                }
            }
        }
    };

    // OpenAI 兼容：{ data: [{ id: "..." }, ...] }
    let mut out = Vec::new();
    if let Some(arr) = v.get("data").and_then(|d| d.as_array()) {
        for item in arr {
            if let Some(id) = item.get("id").and_then(|x| x.as_str()) {
                if !id.trim().is_empty() {
                    out.push(id.trim().to_string());
                }
            }
        }
    }

    // 兜底：一些服务可能返回 { models: [...] } 或 { data: ["id", ...] }
    if out.is_empty() {
        if let Some(arr) = v.get("models").and_then(|d| d.as_array()) {
            for item in arr {
                if let Some(id) = item.get("id").and_then(|x| x.as_str()) {
                    if !id.trim().is_empty() {
                        out.push(id.trim().to_string());
                    }
                } else if let Some(s) = item.as_str() {
                    if !s.trim().is_empty() {
                        out.push(s.trim().to_string());
                    }
                }
            }
        }
    }
    if out.is_empty() {
        if let Some(arr) = v.get("data").and_then(|d| d.as_array()) {
            for item in arr {
                if let Some(s) = item.as_str() {
                    if !s.trim().is_empty() {
                        out.push(s.trim().to_string());
                    }
                }
            }
        }
    }

    // 去重（保持顺序）
    let mut seen = HashSet::new();
    out.retain(|m| seen.insert(normalize_token(m)));

    if out.is_empty() {
        return Err("上游 /v1/models 返回为空或不兼容".to_string());
    }

    Ok(out)
}

async fn get_or_fetch_model_list(
    client: &Client,
    key: &ModelListKey,
    api_key: &str,
) -> Option<Vec<String>> {
    // 1) TTL 缓存命中
    {
        let cache = MODEL_LIST_CACHE.lock().ok()?;
        if let Some(v) = cache.get(key) {
            if v.fetched_at.elapsed() <= MODEL_LIST_TTL {
                return Some(v.models.clone());
            }
        }
    }

    // 2) 失败冷却
    {
        let failures = MODEL_LIST_FAILURES.lock().ok()?;
        if let Some(t) = failures.get(key) {
            if t.elapsed() <= MODEL_LIST_FAILURE_COOLDOWN {
                return None;
            }
        }
    }

    // 3) 拉取
    match fetch_models_via_python_proxy(client, &key.base_url, api_key).await {
        Ok(models) => {
            if let Ok(mut cache) = MODEL_LIST_CACHE.lock() {
                cache.insert(
                    key.clone(),
                    CachedModelList {
                        fetched_at: Instant::now(),
                        models: models.clone(),
                    },
                );
            }
            if let Ok(mut failures) = MODEL_LIST_FAILURES.lock() {
                failures.remove(key);
            }
            Some(models)
        }
        Err(e) => {
            log::debug!(
                "[ModelResolver] /v1/models 拉取失败 provider={} base_url={} err={}",
                key.provider_id,
                key.base_url,
                e
            );
            if let Ok(mut failures) = MODEL_LIST_FAILURES.lock() {
                failures.insert(key.clone(), Instant::now());
            }
            None
        }
    }
}

/// Claude 模型名称智能解析（默认启用）
///
/// - 优先使用 provider 当前配置的 model（若其本来就在 /v1/models 列表内）
/// - 否则基于请求模型的 family/major-minor/thinking 优先级匹配
/// - 若选出更合适的模型，则返回写回建议（只在请求成功后写回）
pub async fn resolve_claude_model_in_body(
    client: &Client,
    provider: &Provider,
    api_key: &str,
    original_request_model: &str,
    body: Value,
) -> (Value, Option<ModelWriteback>) {
    resolve_claude_model_in_body_with_avoid(
        client,
        provider,
        api_key,
        original_request_model,
        body,
        &[],
    )
    .await
}

pub async fn resolve_claude_model_in_body_with_avoid(
    client: &Client,
    provider: &Provider,
    api_key: &str,
    original_request_model: &str,
    mut body: Value,
    avoid_models: &[&str],
) -> (Value, Option<ModelWriteback>) {
    // 仅对“看起来像 Claude 模型”的请求启用解析，避免误处理其它模型体系
    let request_features = parse_features(original_request_model, false);
    let is_claudeish = request_features.family.is_some()
        || (original_request_model.to_lowercase().contains("claude")
            && (request_features.major.is_some() || request_features.minor.is_some()));
    if !is_claudeish {
        return (body, None);
    }

    let thinking_from_body = crate::proxy::model_mapper::has_thinking_enabled(&body);
    let avoid_norm: HashSet<String> = avoid_models.iter().map(|s| normalize_token(s)).collect();

    let Some(base_url) = extract_anthropic_base_url(provider) else {
        return (body, None);
    };
    let key = ModelListKey {
        provider_id: provider.id.clone(),
        base_url,
    };

    let Some(models) = get_or_fetch_model_list(client, &key, api_key).await else {
        return (body, None);
    };

    let current_model = body
        .get("model")
        .and_then(|m| m.as_str())
        .map(|s| s.to_string());
    let Some(current_model) = current_model else {
        return (body, None);
    };

    // 如果当前 model 已在上游列表内，则直接使用（无需智能匹配/写回），除非显式要求避开该 model
    if is_model_in_list(&current_model, &models)
        && !avoid_norm.contains(&normalize_token(&current_model))
    {
        return (body, None);
    }

    // 基于“原始请求模型”做智能匹配（保留 family/版本信息）
    let chosen = choose_best_model_with_avoid(
        original_request_model,
        thinking_from_body,
        &models,
        &avoid_norm,
    );
    let Some(chosen) = chosen else {
        return (body, None);
    };

    if normalize_token(&chosen) == normalize_token(&current_model) {
        return (body, None);
    }

    // 生成写回建议：仅当写回目标 key 不存在或与目标不同才写回
    let env_key = determine_writeback_key(original_request_model, thinking_from_body);
    let existing = read_env_model(provider, env_key);
    let needs_writeback = existing
        .as_deref()
        .map(|v| normalize_token(v) != normalize_token(&chosen))
        .unwrap_or(true);

    log::debug!(
        "[ModelResolver] provider={} model {} → {} (writeback_key={} {})",
        provider.id,
        current_model,
        chosen,
        env_key,
        if needs_writeback { "pending" } else { "skip" }
    );

    body["model"] = serde_json::json!(chosen.clone());

    let writeback = if needs_writeback {
        Some(ModelWriteback {
            env_key,
            value: chosen.clone(),
            from_model: current_model,
            to_model: chosen,
        })
    } else {
        None
    };

    (body, writeback)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_features_handles_examples() {
        let f = parse_features("claude-sonnet-4-5-20250929", false);
        assert_eq!(f.family, Some(Family::Sonnet));
        assert_eq!(f.major, Some(4));
        assert_eq!(f.minor, Some(5));
        assert_eq!(f.date.as_deref(), Some("20250929"));

        let f = parse_features("cursor2-claude-4.5-sonnet", false);
        assert_eq!(f.family, Some(Family::Sonnet));
        assert_eq!(f.major, Some(4));
        assert_eq!(f.minor, Some(5));

        let f = parse_features("claude-sonnet-4-5-thinking", false);
        assert!(f.thinking);
    }

    #[test]
    fn choose_best_prefers_family_then_version_then_thinking() {
        let candidates = vec![
            "claude-haiku-4-5".to_string(),
            "cursor2-claude-4.5-sonnet".to_string(),
            "claude-sonnet-4-5-thinking".to_string(),
        ];

        // 非 thinking：应选择 sonnet 的 4.5（family/版本匹配最优）
        let chosen = choose_best_model("claude-sonnet-4-5-20250929", false, &candidates).unwrap();
        assert_eq!(chosen, "cursor2-claude-4.5-sonnet");

        // thinking：优先 thinking 版本（family 同，版本同，thinking 优先）
        let chosen = choose_best_model("claude-sonnet-4-5-20250929", true, &candidates).unwrap();
        assert_eq!(chosen, "claude-sonnet-4-5-thinking");
    }
}
