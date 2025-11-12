use super::provider::ProviderService;
use crate::app_config::{AppType, MultiAppConfig};
use crate::error::AppError;
use crate::provider::Provider;
use crate::store::AppState;
use chrono::Utc;
use serde_json::Value;
use std::fs;
use std::path::Path;

const MAX_BACKUPS: usize = 10;

/// 配置导入导出相关业务逻辑
pub struct ConfigService;

impl ConfigService {
    /// 为当前 config.json 创建备份，返回备份 ID（若文件不存在则返回空字符串）。
    pub fn create_backup(config_path: &Path) -> Result<String, AppError> {
        if !config_path.exists() {
            return Ok(String::new());
        }

        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let backup_id = format!("backup_{timestamp}");

        let backup_dir = config_path
            .parent()
            .ok_or_else(|| AppError::Config("Invalid config path".into()))?
            .join("backups");

        fs::create_dir_all(&backup_dir).map_err(|e| AppError::io(&backup_dir, e))?;

        let backup_path = backup_dir.join(format!("{backup_id}.json"));
        let contents = fs::read(config_path).map_err(|e| AppError::io(config_path, e))?;
        fs::write(&backup_path, contents).map_err(|e| AppError::io(&backup_path, e))?;

        Self::cleanup_old_backups(&backup_dir, MAX_BACKUPS)?;

        Ok(backup_id)
    }

    fn cleanup_old_backups(backup_dir: &Path, retain: usize) -> Result<(), AppError> {
        if retain == 0 {
            return Ok(());
        }

        let entries = match fs::read_dir(backup_dir) {
            Ok(iter) => iter
                .filter_map(|entry| entry.ok())
                .filter(|entry| {
                    entry
                        .path()
                        .extension()
                        .map(|ext| ext == "json")
                        .unwrap_or(false)
                })
                .collect::<Vec<_>>(),
            Err(_) => return Ok(()),
        };

        if entries.len() <= retain {
            return Ok(());
        }

        let remove_count = entries.len().saturating_sub(retain);
        let mut sorted = entries;

        sorted.sort_by(|a, b| {
            let a_time = a.metadata().and_then(|m| m.modified()).ok();
            let b_time = b.metadata().and_then(|m| m.modified()).ok();
            a_time.cmp(&b_time)
        });

        for entry in sorted.into_iter().take(remove_count) {
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

    /// 将当前 config.json 拷贝到目标路径。
    pub fn export_config_to_path(target_path: &Path) -> Result<(), AppError> {
        let config_path = crate::config::get_app_config_path();
        let config_content =
            fs::read_to_string(&config_path).map_err(|e| AppError::io(&config_path, e))?;
        fs::write(target_path, config_content).map_err(|e| AppError::io(target_path, e))
    }

    /// 从磁盘文件加载配置并写回 config.json，返回备份 ID 及新配置。
    pub fn load_config_for_import(file_path: &Path) -> Result<(MultiAppConfig, String), AppError> {
        let import_content =
            fs::read_to_string(file_path).map_err(|e| AppError::io(file_path, e))?;

        let new_config: MultiAppConfig =
            serde_json::from_str(&import_content).map_err(|e| AppError::json(file_path, e))?;

        let config_path = crate::config::get_app_config_path();
        let backup_id = Self::create_backup(&config_path)?;

        fs::write(&config_path, &import_content).map_err(|e| AppError::io(&config_path, e))?;

        Ok((new_config, backup_id))
    }

    /// 将外部配置文件内容加载并写入应用状态。
    pub fn import_config_from_path(file_path: &Path, state: &AppState) -> Result<String, AppError> {
        let (new_config, backup_id) = Self::load_config_for_import(file_path)?;

        {
            let mut guard = state.config.write().map_err(AppError::from)?;
            *guard = new_config;
        }

        Ok(backup_id)
    }

    /// 同步当前供应商到对应的 live 配置。
    pub fn sync_current_providers_to_live(config: &mut MultiAppConfig) -> Result<(), AppError> {
        Self::sync_current_provider_for_app(config, &AppType::Claude)?;
        Self::sync_current_provider_for_app(config, &AppType::Codex)?;
        Self::sync_current_provider_for_app(config, &AppType::Gemini)?;
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
                        "当前应用 {app_type:?} 的供应商 {current_id} 不存在，跳过 live 同步"
                    );
                    return Ok(());
                }
            };
            (current_id, provider)
        };

        match app_type {
            AppType::Codex => Self::sync_codex_live(config, &current_id, &provider)?,
            AppType::Claude => Self::sync_claude_live(config, &current_id, &provider)?,
            AppType::Gemini => Self::sync_gemini_live(config, &current_id, &provider)?,
        }

        Ok(())
    }

    fn sync_codex_live(
        config: &mut MultiAppConfig,
        provider_id: &str,
        provider: &Provider,
    ) -> Result<(), AppError> {
        let settings = provider.settings_config.as_object().ok_or_else(|| {
            AppError::Config(format!("供应商 {provider_id} 的 Codex 配置必须是对象"))
        })?;
        let auth = settings.get("auth").ok_or_else(|| {
            AppError::Config(format!("供应商 {provider_id} 的 Codex 配置缺少 auth 字段"))
        })?;
        if !auth.is_object() {
            return Err(AppError::Config(format!(
                "供应商 {provider_id} 的 Codex auth 配置必须是 JSON 对象"
            )));
        }
        let cfg_text = settings.get("config").and_then(Value::as_str);

        crate::codex_config::write_codex_live_atomic(auth, cfg_text)?;
        crate::mcp::sync_enabled_to_codex(config)?;

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
            fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
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

    fn sync_gemini_live(
        config: &mut MultiAppConfig,
        provider_id: &str,
        provider: &Provider,
    ) -> Result<(), AppError> {
        use crate::gemini_config::{
            env_to_json, json_to_env, read_gemini_env, write_gemini_env_atomic,
        };

        let env_path = crate::gemini_config::get_gemini_env_path();
        if let Some(parent) = env_path.parent() {
            fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
        }

        // 转换 JSON 配置为 .env 格式
        let env_map = json_to_env(&provider.settings_config)?;

        // Google 官方（OAuth）: env 为空，写入空文件并设置安全标志后返回
        if env_map.is_empty() {
            write_gemini_env_atomic(&env_map)?;
            ProviderService::ensure_google_oauth_security_flag(provider)?;

            let live_after_env = read_gemini_env()?;
            let live_after = env_to_json(&live_after_env);

            if let Some(manager) = config.get_manager_mut(&AppType::Gemini) {
                if let Some(target) = manager.providers.get_mut(provider_id) {
                    target.settings_config = live_after;
                }
            }

            return Ok(());
        }

        // 非 OAuth：按常规写入，并在必要时设置 Packycode 安全标志
        write_gemini_env_atomic(&env_map)?;
        ProviderService::ensure_packycode_security_flag(provider)?;

        // 读回实际写入的内容并更新到配置中
        let live_after_env = read_gemini_env()?;
        let live_after = env_to_json(&live_after_env);

        if let Some(manager) = config.get_manager_mut(&AppType::Gemini) {
            if let Some(target) = manager.providers.get_mut(provider_id) {
                target.settings_config = live_after;
            }
        }

        Ok(())
    }
}
