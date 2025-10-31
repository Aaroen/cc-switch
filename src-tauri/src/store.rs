use crate::app_config::MultiAppConfig;
use crate::error::AppError;
use std::sync::RwLock;

/// 全局应用状态
pub struct AppState {
    pub config: RwLock<MultiAppConfig>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    /// 创建新的应用状态
    pub fn new() -> Self {
        let config = MultiAppConfig::load().unwrap_or_else(|e| {
            log::warn!("加载配置失败: {}, 使用默认配置", e);
            MultiAppConfig::default()
        });

        Self {
            config: RwLock::new(config),
        }
    }

    /// 保存配置到文件
    pub fn save(&self) -> Result<(), AppError> {
        let config = self.config.read().map_err(AppError::from)?;

        config.save()
    }
}
