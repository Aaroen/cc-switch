#![allow(non_snake_case)]

use std::collections::HashMap;

use serde::Serialize;
use tauri::State;

use crate::app_config::AppType;
use crate::claude_mcp;
use crate::services::McpService;
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
    let app_ty = AppType::from(app.as_deref().unwrap_or("claude"));
    let servers = McpService::get_servers(&state, app_ty).map_err(|e| e.to_string())?;
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
    let app_ty = AppType::from(app.as_deref().unwrap_or("claude"));
    McpService::upsert_server(&state, app_ty, &id, spec, sync_other_side.unwrap_or(false))
        .map_err(|e| e.to_string())
}

/// 在 config.json 中删除一个 MCP 服务器定义
#[tauri::command]
pub async fn delete_mcp_server_in_config(
    state: State<'_, AppState>,
    app: Option<String>,
    id: String,
) -> Result<bool, String> {
    let app_ty = AppType::from(app.as_deref().unwrap_or("claude"));
    McpService::delete_server(&state, app_ty, &id).map_err(|e| e.to_string())
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
    McpService::set_enabled(&state, app_ty, &id, enabled).map_err(|e| e.to_string())
}

/// 手动同步：将启用的 MCP 投影到 ~/.claude.json
#[tauri::command]
pub async fn sync_enabled_mcp_to_claude(state: State<'_, AppState>) -> Result<bool, String> {
    McpService::sync_enabled(&state, AppType::Claude)
        .map(|_| true)
        .map_err(|e| e.to_string())
}

/// 手动同步：将启用的 MCP 投影到 ~/.codex/config.toml
#[tauri::command]
pub async fn sync_enabled_mcp_to_codex(state: State<'_, AppState>) -> Result<bool, String> {
    McpService::sync_enabled(&state, AppType::Codex)
        .map(|_| true)
        .map_err(|e| e.to_string())
}

/// 从 ~/.claude.json 导入 MCP 定义到 config.json
#[tauri::command]
pub async fn import_mcp_from_claude(state: State<'_, AppState>) -> Result<usize, String> {
    McpService::import_from_claude(&state).map_err(|e| e.to_string())
}

/// 从 ~/.codex/config.toml 导入 MCP 定义到 config.json
#[tauri::command]
pub async fn import_mcp_from_codex(state: State<'_, AppState>) -> Result<usize, String> {
    McpService::import_from_codex(&state).map_err(|e| e.to_string())
}
