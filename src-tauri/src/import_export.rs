use crate::app_config::{AppType, MultiAppConfig};
use crate::error::AppError;
use crate::provider::Provider;
use chrono::Utc;
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

// 默认仅保留最近 10 份备份，避免目录无限膨胀
const MAX_BACKUPS: usize = 10;

/// 创建配置文件备份
pub fn create_backup(config_path: &PathBuf) -> Result<String, AppError> {
    if !config_path.exists() {
        return Ok(String::new());
    }

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let backup_id = format!("backup_{}", timestamp);

    let backup_dir = config_path
        .parent()
        .ok_or_else(|| AppError::Config("Invalid config path".into()))?
        .join("backups");

    // 创建备份目录
    fs::create_dir_all(&backup_dir).map_err(|e| AppError::io(&backup_dir, e))?;

    let backup_path = backup_dir.join(format!("{}.json", backup_id));

    // 复制配置文件到备份
    fs::copy(config_path, &backup_path).map_err(|e| AppError::io(&backup_path, e))?;

    // 备份完成后清理旧的备份文件（仅保留最近 MAX_BACKUPS 份）
    cleanup_old_backups(&backup_dir, MAX_BACKUPS)?;

    Ok(backup_id)
}

fn cleanup_old_backups(backup_dir: &PathBuf, retain: usize) -> Result<(), AppError> {
    if retain == 0 {
        return Ok(());
    }

    let mut entries: Vec<_> = match fs::read_dir(backup_dir) {
        Ok(iter) => iter
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .map(|ext| ext == "json")
                    .unwrap_or(false)
            })
            .collect(),
        Err(_) => return Ok(()),
    };

    if entries.len() <= retain {
        return Ok(());
    }

    let remove_count = entries.len().saturating_sub(retain);

    entries.sort_by(|a, b| {
        let a_time = a.metadata().and_then(|m| m.modified()).ok();
        let b_time = b.metadata().and_then(|m| m.modified()).ok();
        a_time.cmp(&b_time)
    });

    for entry in entries.into_iter().take(remove_count) {
        if let Err(err) = fs::remove_file(entry.path()) {
            log::warn!(
                "Failed to remove old backup {}: {}",
                entry.path().display(),
                err
            );
        }
    }

    Ok(())
}

fn sync_current_providers_to_live(config: &mut MultiAppConfig) -> Result<(), AppError> {
    sync_current_provider_for_app(config, &AppType::Claude)?;
    sync_current_provider_for_app(config, &AppType::Codex)?;
    Ok(())
}

fn sync_current_provider_for_app(
    config: &mut MultiAppConfig,
    app_type: &AppType,
) -> Result<(), AppError> {
    let (current_id, provider) = {
        let manager = match config.get_manager(app_type) {
            Some(manager) => manager,
            None => return Ok(()),
        };

        if manager.current.is_empty() {
            return Ok(());
        }

        let current_id = manager.current.clone();
        let provider = match manager.providers.get(&current_id) {
            Some(provider) => provider.clone(),
            None => {
                log::warn!(
                    "当前应用 {:?} 的供应商 {} 不存在，跳过 live 同步",
                    app_type,
                    current_id
                );
                return Ok(());
            }
        };
        (current_id, provider)
    };

    match app_type {
        AppType::Codex => sync_codex_live(config, &current_id, &provider)?,
        AppType::Claude => sync_claude_live(config, &current_id, &provider)?,
    }

    Ok(())
}

fn sync_codex_live(
    config: &mut MultiAppConfig,
    provider_id: &str,
    provider: &Provider,
) -> Result<(), AppError> {
    use serde_json::Value;

    let settings = provider
        .settings_config
        .as_object()
        .ok_or_else(|| {
            AppError::Config(format!(
                "供应商 {} 的 Codex 配置必须是对象",
                provider_id
            ))
        })?;
    let auth = settings
        .get("auth")
        .ok_or_else(|| {
            AppError::Config(format!(
                "供应商 {} 的 Codex 配置缺少 auth 字段",
                provider_id
            ))
        })?;
    if !auth.is_object() {
        return Err(AppError::Config(format!(
            "供应商 {} 的 Codex auth 配置必须是 JSON 对象",
            provider_id
        )));
    }
    let cfg_text = settings.get("config").and_then(Value::as_str);

    crate::codex_config::write_codex_live_atomic(auth, cfg_text)?;
    crate::mcp::sync_enabled_to_codex(config).map_err(AppError::Message)?;

    let cfg_text_after = crate::codex_config::read_and_validate_codex_config_text()?;
    if let Some(manager) = config.get_manager_mut(&AppType::Codex) {
        if let Some(target) = manager.providers.get_mut(provider_id) {
            if let Some(obj) = target.settings_config.as_object_mut() {
                obj.insert(
                    "config".to_string(),
                    serde_json::Value::String(cfg_text_after),
                );
            }
        }
    }

    Ok(())
}

fn sync_claude_live(
    config: &mut MultiAppConfig,
    provider_id: &str,
    provider: &Provider,
) -> Result<(), AppError> {
    use crate::config::{read_json_file, write_json_file};

    let settings_path = crate::config::get_claude_settings_path();
    if let Some(parent) = settings_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
    }

    write_json_file(&settings_path, &provider.settings_config)?;

    let live_after = read_json_file::<serde_json::Value>(&settings_path)?;
    if let Some(manager) = config.get_manager_mut(&AppType::Claude) {
        if let Some(target) = manager.providers.get_mut(provider_id) {
            target.settings_config = live_after;
        }
    }

    Ok(())
}

/// 导出配置文件
#[tauri::command]
pub async fn export_config_to_file(file_path: String) -> Result<Value, String> {
    // 读取当前配置文件
    let config_path = crate::config::get_app_config_path();
    let config_content = fs::read_to_string(&config_path)
        .map_err(|e| AppError::io(&config_path, e).to_string())?;

    // 写入到指定文件
    fs::write(&file_path, &config_content)
        .map_err(|e| AppError::io(&file_path, e).to_string())?;

    Ok(json!({
        "success": true,
        "message": "Configuration exported successfully",
        "filePath": file_path
    }))
}

/// 从文件导入配置
#[tauri::command]
pub async fn import_config_from_file(
    file_path: String,
    state: tauri::State<'_, crate::store::AppState>,
) -> Result<Value, String> {
    // 读取导入的文件
    let file_path_ref = std::path::Path::new(&file_path);
    let import_content = fs::read_to_string(file_path_ref)
        .map_err(|e| AppError::io(file_path_ref, e).to_string())?;

    // 验证并解析为配置对象
    let new_config: crate::app_config::MultiAppConfig =
        serde_json::from_str(&import_content)
            .map_err(|e| AppError::json(file_path_ref, e).to_string())?;

    // 备份当前配置
    let config_path = crate::config::get_app_config_path();
    let backup_id = create_backup(&config_path).map_err(|e| e.to_string())?;

    // 写入新配置到磁盘
    fs::write(&config_path, &import_content)
        .map_err(|e| AppError::io(&config_path, e).to_string())?;

    // 更新内存中的状态
    {
        let mut config_state = state
            .config
            .lock()
            .map_err(|e| AppError::from(e).to_string())?;
        *config_state = new_config;
    }

    Ok(json!({
        "success": true,
        "message": "Configuration imported successfully",
        "backupId": backup_id
    }))
}

/// 同步当前供应商配置到对应的 live 文件
#[tauri::command]
pub async fn sync_current_providers_live(
    state: tauri::State<'_, crate::store::AppState>,
) -> Result<Value, String> {
    {
        let mut config_state = state
            .config
            .lock()
            .map_err(|e| AppError::from(e).to_string())?;
        sync_current_providers_to_live(&mut config_state).map_err(|e| e.to_string())?;
    }

    Ok(json!({
        "success": true,
        "message": "Live configuration synchronized"
    }))
}

/// 保存文件对话框
#[tauri::command]
pub async fn save_file_dialog<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    default_name: String,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    let dialog = app.dialog();
    let result = dialog
        .file()
        .add_filter("JSON", &["json"])
        .set_file_name(&default_name)
        .blocking_save_file();

    Ok(result.map(|p| p.to_string()))
}

/// 打开文件对话框
#[tauri::command]
pub async fn open_file_dialog<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    let dialog = app.dialog();
    let result = dialog
        .file()
        .add_filter("JSON", &["json"])
        .blocking_pick_file();

    Ok(result.map(|p| p.to_string()))
}
