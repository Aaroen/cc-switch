use std::path::PathBuf;

use crate::app_config::AppType;
use crate::codex_config::get_codex_auth_path;
use crate::config::get_claude_settings_path;
use crate::error::AppError;
use crate::gemini_config::get_gemini_dir;

/// 返回指定应用所使用的提示词文件路径。
pub fn prompt_file_path(app: &AppType) -> Result<PathBuf, AppError> {
    let base_dir = match app {
        AppType::Claude => get_claude_settings_path()
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .expect("无法获取用户目录")
                    .join(".claude")
            }),
        AppType::Codex => get_codex_auth_path()
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .expect("无法获取用户目录")
                    .join(".codex")
            }),
        AppType::Gemini => get_gemini_dir(),
    };

    let filename = match app {
        AppType::Claude => "CLAUDE.md",
        AppType::Codex => "AGENTS.md",
        AppType::Gemini => "GEMINI.md",
    };

    Ok(base_dir.join(filename))
}
