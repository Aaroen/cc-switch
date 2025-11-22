/// Deep link import functionality for CC Switch
///
/// This module implements the ccswitch:// protocol for importing provider configurations
/// via deep links. See docs/ccswitch-deeplink-design.md for detailed design.
use crate::error::AppError;
use crate::provider::Provider;
use crate::services::ProviderService;
use crate::store::AppState;
use crate::AppType;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use url::Url;

/// Deep link import request model
/// Represents a parsed ccswitch:// URL ready for processing
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeepLinkImportRequest {
    /// Protocol version (e.g., "v1")
    pub version: String,
    /// Resource type to import (e.g., "provider")
    pub resource: String,
    /// Target application (claude/codex/gemini)
    pub app: String,
    /// Provider name
    pub name: String,
    /// Provider homepage URL
    pub homepage: String,
    /// API endpoint/base URL
    pub endpoint: String,
    /// API key
    pub api_key: String,
    /// Optional model name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Optional notes/description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    /// Optional Haiku model (Claude only, v3.7.1+)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub haiku_model: Option<String>,
    /// Optional Sonnet model (Claude only, v3.7.1+)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sonnet_model: Option<String>,
    /// Optional Opus model (Claude only, v3.7.1+)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opus_model: Option<String>,
    /// Optional Base64 encoded config content (v3.8+)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<String>,
    /// Optional config format (json/toml, v3.8+)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_format: Option<String>,
    /// Optional remote config URL (v3.8+)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_url: Option<String>,
}

/// Parse a ccswitch:// URL into a DeepLinkImportRequest
///
/// Expected format:
/// ccswitch://v1/import?resource=provider&app=claude&name=...&homepage=...&endpoint=...&apiKey=...
pub fn parse_deeplink_url(url_str: &str) -> Result<DeepLinkImportRequest, AppError> {
    // Parse URL
    let url = Url::parse(url_str)
        .map_err(|e| AppError::InvalidInput(format!("Invalid deep link URL: {e}")))?;

    // Validate scheme
    let scheme = url.scheme();
    if scheme != "ccswitch" {
        return Err(AppError::InvalidInput(format!(
            "Invalid scheme: expected 'ccswitch', got '{scheme}'"
        )));
    }

    // Extract version from host
    let version = url
        .host_str()
        .ok_or_else(|| AppError::InvalidInput("Missing version in URL host".to_string()))?
        .to_string();

    // Validate version
    if version != "v1" {
        return Err(AppError::InvalidInput(format!(
            "Unsupported protocol version: {version}"
        )));
    }

    // Extract path (should be "/import")
    let path = url.path();
    if path != "/import" {
        return Err(AppError::InvalidInput(format!(
            "Invalid path: expected '/import', got '{path}'"
        )));
    }

    // Parse query parameters
    let params: HashMap<String, String> = url.query_pairs().into_owned().collect();

    // Extract and validate resource type
    let resource = params
        .get("resource")
        .ok_or_else(|| AppError::InvalidInput("Missing 'resource' parameter".to_string()))?
        .clone();

    if resource != "provider" {
        return Err(AppError::InvalidInput(format!(
            "Unsupported resource type: {resource}"
        )));
    }

    // Extract required fields
    let app = params
        .get("app")
        .ok_or_else(|| AppError::InvalidInput("Missing 'app' parameter".to_string()))?
        .clone();

    // Validate app type
    if app != "claude" && app != "codex" && app != "gemini" {
        return Err(AppError::InvalidInput(format!(
            "Invalid app type: must be 'claude', 'codex', or 'gemini', got '{app}'"
        )));
    }

    let name = params
        .get("name")
        .ok_or_else(|| AppError::InvalidInput("Missing 'name' parameter".to_string()))?
        .clone();

    // Make these optional for config file auto-fill (v3.8+)
    let homepage = params.get("homepage").cloned().unwrap_or_default();
    let endpoint = params.get("endpoint").cloned().unwrap_or_default();
    let api_key = params.get("apiKey").cloned().unwrap_or_default();

    // Validate URLs only if provided
    if !homepage.is_empty() {
        validate_url(&homepage, "homepage")?;
    }
    if !endpoint.is_empty() {
        validate_url(&endpoint, "endpoint")?;
    }

    // Extract optional fields
    let model = params.get("model").cloned();
    let notes = params.get("notes").cloned();

    // Extract Claude-specific optional model fields (v3.7.1+)
    let haiku_model = params.get("haikuModel").cloned();
    let sonnet_model = params.get("sonnetModel").cloned();
    let opus_model = params.get("opusModel").cloned();

    // Extract optional config fields (v3.8+)
    let config = params.get("config").cloned();
    let config_format = params.get("configFormat").cloned();
    let config_url = params.get("configUrl").cloned();

    Ok(DeepLinkImportRequest {
        version,
        resource,
        app,
        name,
        homepage,
        endpoint,
        api_key,
        model,
        notes,
        haiku_model,
        sonnet_model,
        opus_model,
        config,
        config_format,
        config_url,
    })
}

/// Validate that a string is a valid HTTP(S) URL
fn validate_url(url_str: &str, field_name: &str) -> Result<(), AppError> {
    let url = Url::parse(url_str)
        .map_err(|e| AppError::InvalidInput(format!("Invalid URL for '{field_name}': {e}")))?;

    let scheme = url.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(AppError::InvalidInput(format!(
            "Invalid URL scheme for '{field_name}': must be http or https, got '{scheme}'"
        )));
    }

    Ok(())
}

/// Import a provider from a deep link request
///
/// This function:
/// 1. Validates the request
/// 2. Merges config file if provided (v3.8+)
/// 3. Converts it to a Provider structure
/// 4. Delegates to ProviderService for actual import
pub fn import_provider_from_deeplink(
    state: &AppState,
    request: DeepLinkImportRequest,
) -> Result<String, AppError> {
    // Step 1: Merge config file if provided (v3.8+)
    let merged_request = parse_and_merge_config(&request)?;

    // Step 2: Validate required fields after merge
    if merged_request.api_key.is_empty() {
        return Err(AppError::InvalidInput(
            "API key is required (either in URL or config file)".to_string(),
        ));
    }
    if merged_request.endpoint.is_empty() {
        return Err(AppError::InvalidInput(
            "Endpoint is required (either in URL or config file)".to_string(),
        ));
    }
    if merged_request.homepage.is_empty() {
        return Err(AppError::InvalidInput(
            "Homepage is required (either in URL or config file)".to_string(),
        ));
    }

    // Parse app type
    let app_type = AppType::from_str(&merged_request.app)
        .map_err(|_| AppError::InvalidInput(format!("Invalid app type: {}", merged_request.app)))?;

    // Build provider configuration based on app type
    let mut provider = build_provider_from_request(&app_type, &merged_request)?;

    // Generate a unique ID for the provider using timestamp + sanitized name
    // This is similar to how frontend generates IDs
    let timestamp = chrono::Utc::now().timestamp_millis();
    let sanitized_name = merged_request
        .name
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect::<String>()
        .to_lowercase();
    provider.id = format!("{sanitized_name}-{timestamp}");

    let provider_id = provider.id.clone();

    // Use ProviderService to add the provider
    ProviderService::add(state, app_type, provider)?;

    Ok(provider_id)
}

/// Build a Provider structure from a deep link request
fn build_provider_from_request(
    app_type: &AppType,
    request: &DeepLinkImportRequest,
) -> Result<Provider, AppError> {
    use serde_json::json;

    let settings_config = match app_type {
        AppType::Claude => {
            // Claude configuration structure
            let mut env = serde_json::Map::new();
            env.insert("ANTHROPIC_AUTH_TOKEN".to_string(), json!(request.api_key));
            env.insert("ANTHROPIC_BASE_URL".to_string(), json!(request.endpoint));

            // Add default model if provided
            if let Some(model) = &request.model {
                env.insert("ANTHROPIC_MODEL".to_string(), json!(model));
            }

            // Add Claude-specific model fields (v3.7.1+)
            if let Some(haiku_model) = &request.haiku_model {
                env.insert(
                    "ANTHROPIC_DEFAULT_HAIKU_MODEL".to_string(),
                    json!(haiku_model),
                );
            }
            if let Some(sonnet_model) = &request.sonnet_model {
                env.insert(
                    "ANTHROPIC_DEFAULT_SONNET_MODEL".to_string(),
                    json!(sonnet_model),
                );
            }
            if let Some(opus_model) = &request.opus_model {
                env.insert(
                    "ANTHROPIC_DEFAULT_OPUS_MODEL".to_string(),
                    json!(opus_model),
                );
            }

            json!({ "env": env })
        }
        AppType::Codex => {
            // Codex configuration structure
            // For Codex, we store auth.json (JSON) and config.toml (TOML string) in settings_config。
            //
            // 这里尽量与前端 `getCodexCustomTemplate` 的默认模板保持一致，
            // 再根据深链接参数注入 base_url / model，避免出现“只有 base_url 行”的极简配置，
            // 让通过 UI 新建和通过深链接导入的 Codex 自定义供应商行为一致。

            // 1. 生成一个适合作为 model_provider 名的安全标识
            //    规则尽量与前端 codexProviderPresets.generateThirdPartyConfig 保持一致：
            //    - 转小写
            //    - 非 [a-z0-9_] 统一替换为下划线
            //    - 去掉首尾下划线
            //    - 若结果为空，则使用 "custom"
            let clean_provider_name = {
                let raw: String = request.name.chars().filter(|c| !c.is_control()).collect();
                let lower = raw.to_lowercase();
                let mut key: String = lower
                    .chars()
                    .map(|c| match c {
                        'a'..='z' | '0'..='9' | '_' => c,
                        _ => '_',
                    })
                    .collect();

                // 去掉首尾下划线
                while key.starts_with('_') {
                    key.remove(0);
                }
                while key.ends_with('_') {
                    key.pop();
                }

                if key.is_empty() {
                    "custom".to_string()
                } else {
                    key
                }
            };

            // 2. 模型名称：优先使用 deeplink 中的 model，否则退回到 Codex 默认模型
            let model_name = request
                .model
                .as_deref()
                .unwrap_or("gpt-5-codex")
                .to_string();

            // 3. 端点：与 UI 中 Base URL 处理方式保持一致，去掉结尾多余的斜杠
            let endpoint = request.endpoint.trim().trim_end_matches('/').to_string();

            // 4. 组装 config.toml 内容
            // 使用 Rust 1.58+ 的内联格式化语法，避免 clippy::uninlined_format_args 警告
            let config_toml = format!(
                r#"model_provider = "{clean_provider_name}"
model = "{model_name}"
model_reasoning_effort = "high"
disable_response_storage = true

[model_providers.{clean_provider_name}]
name = "{clean_provider_name}"
base_url = "{endpoint}"
wire_api = "responses"
requires_openai_auth = true
"#
            );

            json!({
                "auth": {
                    "OPENAI_API_KEY": request.api_key,
                },
                "config": config_toml
            })
        }
        AppType::Gemini => {
            // Gemini configuration structure (.env format)
            let mut env = serde_json::Map::new();
            env.insert("GEMINI_API_KEY".to_string(), json!(request.api_key));
            env.insert(
                "GOOGLE_GEMINI_BASE_URL".to_string(),
                json!(request.endpoint),
            );

            // Add model if provided
            if let Some(model) = &request.model {
                env.insert("GEMINI_MODEL".to_string(), json!(model));
            }

            json!({ "env": env })
        }
    };

    let provider = Provider {
        id: String::new(), // Will be generated by ProviderService
        name: request.name.clone(),
        settings_config,
        website_url: Some(request.homepage.clone()),
        category: None,
        created_at: None,
        sort_index: None,
        notes: request.notes.clone(),
        meta: None,
        icon: None,
        icon_color: None,
    };

    Ok(provider)
}

/// Parse and merge configuration from Base64 encoded config or remote URL
///
/// Priority: URL params > inline config > remote config
pub fn parse_and_merge_config(
    request: &DeepLinkImportRequest,
) -> Result<DeepLinkImportRequest, AppError> {
    use base64::prelude::*;

    // If no config provided, return original request
    if request.config.is_none() && request.config_url.is_none() {
        return Ok(request.clone());
    }

    // Step 1: Get config content
    let config_content = if let Some(config_b64) = &request.config {
        // Decode Base64 inline config
        let decoded = BASE64_STANDARD
            .decode(config_b64)
            .map_err(|e| AppError::InvalidInput(format!("Invalid Base64 encoding: {e}")))?;
        String::from_utf8(decoded)
            .map_err(|e| AppError::InvalidInput(format!("Invalid UTF-8 in config: {e}")))?
    } else if let Some(_config_url) = &request.config_url {
        // Fetch remote config (TODO: implement remote fetching in next phase)
        return Err(AppError::InvalidInput(
            "Remote config URL is not yet supported. Use inline config instead.".to_string(),
        ));
    } else {
        return Ok(request.clone());
    };

    // Step 2: Parse config based on format
    let format = request.config_format.as_deref().unwrap_or("json");
    let config_value: serde_json::Value = match format {
        "json" => serde_json::from_str(&config_content)
            .map_err(|e| AppError::InvalidInput(format!("Invalid JSON config: {e}")))?,
        "toml" => {
            let toml_value: toml::Value = toml::from_str(&config_content)
                .map_err(|e| AppError::InvalidInput(format!("Invalid TOML config: {e}")))?;
            // Convert TOML to JSON for uniform processing
            serde_json::to_value(toml_value)
                .map_err(|e| AppError::Message(format!("Failed to convert TOML to JSON: {e}")))?
        }
        _ => {
            return Err(AppError::InvalidInput(format!(
                "Unsupported config format: {format}"
            )))
        }
    };

    // Step 3: Extract values from config based on app type and merge with URL params
    let mut merged = request.clone();

    match request.app.as_str() {
        "claude" => merge_claude_config(&mut merged, &config_value)?,
        "codex" => merge_codex_config(&mut merged, &config_value)?,
        "gemini" => merge_gemini_config(&mut merged, &config_value)?,
        _ => {
            return Err(AppError::InvalidInput(format!(
                "Invalid app type: {}",
                request.app
            )))
        }
    }

    Ok(merged)
}

/// Merge Claude configuration from config file
///
/// Priority: URL params override config file values
fn merge_claude_config(
    request: &mut DeepLinkImportRequest,
    config: &serde_json::Value,
) -> Result<(), AppError> {
    let env = config
        .get("env")
        .and_then(|v| v.as_object())
        .ok_or_else(|| {
            AppError::InvalidInput("Claude config must have 'env' object".to_string())
        })?;

    // Auto-fill API key if not provided in URL
    if request.api_key.is_empty() {
        if let Some(token) = env.get("ANTHROPIC_AUTH_TOKEN").and_then(|v| v.as_str()) {
            request.api_key = token.to_string();
        }
    }

    // Auto-fill endpoint if not provided in URL
    if request.endpoint.is_empty() {
        if let Some(base_url) = env.get("ANTHROPIC_BASE_URL").and_then(|v| v.as_str()) {
            request.endpoint = base_url.to_string();
        }
    }

    // Auto-fill homepage from endpoint if not provided
    if request.homepage.is_empty() && !request.endpoint.is_empty() {
        request.homepage = infer_homepage_from_endpoint(&request.endpoint)
            .unwrap_or_else(|| "https://anthropic.com".to_string());
    }

    // Auto-fill model fields (URL params take priority)
    if request.model.is_none() {
        request.model = env
            .get("ANTHROPIC_MODEL")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
    }
    if request.haiku_model.is_none() {
        request.haiku_model = env
            .get("ANTHROPIC_DEFAULT_HAIKU_MODEL")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
    }
    if request.sonnet_model.is_none() {
        request.sonnet_model = env
            .get("ANTHROPIC_DEFAULT_SONNET_MODEL")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
    }
    if request.opus_model.is_none() {
        request.opus_model = env
            .get("ANTHROPIC_DEFAULT_OPUS_MODEL")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
    }

    Ok(())
}

/// Merge Codex configuration from config file
fn merge_codex_config(
    request: &mut DeepLinkImportRequest,
    config: &serde_json::Value,
) -> Result<(), AppError> {
    // Auto-fill API key from auth.OPENAI_API_KEY
    if request.api_key.is_empty() {
        if let Some(api_key) = config
            .get("auth")
            .and_then(|v| v.get("OPENAI_API_KEY"))
            .and_then(|v| v.as_str())
        {
            request.api_key = api_key.to_string();
        }
    }

    // Auto-fill endpoint and model from config string
    if let Some(config_str) = config.get("config").and_then(|v| v.as_str()) {
        // Parse TOML config string to extract base_url and model
        if let Ok(toml_value) = toml::from_str::<toml::Value>(config_str) {
            // Extract base_url from model_providers section
            if request.endpoint.is_empty() {
                if let Some(base_url) = extract_codex_base_url(&toml_value) {
                    request.endpoint = base_url;
                }
            }

            // Extract model
            if request.model.is_none() {
                if let Some(model) = toml_value.get("model").and_then(|v| v.as_str()) {
                    request.model = Some(model.to_string());
                }
            }
        }
    }

    // Auto-fill homepage from endpoint
    if request.homepage.is_empty() && !request.endpoint.is_empty() {
        request.homepage = infer_homepage_from_endpoint(&request.endpoint)
            .unwrap_or_else(|| "https://openai.com".to_string());
    }

    Ok(())
}

/// Merge Gemini configuration from config file
fn merge_gemini_config(
    request: &mut DeepLinkImportRequest,
    config: &serde_json::Value,
) -> Result<(), AppError> {
    // Gemini uses flat env structure
    if request.api_key.is_empty() {
        if let Some(api_key) = config.get("GEMINI_API_KEY").and_then(|v| v.as_str()) {
            request.api_key = api_key.to_string();
        }
    }

    if request.endpoint.is_empty() {
        if let Some(base_url) = config.get("GEMINI_BASE_URL").and_then(|v| v.as_str()) {
            request.endpoint = base_url.to_string();
        }
    }

    if request.model.is_none() {
        request.model = config
            .get("GEMINI_MODEL")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
    }

    // Auto-fill homepage from endpoint
    if request.homepage.is_empty() && !request.endpoint.is_empty() {
        request.homepage = infer_homepage_from_endpoint(&request.endpoint)
            .unwrap_or_else(|| "https://ai.google.dev".to_string());
    }

    Ok(())
}

/// Extract base_url from Codex TOML config
fn extract_codex_base_url(toml_value: &toml::Value) -> Option<String> {
    // Try to find base_url in model_providers section
    if let Some(providers) = toml_value.get("model_providers").and_then(|v| v.as_table()) {
        for (_key, provider) in providers.iter() {
            if let Some(base_url) = provider.get("base_url").and_then(|v| v.as_str()) {
                return Some(base_url.to_string());
            }
        }
    }
    None
}

/// Infer homepage URL from API endpoint
///
/// Examples:
/// - https://api.anthropic.com/v1 → https://anthropic.com
/// - https://api.openai.com/v1 → https://openai.com
/// - https://api-test.company.com/v1 → https://company.com
fn infer_homepage_from_endpoint(endpoint: &str) -> Option<String> {
    let url = Url::parse(endpoint).ok()?;
    let host = url.host_str()?;

    // Remove common API prefixes
    let clean_host = host
        .strip_prefix("api.")
        .or_else(|| host.strip_prefix("api-"))
        .unwrap_or(host);

    Some(format!("https://{clean_host}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_claude_deeplink() {
        let url = "ccswitch://v1/import?resource=provider&app=claude&name=Test%20Provider&homepage=https%3A%2F%2Fexample.com&endpoint=https%3A%2F%2Fapi.example.com&apiKey=sk-test-123";

        let request = parse_deeplink_url(url).unwrap();

        assert_eq!(request.version, "v1");
        assert_eq!(request.resource, "provider");
        assert_eq!(request.app, "claude");
        assert_eq!(request.name, "Test Provider");
        assert_eq!(request.homepage, "https://example.com");
        assert_eq!(request.endpoint, "https://api.example.com");
        assert_eq!(request.api_key, "sk-test-123");
    }

    #[test]
    fn test_parse_deeplink_with_notes() {
        let url = "ccswitch://v1/import?resource=provider&app=codex&name=Codex&homepage=https%3A%2F%2Fcodex.com&endpoint=https%3A%2F%2Fapi.codex.com&apiKey=key123&notes=Test%20notes";

        let request = parse_deeplink_url(url).unwrap();

        assert_eq!(request.notes, Some("Test notes".to_string()));
    }

    #[test]
    fn test_parse_invalid_scheme() {
        let url = "https://v1/import?resource=provider&app=claude&name=Test";

        let result = parse_deeplink_url(url);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid scheme"));
    }

    #[test]
    fn test_parse_unsupported_version() {
        let url = "ccswitch://v2/import?resource=provider&app=claude&name=Test";

        let result = parse_deeplink_url(url);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unsupported protocol version"));
    }

    #[test]
    fn test_parse_missing_required_field() {
        // Name is still required even in v3.8+ (only homepage/endpoint/apiKey are optional)
        let url = "ccswitch://v1/import?resource=provider&app=claude";

        let result = parse_deeplink_url(url);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Missing 'name' parameter"));
    }

    #[test]
    fn test_validate_invalid_url() {
        let result = validate_url("not-a-url", "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_invalid_scheme() {
        let result = validate_url("ftp://example.com", "test");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("must be http or https"));
    }

    #[test]
    fn test_build_gemini_provider_with_model() {
        let request = DeepLinkImportRequest {
            version: "v1".to_string(),
            resource: "provider".to_string(),
            app: "gemini".to_string(),
            name: "Test Gemini".to_string(),
            homepage: "https://example.com".to_string(),
            endpoint: "https://api.example.com".to_string(),
            api_key: "test-api-key".to_string(),
            model: Some("gemini-2.0-flash".to_string()),
            notes: None,
            haiku_model: None,
            sonnet_model: None,
            opus_model: None,
            config: None,
            config_format: None,
            config_url: None,
        };

        let provider = build_provider_from_request(&AppType::Gemini, &request).unwrap();

        // Verify provider basic info
        assert_eq!(provider.name, "Test Gemini");
        assert_eq!(
            provider.website_url,
            Some("https://example.com".to_string())
        );

        // Verify settings_config structure
        let env = provider.settings_config["env"].as_object().unwrap();
        assert_eq!(env["GEMINI_API_KEY"], "test-api-key");
        assert_eq!(env["GOOGLE_GEMINI_BASE_URL"], "https://api.example.com");
        assert_eq!(env["GEMINI_MODEL"], "gemini-2.0-flash");
    }

    #[test]
    fn test_build_gemini_provider_without_model() {
        let request = DeepLinkImportRequest {
            version: "v1".to_string(),
            resource: "provider".to_string(),
            app: "gemini".to_string(),
            name: "Test Gemini".to_string(),
            homepage: "https://example.com".to_string(),
            endpoint: "https://api.example.com".to_string(),
            api_key: "test-api-key".to_string(),
            model: None,
            notes: None,
            haiku_model: None,
            sonnet_model: None,
            opus_model: None,
            config: None,
            config_format: None,
            config_url: None,
        };

        let provider = build_provider_from_request(&AppType::Gemini, &request).unwrap();

        // Verify settings_config structure
        let env = provider.settings_config["env"].as_object().unwrap();
        assert_eq!(env["GEMINI_API_KEY"], "test-api-key");
        assert_eq!(env["GOOGLE_GEMINI_BASE_URL"], "https://api.example.com");
        // Model should not be present
        assert!(env.get("GEMINI_MODEL").is_none());
    }

    #[test]
    fn test_infer_homepage() {
        assert_eq!(
            infer_homepage_from_endpoint("https://api.anthropic.com/v1"),
            Some("https://anthropic.com".to_string())
        );
        assert_eq!(
            infer_homepage_from_endpoint("https://api-test.company.com/v1"),
            Some("https://test.company.com".to_string())
        );
        assert_eq!(
            infer_homepage_from_endpoint("https://example.com"),
            Some("https://example.com".to_string())
        );
    }

    #[test]
    fn test_parse_and_merge_config_claude() {
        use base64::prelude::*;

        // Prepare Base64 encoded Claude config
        let config_json = r#"{"env":{"ANTHROPIC_AUTH_TOKEN":"sk-ant-xxx","ANTHROPIC_BASE_URL":"https://api.anthropic.com/v1","ANTHROPIC_MODEL":"claude-sonnet-4.5"}}"#;
        let config_b64 = BASE64_STANDARD.encode(config_json.as_bytes());

        let request = DeepLinkImportRequest {
            version: "v1".to_string(),
            resource: "provider".to_string(),
            app: "claude".to_string(),
            name: "Test".to_string(),
            homepage: String::new(),
            endpoint: String::new(),
            api_key: String::new(),
            model: None,
            notes: None,
            haiku_model: None,
            sonnet_model: None,
            opus_model: None,
            config: Some(config_b64),
            config_format: Some("json".to_string()),
            config_url: None,
        };

        let merged = parse_and_merge_config(&request).unwrap();

        // Should auto-fill from config
        assert_eq!(merged.api_key, "sk-ant-xxx");
        assert_eq!(merged.endpoint, "https://api.anthropic.com/v1");
        assert_eq!(merged.homepage, "https://anthropic.com");
        assert_eq!(merged.model, Some("claude-sonnet-4.5".to_string()));
    }

    #[test]
    fn test_parse_and_merge_config_url_override() {
        use base64::prelude::*;

        let config_json = r#"{"env":{"ANTHROPIC_AUTH_TOKEN":"sk-old","ANTHROPIC_BASE_URL":"https://api.anthropic.com/v1"}}"#;
        let config_b64 = BASE64_STANDARD.encode(config_json.as_bytes());

        let request = DeepLinkImportRequest {
            version: "v1".to_string(),
            resource: "provider".to_string(),
            app: "claude".to_string(),
            name: "Test".to_string(),
            homepage: String::new(),
            endpoint: String::new(),
            api_key: "sk-new".to_string(), // URL param should override
            model: None,
            notes: None,
            haiku_model: None,
            sonnet_model: None,
            opus_model: None,
            config: Some(config_b64),
            config_format: Some("json".to_string()),
            config_url: None,
        };

        let merged = parse_and_merge_config(&request).unwrap();

        // URL param should take priority
        assert_eq!(merged.api_key, "sk-new");
        // Config file value should be used
        assert_eq!(merged.endpoint, "https://api.anthropic.com/v1");
    }
}
