use std::fs;
use std::path::Path;
use serde_json::json;
use std::sync::{Mutex, OnceLock};

use cc_switch_lib::{
    create_backup, get_claude_settings_path, read_json_file, sync_current_providers_to_live,
    update_settings, AppSettings, AppType, MultiAppConfig, Provider,
};

fn ensure_test_home() -> &'static Path {
    static HOME: OnceLock<std::path::PathBuf> = OnceLock::new();
    HOME.get_or_init(|| {
        let base = std::env::temp_dir().join("cc-switch-test-home");
        if base.exists() {
            let _ = std::fs::remove_dir_all(&base);
        }
        std::fs::create_dir_all(&base).expect("create test home");
        std::env::set_var("HOME", &base);
        #[cfg(windows)]
        std::env::set_var("USERPROFILE", &base);
        base
    })
    .as_path()
}

fn reset_test_fs() {
    let home = ensure_test_home();
    for sub in [".claude", ".codex", ".cc-switch"] {
        let path = home.join(sub);
        if path.exists() {
            if let Err(err) = fs::remove_dir_all(&path) {
                eprintln!("failed to clean {}: {}", path.display(), err);
            }
        }
    }
    // 重置内存中的设置缓存，确保测试环境不受上一次调用影响
    // 写入默认设置即可刷新 OnceLock 中的缓存数据
    let _ = update_settings(AppSettings::default());
}

fn test_mutex() -> &'static Mutex<()> {
    static MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
    MUTEX.get_or_init(|| Mutex::new(()))
}

#[test]
fn sync_claude_provider_writes_live_settings() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let home = ensure_test_home();

    let mut config = MultiAppConfig::default();
    let provider_config = json!({
        "env": {
            "ANTHROPIC_AUTH_TOKEN": "test-key",
            "ANTHROPIC_BASE_URL": "https://api.test"
        },
        "ui": {
            "displayName": "Test Provider"
        }
    });

    let provider = Provider::with_id(
        "prov-1".to_string(),
        "Test Claude".to_string(),
        provider_config.clone(),
        None,
    );

    let manager = config
        .get_manager_mut(&AppType::Claude)
        .expect("claude manager");
    manager.providers.insert("prov-1".to_string(), provider);
    manager.current = "prov-1".to_string();

    sync_current_providers_to_live(&mut config).expect("sync live settings");

    let settings_path = get_claude_settings_path();
    assert!(
        settings_path.exists(),
        "live settings should be written to {}",
        settings_path.display()
    );

    let live_value: serde_json::Value = read_json_file(&settings_path).expect("read live file");
    assert_eq!(live_value, provider_config);

    // 确认 SSOT 中的供应商也同步了最新内容
    let updated = config
        .get_manager(&AppType::Claude)
        .and_then(|m| m.providers.get("prov-1"))
        .expect("provider in config");
    assert_eq!(updated.settings_config, provider_config);

    // 额外确认写入位置位于测试 HOME 下
    assert!(
        settings_path.starts_with(home),
        "settings path {:?} should reside under test HOME {:?}",
        settings_path,
        home
    );
}

#[test]
fn create_backup_skips_missing_file() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let home = ensure_test_home();
    let config_path = home.join(".cc-switch").join("config.json");

    // 未创建文件时应返回空字符串，不报错
    let result = create_backup(&config_path).expect("create backup");
    assert!(
        result.is_empty(),
        "expected empty backup id when config file missing"
    );
}

#[test]
fn create_backup_generates_snapshot_file() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let home = ensure_test_home();
    let config_dir = home.join(".cc-switch");
    let config_path = config_dir.join("config.json");
    fs::create_dir_all(&config_dir).expect("prepare config dir");
    fs::write(&config_path, r#"{"version":2}"#).expect("write config file");

    let backup_id = create_backup(&config_path).expect("backup success");
    assert!(
        !backup_id.is_empty(),
        "backup id should contain timestamp information"
    );

    let backup_path = config_dir.join("backups").join(format!("{backup_id}.json"));
    assert!(
        backup_path.exists(),
        "expected backup file at {}",
        backup_path.display()
    );

    let backup_content = fs::read_to_string(&backup_path).expect("read backup");
    assert!(
        backup_content.contains(r#""version":2"#),
        "backup content should match original config"
    );
}
