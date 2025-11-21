use crate::error::AppError;
use auto_launch::AutoLaunch;

/// 初始化 AutoLaunch 实例
fn get_auto_launch() -> Result<AutoLaunch, AppError> {
    let app_name = "CC Switch";
    let app_path = std::env::current_exe().map_err(AppError::AutoLaunchPathError)?;

    let auto_launch = AutoLaunch::new(app_name, &app_path.to_string_lossy(), false, &[] as &[&str]);
    Ok(auto_launch)
}

/// 启用开机自启
pub fn enable_auto_launch() -> Result<(), AppError> {
    let auto_launch = get_auto_launch()?;
    auto_launch
        .enable()
        .map_err(|e| AppError::AutoLaunchEnableError(e.to_string()))?;
    log::info!("Auto-launch enabled");
    Ok(())
}

/// 禁用开机自启
pub fn disable_auto_launch() -> Result<(), AppError> {
    let auto_launch = get_auto_launch()?;
    auto_launch
        .disable()
        .map_err(|e| AppError::AutoLaunchDisableError(e.to_string()))?;
    log::info!("Auto-launch disabled");
    Ok(())
}

/// 检查是否已启用开机自启
pub fn is_auto_launch_enabled() -> Result<bool, AppError> {
    let auto_launch = get_auto_launch()?;
    auto_launch
        .is_enabled()
        .map_err(|e| AppError::AutoLaunchCheckError(e.to_string()))
}
