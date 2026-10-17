//! Verify the per-vault `pages_view` setting round-trips through the real
//! `get_settings` / `save_settings` Tauri command handlers.
//!
//! These tests drive the production `#[tauri::command]` handlers over the real
//! Tauri IPC dispatcher against a REAL temp vault (no stubbing): config
//! load/save, the layout-preservation path, and the `#[serde(default)]`
//! fallback for old configs are all production code.

mod common;

use app_lib::commands::vault::{AppState, VaultState};
use serde_json::json;
use std::sync::Mutex;
use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri::webview::InvokeRequest;
use tauri::WebviewWindow;

fn build_app(vault: &common::TestVault) -> tauri::App<tauri::test::MockRuntime> {
    let vs = VaultState::new(vault.vault_path.clone());
    mock_builder()
        .manage(Mutex::new(vs) as AppState)
        .invoke_handler(tauri::generate_handler![
            app_lib::commands::settings::get_settings,
            app_lib::commands::settings::save_settings,
        ])
        .build(mock_context(noop_assets()))
        .expect("app build")
}

fn invoke(
    webview: &WebviewWindow<tauri::test::MockRuntime>,
    cmd: &str,
    body: serde_json::Value,
) -> Result<serde_json::Value, serde_json::Value> {
    let request = InvokeRequest {
        cmd: cmd.to_string(),
        callback: tauri::ipc::CallbackFn(0),
        error: tauri::ipc::CallbackFn(1),
        url: "tauri://localhost".parse().unwrap(),
        body: tauri::ipc::InvokeBody::Json(body),
        headers: Default::default(),
        invoke_key: tauri::test::INVOKE_KEY.to_string(),
    };
    tauri::test::get_ipc_response(webview, request).map(|b| {
        b.deserialize::<serde_json::Value>()
            .unwrap_or(serde_json::Value::Null)
    })
}

fn webview(app: &tauri::App<tauri::test::MockRuntime>) -> WebviewWindow<tauri::test::MockRuntime> {
    tauri::WebviewWindowBuilder::new(app, "main", Default::default())
        .build()
        .unwrap()
}

/// Build a `save_settings` body with a subset of settings compatible with the
/// handler, carrying the given `layout` value (passed through verbatim).
fn settings_body(tv: &common::TestVault, layout: serde_json::Value) -> serde_json::Value {
    json!({
        "vault_path": tv.vault_path,
        "locale": "en",
        "theme": { "dark_mode": false, "primary_color": "#fff", "secondary_color": "#000", "font_size": 14 },
        "ai": {
            "provider": "custom-openai",
            "endpoint": "http://localhost:8080/v1",
            "api_key": null,
            "model": "llama3.2",
            "models": [],
            "rag_enabled": true,
            "rag_chunk_count": 3,
            "embedding_dimensions": 0
        },
        "research": { "searxng_endpoint": "http://localhost:8888", "max_results": 3, "max_depth": 2 },
        "graph": { "show_connected": true, "show_orphaned": true, "show_tags": true, "charge_strength": 30.0, "link_distance": 120.0, "alpha_decay": 0.1, "velocity_decay": 0.2, "link_curvature": 0.0 },
        "sync": { "mode": "manual", "branch": "master", "auto_commit_interval_secs": 300, "auto_sync_interval_secs": 3600, "commit_template": "{{message}}" },
        "stt": { "endpoint": "http://localhost:9001", "model": "whisper-1", "diarize_model": "", "language": null, "diarize": false, "auto_summarize": false, "auto_identify": false },
        "tts": { "endpoint": "", "voice": "alloy", "format": "mp3", "speed": 1.0 },
        "tray": { "enabled": true, "close_to_tray": true, "quick_capture_shortcut": "CmdOrCtrl+Shift+Space", "capture_target": "journal", "inbox_path": "Inbox.md" },
        "voice": { "enabled": true, "target": "journal:today" },
        "layout": layout,
    })
}

#[test]
fn save_settings_round_trips_pages_view() {
    let tv = common::create_test_vault();
    let app = build_app(&tv);
    let wv = webview(&app);

    invoke(
        &wv,
        "save_settings",
        json!({ "settings": settings_body(&tv, json!({ "pages_view": "tree" })) }),
    )
    .expect("save resolves");

    let res = invoke(&wv, "get_settings", json!({})).expect("get resolves");
    assert_eq!(
        res["layout"]["pages_view"].as_str(),
        Some("tree"),
        "get_settings must return the saved pages_view"
    );

    let on_disk = std::fs::read_to_string(tv.vault_path.join(".pkm").join("config.toml")).unwrap();
    assert!(
        on_disk.contains("pages_view = \"tree\""),
        "on-disk config must persist pages_view = \"tree\": {on_disk}"
    );
}

#[test]
fn get_settings_defaults_pages_view_to_flat() {
    // Fresh vault with no config.toml: pages_view must default to "flat".
    let tv = common::create_test_vault();
    let app = build_app(&tv);
    let wv = webview(&app);

    let res = invoke(&wv, "get_settings", json!({})).expect("get resolves");
    assert_eq!(
        res["layout"]["pages_view"].as_str(),
        Some("flat"),
        "fresh vault must default pages_view to flat"
    );
}

#[test]
fn save_settings_preserves_custom_layout_dirs() {
    // Pre-existing config with a custom pages_dir must keep it after a save
    // that only sets pages_view ("flat").
    let tv = common::create_test_vault();
    let pkm = tv.vault_path.join(".pkm");
    std::fs::create_dir_all(&pkm).unwrap();
    std::fs::write(pkm.join("config.toml"), "[layout]\npages_dir = \"docs\"\n").unwrap();

    let app = build_app(&tv);
    let wv = webview(&app);

    invoke(
        &wv,
        "save_settings",
        json!({ "settings": settings_body(&tv, json!({ "pages_view": "flat" })) }),
    )
    .expect("save resolves");

    let on_disk = std::fs::read_to_string(tv.vault_path.join(".pkm").join("config.toml")).unwrap();
    assert!(
        on_disk.contains("pages_dir = \"docs\""),
        "custom pages_dir must be preserved: {on_disk}"
    );
    assert!(
        on_disk.contains("pages_view = \"flat\""),
        "pages_view must be written alongside the preserved dirs: {on_disk}"
    );
}
