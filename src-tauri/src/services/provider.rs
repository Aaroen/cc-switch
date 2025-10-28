use serde_json::{json, Value};

use crate::app_config::{AppType, MultiAppConfig};
use crate::codex_config::{get_codex_auth_path, get_codex_config_path, write_codex_live_atomic};
use crate::config::{
    delete_file, get_claude_settings_path, get_provider_config_path, read_json_file,
    write_json_file,
};
use crate::error::AppError;
use crate::mcp;

/// 供应商相关业务逻辑
pub struct ProviderService;

impl ProviderService {
    /// 切换指定应用的供应商
    pub fn switch(
        config: &mut MultiAppConfig,
        app_type: AppType,
        provider_id: &str,
    ) -> Result<(), AppError> {
        match app_type {
            AppType::Codex => Self::switch_codex(config, provider_id),
            AppType::Claude => Self::switch_claude(config, provider_id),
        }
    }

    fn switch_codex(config: &mut MultiAppConfig, provider_id: &str) -> Result<(), AppError> {
        let provider = config
            .get_manager(&AppType::Codex)
            .ok_or_else(|| AppError::Message("应用类型不存在: Codex".into()))?
            .providers
            .get(provider_id)
            .cloned()
            .ok_or_else(|| AppError::ProviderNotFound(provider_id.to_string()))?;

        Self::backfill_codex_current(config, provider_id)?;
        Self::write_codex_live(&provider)?;

        if let Some(manager) = config.get_manager_mut(&AppType::Codex) {
            manager.current = provider_id.to_string();
        }

        // 同步启用的 MCP 服务器
        mcp::sync_enabled_to_codex(config)?;

        // 更新持久化快照
        let cfg_text_after = crate::codex_config::read_and_validate_codex_config_text()?;
        if let Some(manager) = config.get_manager_mut(&AppType::Codex) {
            if let Some(target) = manager.providers.get_mut(provider_id) {
                if let Some(obj) = target.settings_config.as_object_mut() {
                    obj.insert("config".to_string(), Value::String(cfg_text_after));
                }
            }
        }

        Ok(())
    }

    fn backfill_codex_current(
        config: &mut MultiAppConfig,
        next_provider: &str,
    ) -> Result<(), AppError> {
        let current_id = config
            .get_manager(&AppType::Codex)
            .map(|m| m.current.clone())
            .unwrap_or_default();

        if current_id.is_empty() || current_id == next_provider {
            return Ok(());
        }

        let auth_path = get_codex_auth_path();
        if !auth_path.exists() {
            return Ok(());
        }

        let auth: Value = read_json_file(&auth_path)?;
        let config_path = get_codex_config_path();
        let config_text = if config_path.exists() {
            std::fs::read_to_string(&config_path).map_err(|e| AppError::io(&config_path, e))?
        } else {
            String::new()
        };

        let live = json!({
            "auth": auth,
            "config": config_text,
        });

        if let Some(manager) = config.get_manager_mut(&AppType::Codex) {
            if let Some(current) = manager.providers.get_mut(&current_id) {
                current.settings_config = live;
            }
        }

        Ok(())
    }

    fn write_codex_live(provider: &crate::provider::Provider) -> Result<(), AppError> {
        let settings = provider
            .settings_config
            .as_object()
            .ok_or_else(|| AppError::Config("Codex 配置必须是 JSON 对象".into()))?;
        let auth = settings
            .get("auth")
            .ok_or_else(|| AppError::Config(format!("供应商 {} 缺少 auth 配置", provider.id)))?;
        if !auth.is_object() {
            return Err(AppError::Config(format!(
                "供应商 {} 的 auth 必须是对象",
                provider.id
            )));
        }
        let cfg_text = settings.get("config").and_then(Value::as_str);

        write_codex_live_atomic(auth, cfg_text)?;
        Ok(())
    }

    fn switch_claude(config: &mut MultiAppConfig, provider_id: &str) -> Result<(), AppError> {
        let provider = {
            let manager = config
                .get_manager(&AppType::Claude)
                .ok_or_else(|| AppError::Message("应用类型不存在: Claude".into()))?;
            manager
                .providers
                .get(provider_id)
                .cloned()
                .ok_or_else(|| AppError::ProviderNotFound(provider_id.to_string()))?
        };

        Self::backfill_claude_current(config, provider_id)?;
        Self::write_claude_live(&provider)?;

        if let Some(manager) = config.get_manager_mut(&AppType::Claude) {
            manager.current = provider_id.to_string();

            if let Some(target) = manager.providers.get_mut(provider_id) {
                let settings_path = get_claude_settings_path();
                let live_after = read_json_file::<Value>(&settings_path)?;
                target.settings_config = live_after;
            }
        }

        Ok(())
    }

    fn backfill_claude_current(
        config: &mut MultiAppConfig,
        next_provider: &str,
    ) -> Result<(), AppError> {
        let settings_path = get_claude_settings_path();
        if !settings_path.exists() {
            return Ok(());
        }

        let current_id = config
            .get_manager(&AppType::Claude)
            .map(|m| m.current.clone())
            .unwrap_or_default();
        if current_id.is_empty() || current_id == next_provider {
            return Ok(());
        }

        let live = read_json_file::<Value>(&settings_path)?;
        if let Some(manager) = config.get_manager_mut(&AppType::Claude) {
            if let Some(current) = manager.providers.get_mut(&current_id) {
                current.settings_config = live;
            }
        }

        Ok(())
    }

    fn write_claude_live(provider: &crate::provider::Provider) -> Result<(), AppError> {
        let settings_path = get_claude_settings_path();
        if let Some(parent) = settings_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
        }

        write_json_file(&settings_path, &provider.settings_config)?;
        Ok(())
    }

    pub fn delete(
        config: &mut MultiAppConfig,
        app_type: AppType,
        provider_id: &str,
    ) -> Result<(), AppError> {
        let current_matches = config
            .get_manager(&app_type)
            .map(|m| m.current == provider_id)
            .unwrap_or(false);
        if current_matches {
            return Err(AppError::Config("不能删除当前正在使用的供应商".into()));
        }

        let provider = config
            .get_manager(&app_type)
            .ok_or_else(|| AppError::Message(format!("应用类型不存在: {:?}", app_type)))?
            .providers
            .get(provider_id)
            .cloned()
            .ok_or_else(|| AppError::ProviderNotFound(provider_id.to_string()))?;

        match app_type {
            AppType::Codex => {
                crate::codex_config::delete_codex_provider_config(provider_id, &provider.name)?;
            }
            AppType::Claude => {
                let by_name = get_provider_config_path(provider_id, Some(&provider.name));
                let by_id = get_provider_config_path(provider_id, None);
                delete_file(&by_name)?;
                delete_file(&by_id)?;
            }
        }

        if let Some(manager) = config.get_manager_mut(&app_type) {
            manager.providers.remove(provider_id);
        }

        Ok(())
    }
}
