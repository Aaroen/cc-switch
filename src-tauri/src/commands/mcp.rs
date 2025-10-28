#![allow(non_snake_case)]

use std::collections::HashMap;

use serde::Serialize;
use tauri::State;

use crate::app_config::AppType;
use crate::claude_mcp;
use crate::error::AppError;
use crate::mcp;
use crate::store::AppState;

/// 获取 Claude MCP 状态
#[tauri::command]
pub async fn get_claude_mcp_status() -> Result<claude_mcp::McpStatus, String> {
    claude_mcp::get_mcp_status().map_err(|e| e.to_string())
}

/// 读取 mcp.json 文本内容
#[tauri::command]
pub async fn read_claude_mcp_config() -> Result<Option<String>, String> {
    claude_mcp::read_mcp_json().map_err(|e| e.to_string())
}

/// 新增或更新一个 MCP 服务器条目
#[tauri::command]
pub async fn upsert_claude_mcp_server(id: String, spec: serde_json::Value) -> Result<bool, String> {
    claude_mcp::upsert_mcp_server(&id, spec).map_err(|e| e.to_string())
}

/// 删除一个 MCP 服务器条目
#[tauri::command]
pub async fn delete_claude_mcp_server(id: String) -> Result<bool, String> {
    claude_mcp::delete_mcp_server(&id).map_err(|e| e.to_string())
}

/// 校验命令是否在 PATH 中可用（不执行）
#[tauri::command]
pub async fn validate_mcp_command(cmd: String) -> Result<bool, String> {
    claude_mcp::validate_command_in_path(&cmd).map_err(|e| e.to_string())
}

#[derive(Serialize)]
pub struct McpConfigResponse {
    pub config_path: String,
    pub servers: HashMap<String, serde_json::Value>,
}

/// 获取 MCP 配置（来自 ~/.cc-switch/config.json）
#[tauri::command]
pub async fn get_mcp_config(
    state: State<'_, AppState>,
    app: Option<String>,
) -> Result<McpConfigResponse, String> {
    let config_path = crate::config::get_app_config_path()
        .to_string_lossy()
        .to_string();
    let mut cfg = state
        .config
        .lock()
        .map_err(|e| format!("获取锁失败: {}", e))?;
    let app_ty = AppType::from(app.as_deref().unwrap_or("claude"));
    let (servers, normalized) = mcp::get_servers_snapshot_for(&mut cfg, &app_ty);
    let need_save = normalized > 0;
    drop(cfg);
    if need_save {
        state.save()?;
    }
    Ok(McpConfigResponse {
        config_path,
        servers,
    })
}

/// 在 config.json 中新增或更新一个 MCP 服务器定义
#[tauri::command]
pub async fn upsert_mcp_server_in_config(
    state: State<'_, AppState>,
    app: Option<String>,
    id: String,
    spec: serde_json::Value,
    sync_other_side: Option<bool>,
) -> Result<bool, String> {
    let mut cfg = state
        .config
        .lock()
        .map_err(|e| format!("获取锁失败: {}", e))?;
    let app_ty = AppType::from(app.as_deref().unwrap_or("claude"));
    let mut sync_targets: Vec<AppType> = Vec::new();

    let changed = mcp::upsert_in_config_for(&mut cfg, &app_ty, &id, spec.clone())?;

    let should_sync_current = cfg
        .mcp_for(&app_ty)
        .servers
        .get(&id)
        .and_then(|entry| entry.get("enabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if should_sync_current {
        sync_targets.push(app_ty.clone());
    }

    if sync_other_side.unwrap_or(false) {
        match app_ty {
            AppType::Claude => sync_targets.push(AppType::Codex),
            AppType::Codex => sync_targets.push(AppType::Claude),
        }
    }

    drop(cfg);
    state.save()?;

    let cfg2 = state
        .config
        .lock()
        .map_err(|e| format!("获取锁失败: {}", e))?;
    for app_ty_to_sync in sync_targets {
        match app_ty_to_sync {
            AppType::Claude => mcp::sync_enabled_to_claude(&cfg2)?,
            AppType::Codex => mcp::sync_enabled_to_codex(&cfg2)?,
        };
    }
    Ok(changed)
}

/// 在 config.json 中删除一个 MCP 服务器定义
#[tauri::command]
pub async fn delete_mcp_server_in_config(
    state: State<'_, AppState>,
    app: Option<String>,
    id: String,
) -> Result<bool, String> {
    let mut cfg = state
        .config
        .lock()
        .map_err(|e| format!("获取锁失败: {}", e))?;
    let app_ty = AppType::from(app.as_deref().unwrap_or("claude"));
    let existed = mcp::delete_in_config_for(&mut cfg, &app_ty, &id)?;
    drop(cfg);
    state.save()?;
    let cfg2 = state
        .config
        .lock()
        .map_err(|e| format!("获取锁失败: {}", e))?;
    match app_ty {
        AppType::Claude => mcp::sync_enabled_to_claude(&cfg2)?,
        AppType::Codex => mcp::sync_enabled_to_codex(&cfg2)?,
    }
    Ok(existed)
}

/// 设置启用状态并同步到客户端配置
#[tauri::command]
pub async fn set_mcp_enabled(
    state: State<'_, AppState>,
    app: Option<String>,
    id: String,
    enabled: bool,
) -> Result<bool, String> {
    let app_ty = AppType::from(app.as_deref().unwrap_or("claude"));
    set_mcp_enabled_internal(&*state, app_ty, &id, enabled).map_err(Into::into)
}

/// 手动同步：将启用的 MCP 投影到 ~/.claude.json
#[tauri::command]
pub async fn sync_enabled_mcp_to_claude(state: State<'_, AppState>) -> Result<bool, String> {
    let mut cfg = state
        .config
        .lock()
        .map_err(|e| format!("获取锁失败: {}", e))?;
    let normalized = mcp::normalize_servers_for(&mut cfg, &AppType::Claude);
    mcp::sync_enabled_to_claude(&cfg)?;
    let need_save = normalized > 0;
    drop(cfg);
    if need_save {
        state.save()?;
    }
    Ok(true)
}

/// 手动同步：将启用的 MCP 投影到 ~/.codex/config.toml
#[tauri::command]
pub async fn sync_enabled_mcp_to_codex(state: State<'_, AppState>) -> Result<bool, String> {
    let mut cfg = state
        .config
        .lock()
        .map_err(|e| format!("获取锁失败: {}", e))?;
    let normalized = mcp::normalize_servers_for(&mut cfg, &AppType::Codex);
    mcp::sync_enabled_to_codex(&cfg)?;
    let need_save = normalized > 0;
    drop(cfg);
    if need_save {
        state.save()?;
    }
    Ok(true)
}

/// 从 ~/.claude.json 导入 MCP 定义到 config.json
#[tauri::command]
pub async fn import_mcp_from_claude(state: State<'_, AppState>) -> Result<usize, String> {
    import_mcp_from_claude_internal(&*state).map_err(Into::into)
}

/// 从 ~/.codex/config.toml 导入 MCP 定义到 config.json
#[tauri::command]
pub async fn import_mcp_from_codex(state: State<'_, AppState>) -> Result<usize, String> {
    import_mcp_from_codex_internal(&*state).map_err(Into::into)
}

fn set_mcp_enabled_internal(
    state: &AppState,
    app_ty: AppType,
    id: &str,
    enabled: bool,
) -> Result<bool, AppError> {
    let mut cfg = state.config.lock()?;
    let changed = mcp::set_enabled_and_sync_for(&mut cfg, &app_ty, id, enabled)?;
    drop(cfg);
    state.save()?;
    Ok(changed)
}

#[doc(hidden)]
pub fn set_mcp_enabled_test_hook(
    state: &AppState,
    app_ty: AppType,
    id: &str,
    enabled: bool,
) -> Result<bool, AppError> {
    set_mcp_enabled_internal(state, app_ty, id, enabled)
}

fn import_mcp_from_claude_internal(state: &AppState) -> Result<usize, AppError> {
    let mut cfg = state.config.lock()?;
    let changed = mcp::import_from_claude(&mut cfg)?;
    drop(cfg);
    if changed > 0 {
        state.save()?;
    }
    Ok(changed)
}

#[doc(hidden)]
pub fn import_mcp_from_claude_test_hook(state: &AppState) -> Result<usize, AppError> {
    import_mcp_from_claude_internal(state)
}

fn import_mcp_from_codex_internal(state: &AppState) -> Result<usize, AppError> {
    let mut cfg = state.config.lock()?;
    let changed = mcp::import_from_codex(&mut cfg)?;
    drop(cfg);
    if changed > 0 {
        state.save()?;
    }
    Ok(changed)
}

#[doc(hidden)]
pub fn import_mcp_from_codex_test_hook(state: &AppState) -> Result<usize, AppError> {
    import_mcp_from_codex_internal(state)
}
