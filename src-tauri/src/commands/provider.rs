#![allow(non_snake_case)]

use std::collections::HashMap;

use serde::Deserialize;
use tauri::State;

use crate::app_config::AppType;
use crate::codex_config;
use crate::config::get_claude_settings_path;
use crate::error::AppError;
use crate::provider::{Provider, ProviderMeta};
use crate::services::ProviderService;
use crate::speedtest;
use crate::store::AppState;

fn validate_provider_settings(app_type: &AppType, provider: &Provider) -> Result<(), String> {
    match app_type {
        AppType::Claude => {
            if !provider.settings_config.is_object() {
                return Err("Claude 配置必须是 JSON 对象".to_string());
            }
        }
        AppType::Codex => {
            let settings = provider
                .settings_config
                .as_object()
                .ok_or_else(|| "Codex 配置必须是 JSON 对象".to_string())?;
            let auth = settings
                .get("auth")
                .ok_or_else(|| "Codex 配置缺少 auth 字段".to_string())?;
            if !auth.is_object() {
                return Err("Codex auth 配置必须是 JSON 对象".to_string());
            }
            if let Some(config_value) = settings.get("config") {
                if !(config_value.is_string() || config_value.is_null()) {
                    return Err("Codex config 字段必须是字符串".to_string());
                }
                if let Some(cfg_text) = config_value.as_str() {
                    codex_config::validate_config_toml(cfg_text)?;
                }
            }
        }
    }
    Ok(())
}

/// 获取所有供应商
#[tauri::command]
pub async fn get_providers(
    state: State<'_, AppState>,
    app_type: Option<AppType>,
    app: Option<String>,
    appType: Option<String>,
) -> Result<HashMap<String, Provider>, String> {
    let app_type = app_type
        .or_else(|| app.as_deref().map(|s| s.into()))
        .or_else(|| appType.as_deref().map(|s| s.into()))
        .unwrap_or(AppType::Claude);

    let config = state
        .config
        .lock()
        .map_err(|e| format!("获取锁失败: {}", e))?;

    let manager = config
        .get_manager(&app_type)
        .ok_or_else(|| format!("应用类型不存在: {:?}", app_type))?;

    Ok(manager.get_all_providers().clone())
}

/// 获取当前供应商ID
#[tauri::command]
pub async fn get_current_provider(
    state: State<'_, AppState>,
    app_type: Option<AppType>,
    app: Option<String>,
    appType: Option<String>,
) -> Result<String, String> {
    let app_type = app_type
        .or_else(|| app.as_deref().map(|s| s.into()))
        .or_else(|| appType.as_deref().map(|s| s.into()))
        .unwrap_or(AppType::Claude);

    let config = state
        .config
        .lock()
        .map_err(|e| format!("获取锁失败: {}", e))?;

    let manager = config
        .get_manager(&app_type)
        .ok_or_else(|| format!("应用类型不存在: {:?}", app_type))?;

    Ok(manager.current.clone())
}

/// 添加供应商
#[tauri::command]
pub async fn add_provider(
    state: State<'_, AppState>,
    app_type: Option<AppType>,
    app: Option<String>,
    appType: Option<String>,
    provider: Provider,
) -> Result<bool, String> {
    let app_type = app_type
        .or_else(|| app.as_deref().map(|s| s.into()))
        .or_else(|| appType.as_deref().map(|s| s.into()))
        .unwrap_or(AppType::Claude);

    validate_provider_settings(&app_type, &provider)?;

    let is_current = {
        let config = state
            .config
            .lock()
            .map_err(|e| format!("获取锁失败: {}", e))?;
        let manager = config
            .get_manager(&app_type)
            .ok_or_else(|| format!("应用类型不存在: {:?}", app_type))?;
        manager.current == provider.id
    };

    if is_current {
        match app_type {
            AppType::Claude => {
                let settings_path = crate::config::get_claude_settings_path();
                crate::config::write_json_file(&settings_path, &provider.settings_config)?;
            }
            AppType::Codex => {
                let auth = provider
                    .settings_config
                    .get("auth")
                    .ok_or_else(|| "目标供应商缺少 auth 配置".to_string())?;
                let cfg_text = provider
                    .settings_config
                    .get("config")
                    .and_then(|v| v.as_str());
                crate::codex_config::write_codex_live_atomic(auth, cfg_text)?;
            }
        }
    }

    {
        let mut config = state
            .config
            .lock()
            .map_err(|e| format!("获取锁失败: {}", e))?;
        let manager = config
            .get_manager_mut(&app_type)
            .ok_or_else(|| format!("应用类型不存在: {:?}", app_type))?;
        manager
            .providers
            .insert(provider.id.clone(), provider.clone());
    }
    state.save()?;

    Ok(true)
}

/// 更新供应商
#[tauri::command]
pub async fn update_provider(
    state: State<'_, AppState>,
    app_type: Option<AppType>,
    app: Option<String>,
    appType: Option<String>,
    provider: Provider,
) -> Result<bool, String> {
    let app_type = app_type
        .or_else(|| app.as_deref().map(|s| s.into()))
        .or_else(|| appType.as_deref().map(|s| s.into()))
        .unwrap_or(AppType::Claude);

    validate_provider_settings(&app_type, &provider)?;

    let (exists, is_current) = {
        let config = state
            .config
            .lock()
            .map_err(|e| format!("获取锁失败: {}", e))?;
        let manager = config
            .get_manager(&app_type)
            .ok_or_else(|| format!("应用类型不存在: {:?}", app_type))?;
        (
            manager.providers.contains_key(&provider.id),
            manager.current == provider.id,
        )
    };
    if !exists {
        return Err(format!("供应商不存在: {}", provider.id));
    }

    if is_current {
        match app_type {
            AppType::Claude => {
                let settings_path = crate::config::get_claude_settings_path();
                crate::config::write_json_file(&settings_path, &provider.settings_config)?;
            }
            AppType::Codex => {
                let auth = provider
                    .settings_config
                    .get("auth")
                    .ok_or_else(|| "目标供应商缺少 auth 配置".to_string())?;
                let cfg_text = provider
                    .settings_config
                    .get("config")
                    .and_then(|v| v.as_str());
                crate::codex_config::write_codex_live_atomic(auth, cfg_text)?;
            }
        }
    }

    {
        let mut config = state
            .config
            .lock()
            .map_err(|e| format!("获取锁失败: {}", e))?;
        let manager = config
            .get_manager_mut(&app_type)
            .ok_or_else(|| format!("应用类型不存在: {:?}", app_type))?;

        let merged_provider = if let Some(existing) = manager.providers.get(&provider.id) {
            let mut updated = provider.clone();

            match (existing.meta.as_ref(), updated.meta.take()) {
                (Some(old_meta), None) => {
                    updated.meta = Some(old_meta.clone());
                }
                (Some(old_meta), Some(mut new_meta)) => {
                    let mut merged_map = old_meta.custom_endpoints.clone();
                    for (url, ep) in new_meta.custom_endpoints.drain() {
                        merged_map.entry(url).or_insert(ep);
                    }
                    updated.meta = Some(ProviderMeta {
                        custom_endpoints: merged_map,
                        usage_script: new_meta.usage_script.clone(),
                    });
                }
                (None, maybe_new) => {
                    updated.meta = maybe_new;
                }
            }

            updated
        } else {
            provider.clone()
        };

        manager
            .providers
            .insert(merged_provider.id.clone(), merged_provider);
    }
    state.save()?;

    Ok(true)
}

/// 删除供应商
#[tauri::command]
pub async fn delete_provider(
    state: State<'_, AppState>,
    app_type: Option<AppType>,
    app: Option<String>,
    appType: Option<String>,
    id: String,
) -> Result<bool, String> {
    let app_type = app_type
        .or_else(|| app.as_deref().map(|s| s.into()))
        .or_else(|| appType.as_deref().map(|s| s.into()))
        .unwrap_or(AppType::Claude);

    {
        let mut config = state
            .config
            .lock()
            .map_err(|e| format!("获取锁失败: {}", e))?;
        ProviderService::delete(&mut config, app_type, &id).map_err(|e| e.to_string())?;
    }

    state.save()?;

    Ok(true)
}

/// 切换供应商
fn switch_provider_internal(state: &AppState, app_type: AppType, id: &str) -> Result<(), AppError> {
    let mut config = state.config.lock().map_err(AppError::from)?;

    ProviderService::switch(&mut config, app_type, id)?;

    drop(config);
    state.save()
}

#[doc(hidden)]
pub fn switch_provider_test_hook(
    state: &AppState,
    app_type: AppType,
    id: &str,
) -> Result<(), AppError> {
    switch_provider_internal(state, app_type, id)
}

#[tauri::command]
pub async fn switch_provider(
    state: State<'_, AppState>,
    app_type: Option<AppType>,
    app: Option<String>,
    appType: Option<String>,
    id: String,
) -> Result<bool, String> {
    let app_type = app_type
        .or_else(|| app.as_deref().map(|s| s.into()))
        .or_else(|| appType.as_deref().map(|s| s.into()))
        .unwrap_or(AppType::Claude);

    switch_provider_internal(&state, app_type, &id)
        .map(|_| true)
        .map_err(|e| e.to_string())
}

fn import_default_config_internal(state: &AppState, app_type: AppType) -> Result<(), AppError> {
    {
        let config = state.config.lock()?;
        if let Some(manager) = config.get_manager(&app_type) {
            if !manager.get_all_providers().is_empty() {
                // 已存在供应商则视为已导入，保持与原逻辑一致
                return Ok(());
            }
        }
    }

    let settings_config = match app_type {
        AppType::Codex => {
            let auth_path = codex_config::get_codex_auth_path();
            if !auth_path.exists() {
                return Err(AppError::Message("Codex 配置文件不存在".to_string()));
            }
            let auth: serde_json::Value = crate::config::read_json_file(&auth_path)?;
            let config_str = crate::codex_config::read_and_validate_codex_config_text()?;
            serde_json::json!({ "auth": auth, "config": config_str })
        }
        AppType::Claude => {
            let settings_path = get_claude_settings_path();
            if !settings_path.exists() {
                return Err(AppError::Message("Claude Code 配置文件不存在".to_string()));
            }
            crate::config::read_json_file(&settings_path)?
        }
    };

    let provider = Provider::with_id(
        "default".to_string(),
        "default".to_string(),
        settings_config,
        None,
    );

    let mut config = state.config.lock()?;
    let manager = config
        .get_manager_mut(&app_type)
        .ok_or_else(|| AppError::Message(format!("应用类型不存在: {:?}", app_type)))?;

    manager.providers.insert(provider.id.clone(), provider);
    manager.current = "default".to_string();

    drop(config);
    state.save()?;

    Ok(())
}

#[doc(hidden)]
pub fn import_default_config_test_hook(
    state: &AppState,
    app_type: AppType,
) -> Result<(), AppError> {
    import_default_config_internal(state, app_type)
}

/// 导入当前配置为默认供应商
#[tauri::command]
pub async fn import_default_config(
    state: State<'_, AppState>,
    app_type: Option<AppType>,
    app: Option<String>,
    appType: Option<String>,
) -> Result<bool, String> {
    let app_type = app_type
        .or_else(|| app.as_deref().map(|s| s.into()))
        .or_else(|| appType.as_deref().map(|s| s.into()))
        .unwrap_or(AppType::Claude);

    import_default_config_internal(&*state, app_type)
        .map(|_| true)
        .map_err(Into::into)
}

/// 查询供应商用量
#[tauri::command]
pub async fn query_provider_usage(
    state: State<'_, AppState>,
    provider_id: Option<String>,
    providerId: Option<String>,
    app_type: Option<AppType>,
    app: Option<String>,
    appType: Option<String>,
) -> Result<crate::provider::UsageResult, String> {
    use crate::provider::{UsageData, UsageResult};

    let provider_id = provider_id.or(providerId).ok_or("缺少 providerId 参数")?;

    let app_type = app_type
        .or_else(|| app.as_deref().map(|s| s.into()))
        .or_else(|| appType.as_deref().map(|s| s.into()))
        .unwrap_or(AppType::Claude);

    let (api_key, base_url, usage_script_code, timeout) = {
        let config = state
            .config
            .lock()
            .map_err(|e| format!("获取锁失败: {}", e))?;

        let manager = config.get_manager(&app_type).ok_or("应用类型不存在")?;

        let provider = manager.providers.get(&provider_id).ok_or("供应商不存在")?;

        let usage_script = provider
            .meta
            .as_ref()
            .and_then(|m| m.usage_script.as_ref())
            .ok_or("未配置用量查询脚本")?;

        if !usage_script.enabled {
            return Err("用量查询未启用".to_string());
        }

        let (api_key, base_url) = extract_credentials(provider, &app_type)?;
        let timeout = usage_script.timeout.unwrap_or(10);
        let code = usage_script.code.clone();

        drop(config);

        (api_key, base_url, code, timeout)
    };

    let result =
        crate::usage_script::execute_usage_script(&usage_script_code, &api_key, &base_url, timeout)
            .await;

    match result {
        Ok(data) => {
            let usage_list: Vec<UsageData> = if data.is_array() {
                serde_json::from_value(data).map_err(|e| format!("数据格式错误: {}", e))?
            } else {
                let single: UsageData =
                    serde_json::from_value(data).map_err(|e| format!("数据格式错误: {}", e))?;
                vec![single]
            };

            Ok(UsageResult {
                success: true,
                data: Some(usage_list),
                error: None,
            })
        }
        Err(e) => Ok(UsageResult {
            success: false,
            data: None,
            error: Some(e.to_string()),
        }),
    }
}

fn extract_credentials(
    provider: &crate::provider::Provider,
    app_type: &AppType,
) -> Result<(String, String), String> {
    match app_type {
        AppType::Claude => {
            let env = provider
                .settings_config
                .get("env")
                .and_then(|v| v.as_object())
                .ok_or("配置格式错误: 缺少 env")?;

            let api_key = env
                .get("ANTHROPIC_AUTH_TOKEN")
                .and_then(|v| v.as_str())
                .ok_or("缺少 API Key")?
                .to_string();

            let base_url = env
                .get("ANTHROPIC_BASE_URL")
                .and_then(|v| v.as_str())
                .ok_or("缺少 ANTHROPIC_BASE_URL 配置")?
                .to_string();

            Ok((api_key, base_url))
        }
        AppType::Codex => {
            let auth = provider
                .settings_config
                .get("auth")
                .and_then(|v| v.as_object())
                .ok_or("配置格式错误: 缺少 auth")?;

            let api_key = auth
                .get("OPENAI_API_KEY")
                .and_then(|v| v.as_str())
                .ok_or("缺少 API Key")?
                .to_string();

            let config_toml = provider
                .settings_config
                .get("config")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let base_url = if config_toml.contains("base_url") {
                let re = regex::Regex::new(r#"base_url\s*=\s*["']([^"']+)["']"#).unwrap();
                re.captures(config_toml)
                    .and_then(|caps| caps.get(1))
                    .map(|m| m.as_str().to_string())
                    .ok_or("config.toml 中 base_url 格式错误")?
            } else {
                return Err("config.toml 中缺少 base_url 配置".to_string());
            };

            Ok((api_key, base_url))
        }
    }
}

/// 读取当前生效的配置内容
#[tauri::command]
pub async fn read_live_provider_settings(
    app_type: Option<AppType>,
    app: Option<String>,
    appType: Option<String>,
) -> Result<serde_json::Value, String> {
    let app_type = app_type
        .or_else(|| app.as_deref().map(|s| s.into()))
        .or_else(|| appType.as_deref().map(|s| s.into()))
        .unwrap_or(AppType::Claude);

    match app_type {
        AppType::Codex => {
            let auth_path = crate::codex_config::get_codex_auth_path();
            if !auth_path.exists() {
                return Err("Codex 配置文件不存在：缺少 auth.json".to_string());
            }
            let auth: serde_json::Value = crate::config::read_json_file(&auth_path)?;
            let cfg_text = crate::codex_config::read_and_validate_codex_config_text()?;
            Ok(serde_json::json!({ "auth": auth, "config": cfg_text }))
        }
        AppType::Claude => {
            let path = crate::config::get_claude_settings_path();
            if !path.exists() {
                return Err("Claude Code 配置文件不存在".to_string());
            }
            let v: serde_json::Value = crate::config::read_json_file(&path)?;
            Ok(v)
        }
    }
}

/// 测试第三方/自定义供应商端点的网络延迟
#[tauri::command]
pub async fn test_api_endpoints(
    urls: Vec<String>,
    timeout_secs: Option<u64>,
) -> Result<Vec<speedtest::EndpointLatency>, String> {
    let filtered: Vec<String> = urls
        .into_iter()
        .filter(|url| !url.trim().is_empty())
        .collect();
    speedtest::test_endpoints(filtered, timeout_secs)
        .await
        .map_err(|e| e.to_string())
}

/// 获取自定义端点列表
#[tauri::command]
pub async fn get_custom_endpoints(
    state: State<'_, AppState>,
    app_type: Option<AppType>,
    app: Option<String>,
    appType: Option<String>,
    provider_id: Option<String>,
    providerId: Option<String>,
) -> Result<Vec<crate::settings::CustomEndpoint>, String> {
    let app_type = app_type
        .or_else(|| app.as_deref().map(|s| s.into()))
        .or_else(|| appType.as_deref().map(|s| s.into()))
        .unwrap_or(AppType::Claude);
    let provider_id = provider_id
        .or(providerId)
        .ok_or_else(|| "缺少 providerId".to_string())?;
    let mut cfg_guard = state
        .config
        .lock()
        .map_err(|e| format!("获取锁失败: {}", e))?;

    let manager = cfg_guard
        .get_manager_mut(&app_type)
        .ok_or_else(|| format!("应用类型不存在: {:?}", app_type))?;

    let Some(provider) = manager.providers.get_mut(&provider_id) else {
        return Ok(vec![]);
    };

    let meta = provider.meta.get_or_insert_with(ProviderMeta::default);
    if !meta.custom_endpoints.is_empty() {
        let mut result: Vec<_> = meta.custom_endpoints.values().cloned().collect();
        result.sort_by(|a, b| b.added_at.cmp(&a.added_at));
        return Ok(result);
    }

    Ok(vec![])
}

/// 添加自定义端点
#[tauri::command]
pub async fn add_custom_endpoint(
    state: State<'_, AppState>,
    app_type: Option<AppType>,
    app: Option<String>,
    appType: Option<String>,
    provider_id: Option<String>,
    providerId: Option<String>,
    url: String,
) -> Result<(), String> {
    let app_type = app_type
        .or_else(|| app.as_deref().map(|s| s.into()))
        .or_else(|| appType.as_deref().map(|s| s.into()))
        .unwrap_or(AppType::Claude);
    let provider_id = provider_id
        .or(providerId)
        .ok_or_else(|| "缺少 providerId".to_string())?;
    let normalized = url.trim().trim_end_matches('/').to_string();
    if normalized.is_empty() {
        return Err("URL 不能为空".to_string());
    }

    let mut cfg_guard = state
        .config
        .lock()
        .map_err(|e| format!("获取锁失败: {}", e))?;
    let manager = cfg_guard
        .get_manager_mut(&app_type)
        .ok_or_else(|| format!("应用类型不存在: {:?}", app_type))?;

    let Some(provider) = manager.providers.get_mut(&provider_id) else {
        return Err("供应商不存在或未选择".to_string());
    };
    let meta = provider.meta.get_or_insert_with(ProviderMeta::default);

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    let endpoint = crate::settings::CustomEndpoint {
        url: normalized.clone(),
        added_at: timestamp,
        last_used: None,
    };
    meta.custom_endpoints.insert(normalized, endpoint);
    drop(cfg_guard);
    state.save()?;
    Ok(())
}

/// 删除自定义端点
#[tauri::command]
pub async fn remove_custom_endpoint(
    state: State<'_, AppState>,
    app_type: Option<AppType>,
    app: Option<String>,
    appType: Option<String>,
    provider_id: Option<String>,
    providerId: Option<String>,
    url: String,
) -> Result<(), String> {
    let app_type = app_type
        .or_else(|| app.as_deref().map(|s| s.into()))
        .or_else(|| appType.as_deref().map(|s| s.into()))
        .unwrap_or(AppType::Claude);
    let provider_id = provider_id
        .or(providerId)
        .ok_or_else(|| "缺少 providerId".to_string())?;
    let normalized = url.trim().trim_end_matches('/').to_string();

    let mut cfg_guard = state
        .config
        .lock()
        .map_err(|e| format!("获取锁失败: {}", e))?;
    let manager = cfg_guard
        .get_manager_mut(&app_type)
        .ok_or_else(|| format!("应用类型不存在: {:?}", app_type))?;

    if let Some(provider) = manager.providers.get_mut(&provider_id) {
        if let Some(meta) = provider.meta.as_mut() {
            meta.custom_endpoints.remove(&normalized);
        }
    }
    drop(cfg_guard);
    state.save()?;
    Ok(())
}

/// 更新端点最后使用时间
#[tauri::command]
pub async fn update_endpoint_last_used(
    state: State<'_, AppState>,
    app_type: Option<AppType>,
    app: Option<String>,
    appType: Option<String>,
    provider_id: Option<String>,
    providerId: Option<String>,
    url: String,
) -> Result<(), String> {
    let app_type = app_type
        .or_else(|| app.as_deref().map(|s| s.into()))
        .or_else(|| appType.as_deref().map(|s| s.into()))
        .unwrap_or(AppType::Claude);
    let provider_id = provider_id
        .or(providerId)
        .ok_or_else(|| "缺少 providerId".to_string())?;
    let normalized = url.trim().trim_end_matches('/').to_string();

    let mut cfg_guard = state
        .config
        .lock()
        .map_err(|e| format!("获取锁失败: {}", e))?;
    let manager = cfg_guard
        .get_manager_mut(&app_type)
        .ok_or_else(|| format!("应用类型不存在: {:?}", app_type))?;

    if let Some(provider) = manager.providers.get_mut(&provider_id) {
        if let Some(meta) = provider.meta.as_mut() {
            if let Some(endpoint) = meta.custom_endpoints.get_mut(&normalized) {
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as i64;
                endpoint.last_used = Some(timestamp);
            }
        }
    }
    drop(cfg_guard);
    state.save()?;
    Ok(())
}

#[derive(Deserialize)]
pub struct ProviderSortUpdate {
    pub id: String,
    #[serde(rename = "sortIndex")]
    pub sort_index: usize,
}

/// 更新多个供应商的排序
#[tauri::command]
pub async fn update_providers_sort_order(
    state: State<'_, AppState>,
    app_type: Option<AppType>,
    app: Option<String>,
    appType: Option<String>,
    updates: Vec<ProviderSortUpdate>,
) -> Result<bool, String> {
    let app_type = app_type
        .or_else(|| app.as_deref().map(|s| s.into()))
        .or_else(|| appType.as_deref().map(|s| s.into()))
        .unwrap_or(AppType::Claude);

    let mut config = state
        .config
        .lock()
        .map_err(|e| format!("获取锁失败: {}", e))?;

    let manager = config
        .get_manager_mut(&app_type)
        .ok_or_else(|| format!("应用类型不存在: {:?}", app_type))?;

    for update in updates {
        if let Some(provider) = manager.providers.get_mut(&update.id) {
            provider.sort_index = Some(update.sort_index);
        }
    }

    drop(config);
    state.save()?;

    Ok(true)
}
