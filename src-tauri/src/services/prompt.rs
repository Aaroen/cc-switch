use std::collections::HashMap;

use crate::app_config::AppType;
use crate::config::write_text_file;
use crate::error::AppError;
use crate::prompt::Prompt;
use crate::prompt_files::prompt_file_path;
use crate::store::AppState;

pub struct PromptService;

impl PromptService {
    pub fn get_prompts(
        state: &AppState,
        app: AppType,
    ) -> Result<HashMap<String, Prompt>, AppError> {
        let cfg = state.config.read()?;
        let prompts = match app {
            AppType::Claude => &cfg.prompts.claude.prompts,
            AppType::Codex => &cfg.prompts.codex.prompts,
            AppType::Gemini => &cfg.prompts.gemini.prompts,
        };
        Ok(prompts.clone())
    }

    pub fn upsert_prompt(
        state: &AppState,
        app: AppType,
        id: &str,
        prompt: Prompt,
    ) -> Result<(), AppError> {
        // 检查是否为已启用的提示词
        let is_enabled = prompt.enabled;

        let mut cfg = state.config.write()?;
        let prompts = match app {
            AppType::Claude => &mut cfg.prompts.claude.prompts,
            AppType::Codex => &mut cfg.prompts.codex.prompts,
            AppType::Gemini => &mut cfg.prompts.gemini.prompts,
        };
        prompts.insert(id.to_string(), prompt.clone());
        drop(cfg);
        state.save()?;

        // 如果是已启用的提示词，同步更新到对应的文件
        if is_enabled {
            let target_path = prompt_file_path(&app)?;
            write_text_file(&target_path, &prompt.content)?;
        }

        Ok(())
    }

    pub fn delete_prompt(state: &AppState, app: AppType, id: &str) -> Result<(), AppError> {
        let mut cfg = state.config.write()?;
        let prompts = match app {
            AppType::Claude => &mut cfg.prompts.claude.prompts,
            AppType::Codex => &mut cfg.prompts.codex.prompts,
            AppType::Gemini => &mut cfg.prompts.gemini.prompts,
        };

        if let Some(prompt) = prompts.get(id) {
            if prompt.enabled {
                return Err(AppError::InvalidInput(
                    "无法删除已启用的提示词".to_string(),
                ));
            }
        }

        prompts.remove(id);
        drop(cfg);
        state.save()?;
        Ok(())
    }

    pub fn enable_prompt(state: &AppState, app: AppType, id: &str) -> Result<(), AppError> {
        // 先保存当前文件内容（如果存在且没有对应的提示词）
        let target_path = prompt_file_path(&app)?;
        if target_path.exists() {
            let mut cfg = state.config.write()?;
            let prompts = match app {
                AppType::Claude => &mut cfg.prompts.claude.prompts,
                AppType::Codex => &mut cfg.prompts.codex.prompts,
                AppType::Gemini => &mut cfg.prompts.gemini.prompts,
            };

            // 检查是否有已启用的提示词
            let has_enabled = prompts.values().any(|p| p.enabled);

            // 如果没有已启用的提示词，自动保存当前文件
            if !has_enabled {
                if let Ok(content) = std::fs::read_to_string(&target_path) {
                    if !content.trim().is_empty() {
                        // 检查是否已存在相同内容的提示词，避免重复备份
                        let content_exists = prompts.values().any(|p| p.content.trim() == content.trim());
                        
                        if !content_exists {
                            let timestamp = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs() as i64;
                            let backup_id = format!("backup-{timestamp}");
                            let backup_prompt = Prompt {
                                id: backup_id.clone(),
                                name: format!("原始提示词 {}", chrono::Local::now().format("%Y-%m-%d %H:%M")),
                                content,
                                description: Some("自动备份的原始提示词".to_string()),
                                enabled: false,
                                created_at: Some(timestamp),
                                updated_at: Some(timestamp),
                            };
                            prompts.insert(backup_id, backup_prompt);
                        }
                    }
                }
            }
            drop(cfg);
        }

        // 启用目标提示词
        let mut cfg = state.config.write()?;
        let prompts = match app {
            AppType::Claude => &mut cfg.prompts.claude.prompts,
            AppType::Codex => &mut cfg.prompts.codex.prompts,
            AppType::Gemini => &mut cfg.prompts.gemini.prompts,
        };

        for prompt in prompts.values_mut() {
            prompt.enabled = false;
        }

        if let Some(prompt) = prompts.get_mut(id) {
            prompt.enabled = true;
            write_text_file(&target_path, &prompt.content)?;
        } else {
            return Err(AppError::InvalidInput(format!("提示词 {id} 不存在")));
        }

        drop(cfg);
        state.save()?;
        Ok(())
    }

    pub fn import_from_file(state: &AppState, app: AppType) -> Result<String, AppError> {
        let file_path = prompt_file_path(&app)?;

        if !file_path.exists() {
            return Err(AppError::Message("提示词文件不存在".to_string()));
        }

        let content = std::fs::read_to_string(&file_path).map_err(|e| AppError::io(&file_path, e))?;
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let id = format!("imported-{timestamp}");
        let prompt = Prompt {
            id: id.clone(),
            name: format!(
                "导入的提示词 {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M")
            ),
            content,
            description: Some("从现有配置文件导入".to_string()),
            enabled: false,
            created_at: Some(timestamp),
            updated_at: Some(timestamp),
        };

        Self::upsert_prompt(state, app, &id, prompt)?;
        Ok(id)
    }

    pub fn get_current_file_content(app: AppType) -> Result<Option<String>, AppError> {
        let file_path = prompt_file_path(&app)?;
        if !file_path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&file_path).map_err(|e| AppError::io(&file_path, e))?;
        Ok(Some(content))
    }
}
