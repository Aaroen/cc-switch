//! OpenAI/Codex 模型名称智能解析与写回（默认启用）
//!
//! 目标：
//! - 不同供应商对同一 OpenAI 模型使用不同别名（或仅开放子集）时，自动选取最接近且真实可用的模型 ID
//! - 在首次成功后写回 Provider 配置（避免后续重复匹配）
//!
//! 约束：
//! - 不进行“跨家族”映射（例如 gpt-* 不会映射到 deepseek/qwen 等）
//! - 仅当能确认候选存在（/v1/models）或已命中历史写回映射时才改写；否则保持原样，保证可用性

use crate::provider::Provider;
use crate::proxy::model_catalog::{detect_model_family, is_same_family, ModelFamily};
use crate::proxy::model_resolver::ModelWriteback;
use crate::proxy::model_sanitizer::sanitize_gpt_model_name;
use once_cell::sync::Lazy;
use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const MODEL_LIST_TTL: Duration = Duration::from_secs(6 * 60 * 60); // 6h
const MODEL_LIST_FAILURE_COOLDOWN: Duration = Duration::from_secs(30 * 60); // 30m
const MODELS_FETCH_TIMEOUT: Duration = Duration::from_secs(5);

pub const CODEX_ALIASES_ENV_KEY: &str = "CC_SWITCH_CODEX_MODEL_ALIASES";

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

fn normalize_token(s: &str) -> String {
    s.trim().to_lowercase()
}

fn sanitize_openai_model_name(model: &str) -> String {
    let trimmed = model.trim();
    if trimmed.is_empty() {
        return trimmed.to_string();
    }
    let lower = trimmed.to_lowercase();
    if lower.starts_with("gpt-") {
        return sanitize_gpt_model_name(trimmed);
    }
    trimmed.to_string()
}

fn extract_openai_base_url(provider: &Provider) -> Option<String> {
    // codex provider: settings_config.base_url
    provider
        .settings_config
        .get("base_url")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().trim_end_matches('/').to_string())
}

async fn fetch_models(base_url: &str, api_key: &str) -> Result<Vec<String>, String> {
    fn build_url(base_url: &str, endpoint: &str) -> String {
        let base_trimmed = base_url.trim_end_matches('/');
        let endpoint_trimmed = endpoint.trim_start_matches('/');
        let mut url = format!("{base_trimmed}/{endpoint_trimmed}");
        if url.contains("/v1/v1") {
            url = url.replace("/v1/v1", "/v1");
        }
        url
    }

    let client = Client::builder()
        .timeout(MODELS_FETCH_TIMEOUT)
        .build()
        .map_err(|e| e.to_string())?;

    let endpoints = ["/v1/models", "/models"];
    for ep in endpoints.iter() {
        let url = build_url(base_url, ep);
        let resp = client
            .get(url)
            .header("accept", "application/json")
            .header("authorization", format!("Bearer {}", api_key))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            continue;
        }
        let v = resp
            .json::<serde_json::Value>()
            .await
            .map_err(|e| e.to_string())?;
        let data = v
            .get("data")
            .and_then(|x| x.as_array())
            .ok_or_else(|| "missing data[]".to_string())?;

        let mut out: Vec<String> = Vec::new();
        for item in data.iter() {
            let Some(id) = item.get("id").and_then(|x| x.as_str()) else {
                continue;
            };
            let id = sanitize_openai_model_name(id);
            if id.trim().is_empty() {
                continue;
            }
            if out.iter().any(|x| x.eq_ignore_ascii_case(&id)) {
                continue;
            }
            out.push(id);
        }
        if !out.is_empty() {
            return Ok(out);
        }
    }

    Err("no models endpoint available".to_string())
}

fn read_alias_map(provider: &Provider) -> HashMap<String, String> {
    let Some(env) = provider.settings_config.get("env").and_then(|v| v.as_object()) else {
        return HashMap::new();
    };
    let Some(raw) = env.get(CODEX_ALIASES_ENV_KEY).and_then(|v| v.as_str()) else {
        return HashMap::new();
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(raw) else {
        return HashMap::new();
    };
    let Some(obj) = v.as_object() else {
        return HashMap::new();
    };
    let mut out: HashMap<String, String> = HashMap::new();
    for (k, val) in obj.iter() {
        let Some(s) = val.as_str() else { continue };
        let k = sanitize_openai_model_name(k);
        let s = sanitize_openai_model_name(s);
        if k.trim().is_empty() || s.trim().is_empty() {
            continue;
        }
        out.insert(normalize_token(&k), s);
    }
    out
}

fn merge_alias_map(
    mut current: HashMap<String, String>,
    request_key: &str,
    chosen: &str,
) -> String {
    current.insert(normalize_token(request_key), chosen.to_string());
    // 限制大小，避免无限增长：最多保留 64 条（按 key 排序后截断）
    let mut keys: Vec<String> = current.keys().cloned().collect();
    keys.sort();
    if keys.len() > 64 {
        let drop_n = keys.len() - 64;
        for k in keys.into_iter().take(drop_n) {
            current.remove(&k);
        }
    }
    serde_json::to_string(&current).unwrap_or_else(|_| "{}".to_string())
}

fn extract_major_minor_gpt(s: &str) -> (Option<u32>, Option<u32>) {
    // gpt-5.2 / gpt-5.1-codex / gpt-4.1 / gpt-4o -> major=4 minor=None（4o 视为同 major 特例）
    let lower = s.to_lowercase();
    let Some(rest) = lower.strip_prefix("gpt-") else {
        return (None, None);
    };
    let head = rest
        .split(|c: char| c == '-' || c == '_' || c == '/')
        .next()
        .unwrap_or(rest);
    if head.starts_with("4o") {
        return (Some(4), None);
    }
    let mut parts = head.split('.');
    let major = parts.next().and_then(|x| x.parse::<u32>().ok());
    let minor = parts.next().and_then(|x| {
        x.chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse::<u32>()
            .ok()
    });
    (major, minor)
}

fn score_candidate(request: &str, candidate: &str) -> i32 {
    let req = normalize_token(request);
    let cand = normalize_token(candidate);
    if req == cand {
        return 10_000;
    }

    if !is_same_family(request, candidate) {
        return -100_000;
    }

    let mut score = 0i32;

    let req_is_gpt = req.starts_with("gpt-");
    let cand_is_gpt = cand.starts_with("gpt-");
    if req_is_gpt != cand_is_gpt {
        score -= 10_000;
    }

    let (rmj, rmn) = extract_major_minor_gpt(&req);
    let (cmj, cmn) = extract_major_minor_gpt(&cand);
    match (rmj, cmj) {
        (Some(a), Some(b)) if a == b => score += 120,
        (Some(_), Some(_)) => score -= 200,
        _ => {}
    }
    match (rmn, cmn) {
        (Some(a), Some(b)) if a == b => score += 60,
        (Some(_), Some(_)) => score -= 30,
        _ => {}
    }

    let req_codex = req.contains("codex");
    let cand_codex = cand.contains("codex");
    if req_codex == cand_codex {
        score += 10;
    } else if req_codex && !cand_codex {
        score -= 15;
    }

    let req_prefers_low = req.contains("-low") || req.ends_with("-low");
    let cand_is_low = cand.contains("-low") || cand.ends_with("-low");
    if req_prefers_low == cand_is_low {
        score += 3;
    } else if req_prefers_low && !cand_is_low {
        score -= 3;
    }

    let req_prefers_mini = req.contains("mini");
    let cand_is_mini = cand.contains("mini");
    if req_prefers_mini == cand_is_mini {
        score += 2;
    } else if req_prefers_mini && !cand_is_mini {
        score -= 2;
    }

    score -= (candidate.len() as i32).min(60) / 6;
    score
}

fn choose_best_model(request_model: &str, candidates: &[String]) -> Option<String> {
    let req = sanitize_openai_model_name(request_model);
    let req_family = detect_model_family(&req);

    let mut best: Option<(i32, String)> = None;
    for c in candidates.iter() {
        if req_family != ModelFamily::Other && detect_model_family(c) != req_family {
            continue;
        }
        let s = score_candidate(&req, c);
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

pub async fn resolve_openai_model_in_body(
    _client: &Client,
    provider: &Provider,
    api_key: &str,
    original_request_model: &str,
    mut body: Value,
) -> (Value, Option<ModelWriteback>) {
    let request_model = sanitize_openai_model_name(original_request_model);
    if request_model.trim().is_empty() || request_model == "unknown" {
        return (body, None);
    }

    if detect_model_family(&request_model) != ModelFamily::OpenAi {
        return (body, None);
    }

    // 0) 已写回别名优先（无网络）
    let aliases = read_alias_map(provider);
    let request_key = normalize_token(&request_model);
    if let Some(mapped) = aliases.get(&request_key) {
        if is_same_family(&request_model, mapped) && normalize_token(mapped) != request_key {
            body["model"] = serde_json::json!(mapped);
            return (body, None);
        }
    }

    let Some(base_url) = extract_openai_base_url(provider) else {
        return (body, None);
    };
    let key = ModelListKey {
        provider_id: provider.id.clone(),
        base_url: base_url.clone(),
    };

    // 1) 缓存命中
    if let Ok(cache) = MODEL_LIST_CACHE.lock() {
        if let Some(v) = cache.get(&key) {
            if v.fetched_at.elapsed() <= MODEL_LIST_TTL {
                let models = &v.models;
                return resolve_from_model_list(&request_model, models, aliases, body);
            }
        }
    }

    // 2) 失败冷却
    if let Ok(failures) = MODEL_LIST_FAILURES.lock() {
        if let Some(t) = failures.get(&key) {
            if t.elapsed() <= MODEL_LIST_FAILURE_COOLDOWN {
                return (body, None);
            }
        }
    }

    // 3) 拉取
    match fetch_models(&base_url, api_key).await {
        Ok(list) => {
            if let Ok(mut cache) = MODEL_LIST_CACHE.lock() {
                cache.insert(
                    key.clone(),
                    CachedModelList {
                        fetched_at: Instant::now(),
                        models: list.clone(),
                    },
                );
            }
            if let Ok(mut failures) = MODEL_LIST_FAILURES.lock() {
                failures.remove(&key);
            }
            resolve_from_model_list(&request_model, &list, aliases, body)
        }
        Err(e) => {
            if let Ok(mut failures) = MODEL_LIST_FAILURES.lock() {
                failures.insert(key.clone(), Instant::now());
            }
            log::debug!(
                "[OpenAIModelResolver] /v1/models 拉取失败 provider={} base_url={} err={}",
                provider.id,
                base_url,
                e
            );
            (body, None)
        }
    }
}

fn resolve_from_model_list(
    request_model: &str,
    models: &[String],
    aliases: HashMap<String, String>,
    mut body: Value,
) -> (Value, Option<ModelWriteback>) {
    // 2) 若上游本来就支持该 model，则优先不改
    if models
        .iter()
        .any(|m| normalize_token(m) == normalize_token(request_model))
    {
        return (body, None);
    }

    let chosen = choose_best_model(request_model, models);
    let Some(chosen) = chosen else {
        return (body, None);
    };
    if normalize_token(&chosen) == normalize_token(request_model) {
        return (body, None);
    }

    body["model"] = serde_json::json!(chosen.clone());

    let new_aliases_json = merge_alias_map(aliases, request_model, &chosen);
    let wb = ModelWriteback {
        env_key: CODEX_ALIASES_ENV_KEY,
        value: new_aliases_json,
        from_model: request_model.to_string(),
        to_model: chosen,
    };
    (body, Some(wb))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn provider_with_base(base: &str) -> Provider {
        Provider {
            id: "p1".to_string(),
            name: "P1".to_string(),
            settings_config: json!({
                "base_url": base,
                "env": {
                    "OPENAI_API_KEY": "sk-test"
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
    fn score_prefers_same_major_minor() {
        let cands = vec![
            "gpt-5.1".to_string(),
            "gpt-5.2-codex".to_string(),
            "gpt-4o".to_string(),
        ];
        let best = choose_best_model("gpt-5.2", &cands).unwrap();
        assert_eq!(best, "gpt-5.2-codex");
    }

    #[test]
    fn alias_map_merge_is_bounded() {
        let p = provider_with_base("https://example.com");
        let mut map = read_alias_map(&p);
        for i in 0..100 {
            map.insert(format!("gpt-5.2-{i}"), "gpt-5.2".to_string());
        }
        let json = merge_alias_map(map, "gpt-5.2", "gpt-5.2-codex");
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(v.as_object().unwrap().len() <= 64);
    }

    #[test]
    fn sanitize_openai_model_strips_legacy_mmdd() {
        assert_eq!(sanitize_openai_model_name("gpt-4-0613"), "gpt-4");
    }
}
