use std::collections::HashMap;

use serde_json::Value;

use crate::app_config::{AppType, MultiAppConfig};
use crate::error::AppError;
use crate::mcp;
use crate::store::AppState;

/// MCP 相关业务逻辑
pub struct McpService;

impl McpService {
    /// 获取指定应用的 MCP 服务器快照，并在必要时回写归一化后的配置。
    pub fn get_servers(state: &AppState, app: AppType) -> Result<HashMap<String, Value>, AppError> {
        let mut cfg = state.config.write()?;
        let (snapshot, normalized) = mcp::get_servers_snapshot_for(&mut cfg, &app);
        drop(cfg);
        if normalized > 0 {
            state.save()?;
        }
        Ok(snapshot)
    }

    /// 在 config.json 中新增或更新指定 MCP 服务器，并按需同步到对应客户端。
    pub fn upsert_server(
        state: &AppState,
        app: AppType,
        id: &str,
        spec: Value,
        sync_other_side: bool,
    ) -> Result<bool, AppError> {
        let (changed, snapshot, sync_claude, sync_codex): (
            bool,
            Option<MultiAppConfig>,
            bool,
            bool,
        ) = {
            let mut cfg = state.config.write()?;
            let changed = mcp::upsert_in_config_for(&mut cfg, &app, id, spec)?;

            // 修复：默认启用（unwrap_or(true)）
            // 新增的 MCP 如果缺少 enabled 字段，应该默认为启用状态
            let enabled = cfg
                .mcp_for(&app)
                .servers
                .get(id)
                .and_then(|entry| entry.get("enabled"))
                .and_then(|v| v.as_bool())
                .unwrap_or(true);

            let mut sync_claude = matches!(app, AppType::Claude) && enabled;
            let mut sync_codex = matches!(app, AppType::Codex) && enabled;

            // 修复：sync_other_side=true 时，先将 MCP 复制到另一侧，然后强制同步
            // 这才是"同步到另一侧"的正确语义：将 MCP 跨应用复制
            if sync_other_side && app != AppType::Gemini {
                // Gemini 暂不支持跨应用复制，直接跳过
                // 获取当前 MCP 条目的克隆（刚刚插入的，不可能失败）
                let current_entry = cfg
                    .mcp_for(&app)
                    .servers
                    .get(id)
                    .cloned()
                    .expect("刚刚插入的 MCP 条目必定存在");

                // 将该 MCP 复制到另一侧的 servers
                let other_app = match app {
                    AppType::Claude => AppType::Codex,
                    AppType::Codex => AppType::Claude,
                    AppType::Gemini => unreachable!("Gemini 已在外层 if 中跳过"),
                };

                cfg.mcp_for_mut(&other_app)
                    .servers
                    .insert(id.to_string(), current_entry);

                // 强制同步另一侧
                match app {
                    AppType::Claude => sync_codex = true,
                    AppType::Codex => sync_claude = true,
                    AppType::Gemini => unreachable!("Gemini 已在外层 if 中跳过"),
                }
            }

            let snapshot = if sync_claude || sync_codex {
                Some(cfg.clone())
            } else {
                None
            };

            (changed, snapshot, sync_claude, sync_codex)
        };

        // 保持原有行为：始终尝试持久化，避免遗漏 normalize 带来的隐式变更
        state.save()?;

        if let Some(snapshot) = snapshot {
            if sync_claude {
                mcp::sync_enabled_to_claude(&snapshot)?;
            }
            if sync_codex {
                mcp::sync_enabled_to_codex(&snapshot)?;
            }
        }

        Ok(changed)
    }

    /// 删除 config.json 中的 MCP 服务器条目，并同步客户端配置。
    pub fn delete_server(state: &AppState, app: AppType, id: &str) -> Result<bool, AppError> {
        let (existed, snapshot): (bool, Option<MultiAppConfig>) = {
            let mut cfg = state.config.write()?;
            let existed = mcp::delete_in_config_for(&mut cfg, &app, id)?;
            let snapshot = if existed { Some(cfg.clone()) } else { None };
            (existed, snapshot)
        };
        if existed {
            state.save()?;
            if let Some(snapshot) = snapshot {
                match app {
                    AppType::Claude => mcp::sync_enabled_to_claude(&snapshot)?,
                    AppType::Codex => mcp::sync_enabled_to_codex(&snapshot)?,
                    AppType::Gemini => {} // Gemini 暂不支持 MCP 同步
                }
            }
        }
        Ok(existed)
    }

    /// 设置 MCP 启用状态，并同步到客户端配置。
    pub fn set_enabled(
        state: &AppState,
        app: AppType,
        id: &str,
        enabled: bool,
    ) -> Result<bool, AppError> {
        let (existed, snapshot): (bool, Option<MultiAppConfig>) = {
            let mut cfg = state.config.write()?;
            let existed = mcp::set_enabled_flag_for(&mut cfg, &app, id, enabled)?;
            let snapshot = if existed { Some(cfg.clone()) } else { None };
            (existed, snapshot)
        };

        if existed {
            state.save()?;
            if let Some(snapshot) = snapshot {
                match app {
                    AppType::Claude => mcp::sync_enabled_to_claude(&snapshot)?,
                    AppType::Codex => mcp::sync_enabled_to_codex(&snapshot)?,
                    AppType::Gemini => {} // Gemini 暂不支持 MCP 同步
                }
            }
        }
        Ok(existed)
    }

    /// 手动同步已启用的 MCP 服务器到客户端配置。
    pub fn sync_enabled(state: &AppState, app: AppType) -> Result<(), AppError> {
        let (snapshot, normalized): (MultiAppConfig, usize) = {
            let mut cfg = state.config.write()?;
            let normalized = mcp::normalize_servers_for(&mut cfg, &app);
            (cfg.clone(), normalized)
        };
        if normalized > 0 {
            state.save()?;
        }
        match app {
            AppType::Claude => mcp::sync_enabled_to_claude(&snapshot)?,
            AppType::Codex => mcp::sync_enabled_to_codex(&snapshot)?,
            AppType::Gemini => {} // Gemini 暂不支持 MCP 同步
        }
        Ok(())
    }

    /// 从 Claude 客户端配置导入 MCP 定义。
    pub fn import_from_claude(state: &AppState) -> Result<usize, AppError> {
        let mut cfg = state.config.write()?;
        let changed = mcp::import_from_claude(&mut cfg)?;
        drop(cfg);
        if changed > 0 {
            state.save()?;
        }
        Ok(changed)
    }

    /// 从 Codex 客户端配置导入 MCP 定义。
    pub fn import_from_codex(state: &AppState) -> Result<usize, AppError> {
        let mut cfg = state.config.write()?;
        let changed = mcp::import_from_codex(&mut cfg)?;
        drop(cfg);
        if changed > 0 {
            state.save()?;
        }
        Ok(changed)
    }
}
