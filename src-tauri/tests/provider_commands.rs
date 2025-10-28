use serde_json::json;

use cc_switch_lib::{
    get_codex_auth_path, get_codex_config_path, read_json_file, switch_provider_test_hook,
    write_codex_live_atomic, AppState, AppType, MultiAppConfig, Provider,
};

#[path = "support.rs"]
mod support;
use support::{ensure_test_home, reset_test_fs, test_mutex};

#[test]
fn switch_provider_updates_codex_live_and_state() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let _home = ensure_test_home();

    let legacy_auth = json!({"OPENAI_API_KEY": "legacy-key"});
    let legacy_config = r#"[mcp_servers.legacy]
type = "stdio"
command = "echo"
"#;
    write_codex_live_atomic(&legacy_auth, Some(legacy_config))
        .expect("seed existing codex live config");

    let mut config = MultiAppConfig::default();
    {
        let manager = config
            .get_manager_mut(&AppType::Codex)
            .expect("codex manager");
        manager.current = "old-provider".to_string();
        manager.providers.insert(
            "old-provider".to_string(),
            Provider::with_id(
                "old-provider".to_string(),
                "Legacy".to_string(),
                json!({
                    "auth": {"OPENAI_API_KEY": "stale"},
                    "config": "stale-config"
                }),
                None,
            ),
        );
        manager.providers.insert(
            "new-provider".to_string(),
            Provider::with_id(
                "new-provider".to_string(),
                "Latest".to_string(),
                json!({
                    "auth": {"OPENAI_API_KEY": "fresh-key"},
                    "config": r#"[mcp_servers.latest]
type = "stdio"
command = "say"
"#
                }),
                None,
            ),
        );
    }

    config.mcp.codex.servers.insert(
        "echo-server".into(),
        json!({
            "id": "echo-server",
            "enabled": true,
            "server": {
                "type": "stdio",
                "command": "echo"
            }
        }),
    );

    let app_state = AppState {
        config: std::sync::Mutex::new(config),
    };

    switch_provider_test_hook(&app_state, AppType::Codex, "new-provider")
        .expect("switch provider should succeed");

    let auth_value: serde_json::Value =
        read_json_file(&get_codex_auth_path()).expect("read auth.json");
    assert_eq!(
        auth_value
            .get("OPENAI_API_KEY")
            .and_then(|v| v.as_str())
            .unwrap_or(""),
        "fresh-key",
        "live auth.json should reflect new provider"
    );

    let config_text =
        std::fs::read_to_string(get_codex_config_path()).expect("read config.toml");
    assert!(
        config_text.contains("mcp_servers.echo-server"),
        "config.toml should contain synced MCP servers"
    );

    let locked = app_state
        .config
        .lock()
        .expect("lock config after switch");
    let manager = locked
        .get_manager(&AppType::Codex)
        .expect("codex manager after switch");
    assert_eq!(manager.current, "new-provider", "current provider updated");

    let new_provider = manager
        .providers
        .get("new-provider")
        .expect("new provider exists");
    let new_config_text = new_provider
        .settings_config
        .get("config")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    assert_eq!(
        new_config_text, config_text,
        "provider config snapshot should match live file"
    );

    let legacy = manager
        .providers
        .get("old-provider")
        .expect("legacy provider still exists");
    let legacy_auth_value = legacy
        .settings_config
        .get("auth")
        .and_then(|v| v.get("OPENAI_API_KEY"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert_eq!(
        legacy_auth_value, "legacy-key",
        "previous provider should be backfilled with live auth"
    );
}

#[test]
fn switch_provider_missing_provider_returns_error() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();

    let mut config = MultiAppConfig::default();
    config
        .get_manager_mut(&AppType::Claude)
        .expect("claude manager")
        .current = "does-not-exist".to_string();

    let app_state = AppState {
        config: std::sync::Mutex::new(config),
    };

    let err = switch_provider_test_hook(&app_state, AppType::Claude, "missing-provider")
        .expect_err("switching to a missing provider should fail");

    assert!(
        err.to_string().contains("供应商不存在"),
        "error message should mention missing provider"
    );
}
