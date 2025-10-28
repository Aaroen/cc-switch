#![allow(non_snake_case)]

use std::collections::HashMap;

use tauri::State;

use crate::app_config::AppType;
use crate::error::AppError;
use crate::provider::Provider;
use crate::services::{EndpointLatency, ProviderService, ProviderSortUpdate, SpeedtestService};
use crate::store::AppState;

fn missing_param(param: &str) -> String {
    format!("缺少 {} 参数 (Missing {} parameter)", param, param)
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

    ProviderService::list(state.inner(), app_type).map_err(|e| e.to_string())
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

    ProviderService::current(state.inner(), app_type).map_err(|e| e.to_string())
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

    ProviderService::add(state.inner(), app_type, provider).map_err(|e| e.to_string())
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

    ProviderService::update(state.inner(), app_type, provider).map_err(|e| e.to_string())
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
            .write()
            .map_err(|e| format!("获取锁失败: {}", e))?;
        ProviderService::delete(&mut config, app_type, &id).map_err(|e| e.to_string())?;
    }

    state.save()?;

    Ok(true)
}

/// 切换供应商
fn switch_provider_internal(state: &AppState, app_type: AppType, id: &str) -> Result<(), AppError> {
    ProviderService::switch(state, app_type, id)
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
    ProviderService::import_default_config(state, app_type)
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

    import_default_config_internal(&state, app_type)
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
    let provider_id = provider_id
        .or(providerId)
        .ok_or_else(|| missing_param("providerId"))?;

    let app_type = app_type
        .or_else(|| app.as_deref().map(|s| s.into()))
        .or_else(|| appType.as_deref().map(|s| s.into()))
        .unwrap_or(AppType::Claude);

    ProviderService::query_usage(state.inner(), app_type, &provider_id)
        .await
        .map_err(|e| e.to_string())
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

    ProviderService::read_live_settings(app_type).map_err(|e| e.to_string())
}

/// 测试第三方/自定义供应商端点的网络延迟
#[tauri::command]
pub async fn test_api_endpoints(
    urls: Vec<String>,
    timeout_secs: Option<u64>,
) -> Result<Vec<EndpointLatency>, String> {
    SpeedtestService::test_endpoints(urls, timeout_secs)
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
        .ok_or_else(|| missing_param("providerId"))?;
    ProviderService::get_custom_endpoints(state.inner(), app_type, &provider_id)
        .map_err(|e| e.to_string())
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
        .ok_or_else(|| missing_param("providerId"))?;
    ProviderService::add_custom_endpoint(state.inner(), app_type, &provider_id, url)
        .map_err(|e| e.to_string())
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
        .ok_or_else(|| missing_param("providerId"))?;
    ProviderService::remove_custom_endpoint(state.inner(), app_type, &provider_id, url)
        .map_err(|e| e.to_string())
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
        .ok_or_else(|| missing_param("providerId"))?;
    ProviderService::update_endpoint_last_used(state.inner(), app_type, &provider_id, url)
        .map_err(|e| e.to_string())
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

    ProviderService::update_sort_order(state.inner(), app_type, updates).map_err(|e| e.to_string())
}
