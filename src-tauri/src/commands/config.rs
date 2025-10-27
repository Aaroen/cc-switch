#![allow(non_snake_case)]

use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

use crate::app_config::AppType;
use crate::codex_config;
use crate::config::{self, get_claude_settings_path, ConfigStatus};

/// 获取 Claude Code 配置状态
#[tauri::command]
pub async fn get_claude_config_status() -> Result<ConfigStatus, String> {
    Ok(config::get_claude_config_status())
}

/// 获取应用配置状态
#[tauri::command]
pub async fn get_config_status(
    app_type: Option<AppType>,
    app: Option<String>,
    appType: Option<String>,
) -> Result<ConfigStatus, String> {
    let app = app_type
        .or_else(|| app.as_deref().map(|s| s.into()))
        .or_else(|| appType.as_deref().map(|s| s.into()))
        .unwrap_or(AppType::Claude);

    match app {
        AppType::Claude => Ok(config::get_claude_config_status()),
        AppType::Codex => {
            let auth_path = codex_config::get_codex_auth_path();
            let exists = auth_path.exists();
            let path = codex_config::get_codex_config_dir()
                .to_string_lossy()
                .to_string();

            Ok(ConfigStatus { exists, path })
        }
    }
}

/// 获取 Claude Code 配置文件路径
#[tauri::command]
pub async fn get_claude_code_config_path() -> Result<String, String> {
    Ok(get_claude_settings_path().to_string_lossy().to_string())
}

/// 获取当前生效的配置目录
#[tauri::command]
pub async fn get_config_dir(
    app_type: Option<AppType>,
    app: Option<String>,
    appType: Option<String>,
) -> Result<String, String> {
    let app = app_type
        .or_else(|| app.as_deref().map(|s| s.into()))
        .or_else(|| appType.as_deref().map(|s| s.into()))
        .unwrap_or(AppType::Claude);

    let dir = match app {
        AppType::Claude => config::get_claude_config_dir(),
        AppType::Codex => codex_config::get_codex_config_dir(),
    };

    Ok(dir.to_string_lossy().to_string())
}

/// 打开配置文件夹
#[tauri::command]
pub async fn open_config_folder(
    handle: AppHandle,
    app_type: Option<AppType>,
    app: Option<String>,
    appType: Option<String>,
) -> Result<bool, String> {
    let app_type = app_type
        .or_else(|| app.as_deref().map(|s| s.into()))
        .or_else(|| appType.as_deref().map(|s| s.into()))
        .unwrap_or(AppType::Claude);

    let config_dir = match app_type {
        AppType::Claude => config::get_claude_config_dir(),
        AppType::Codex => codex_config::get_codex_config_dir(),
    };

    if !config_dir.exists() {
        std::fs::create_dir_all(&config_dir).map_err(|e| format!("创建目录失败: {}", e))?;
    }

    handle
        .opener()
        .open_path(config_dir.to_string_lossy().to_string(), None::<String>)
        .map_err(|e| format!("打开文件夹失败: {}", e))?;

    Ok(true)
}

/// 弹出系统目录选择器并返回用户选择的路径
#[tauri::command]
pub async fn pick_directory(
    app: AppHandle,
    default_path: Option<String>,
) -> Result<Option<String>, String> {
    let initial = default_path
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty());

    let result = tauri::async_runtime::spawn_blocking(move || {
        let mut builder = app.dialog().file();
        if let Some(path) = initial {
            builder = builder.set_directory(path);
        }
        builder.blocking_pick_folder()
    })
    .await
    .map_err(|e| format!("弹出目录选择器失败: {}", e))?;

    match result {
        Some(file_path) => {
            let resolved = file_path
                .simplified()
                .into_path()
                .map_err(|e| format!("解析选择的目录失败: {}", e))?;
            Ok(Some(resolved.to_string_lossy().to_string()))
        }
        None => Ok(None),
    }
}

/// 获取应用配置文件路径
#[tauri::command]
pub async fn get_app_config_path() -> Result<String, String> {
    let config_path = config::get_app_config_path();
    Ok(config_path.to_string_lossy().to_string())
}

/// 打开应用配置文件夹
#[tauri::command]
pub async fn open_app_config_folder(handle: AppHandle) -> Result<bool, String> {
    let config_dir = config::get_app_config_dir();

    if !config_dir.exists() {
        std::fs::create_dir_all(&config_dir).map_err(|e| format!("创建目录失败: {}", e))?;
    }

    handle
        .opener()
        .open_path(config_dir.to_string_lossy().to_string(), None::<String>)
        .map_err(|e| format!("打开文件夹失败: {}", e))?;

    Ok(true)
}
