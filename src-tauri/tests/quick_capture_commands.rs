//! Integration tests for the tray / quick-capture Tauri commands.
//!
//! Drives the REAL `#[tauri::command]` handlers (`get_tray_status`,
//! `quick_capture_save`) over the real Tauri IPC dispatcher using the
//! `tauri::test` mock-app harness, backed by a REAL temp vault. Nothing is
//! stubbed — the command handlers and the pkm-block/pkm-markdown persistence
//! are all production code.
//!
//! Coverage maps to the task ACs:
//!   * capture writes a block into today's journal (default target) and
//!     persists it to disk (the .md file contains the captured text)
//!   * inbox target: configured `capture_target = "inbox"` routes the capture
//!     to the configured inbox page and creates the file on disk if missing
//!   * get_tray_status round-trips the tray config from `<vault>/.pkm/config.toml`
//!   * a missing config file yields safe defaults (tray present but disabled
//!     is only ever true on mobile; desktop defaults are returned)

mod common;

use app_lib::commands::vault::{AppState, VaultState};
use serde_json::json;
use std::sync::Mutex;
use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri::webview::InvokeRequest;
use tauri::WebviewWindow;

/// Build a mock Tauri app whose `VaultState` is backed by `vault`, with the
/// tray commands registered.
fn build_app(vault: &common::TestVault) -> tauri::App<tauri::test::MockRuntime> {
    let vs = VaultState::new(vault.vault_path.clone());
    mock_builder()
        .manage(Mutex::new(vs) as AppState)
        .invoke_handler(tauri::generate_handler![
            app_lib::commands::tray::get_tray_status,
            app_lib::commands::tray::quick_capture_save,
        ])
        .build(mock_context(noop_assets()))
        .expect("app build")
}

/// Drive one command invocation through the real IPC dispatcher and return the
/// deserialized JSON response or the rejection value.
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

/// Local YYYY-MM-DD for today (journal path generation uses `chrono::Local`,
/// which we mirror here).
fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

#[test]
fn quick_capture_save_into_default_journal_writes_block_and_disk() {
    let tv = common::create_test_vault();
    let app = build_app(&tv);
    let wv = webview(&app);

    let resp = invoke(
        &wv,
        "quick_capture_save",
        json!({ "text": "captured idea from tray" }),
    )
    .expect("quick_capture_save must resolve");

    let page_path = format!("journals/{}.md", today());
    assert_eq!(resp["page_path"], page_path);
    assert!(resp["block_id"].as_str().is_some(), "block_id returned");

    // The block is registered in the SQLite store...
    let blocks = tv
        .store
        .get_blocks_by_page(&page_path)
        .expect("blocks readable");
    assert_eq!(blocks.len(), 1, "exactly one block in journal");
    assert_eq!(blocks[0].content, "captured idea from tray");

    // ...and the .md file on disk contains the captured text (git-syncable).
    let disk = std::fs::read_to_string(tv.vault_path.join(&page_path)).expect("journal md exists");
    assert!(
        disk.contains("captured idea from tray"),
        "captured text must be persisted to the .md file, got: {disk}"
    );
}

#[test]
fn quick_capture_save_into_inbox_target_creates_page() {
    let tv = common::create_test_vault();

    // Configure an inbox capture target.
    let config = pkm_core::Config {
        vault_path: tv.vault_path.clone(),
        tray: pkm_core::TrayConfig {
            capture_target: pkm_core::CaptureTarget::Inbox,
            inbox_path: "Inbox.md".to_string(),
            ..Default::default()
        },
        ..Default::default()
    };
    let config_path = tv.vault_path.join(".pkm").join("config.toml");
    config.save(&config_path).expect("config saved");

    let app = build_app(&tv);
    let wv = webview(&app);

    let resp = invoke(
        &wv,
        "quick_capture_save",
        json!({ "text": "inbox quick note" }),
    )
    .expect("quick_capture_save must resolve");

    assert_eq!(resp["page_path"], "Inbox.md");
    assert!(resp["block_id"].as_str().is_some());

    let blocks = tv
        .store
        .get_blocks_by_page("Inbox.md")
        .expect("inbox blocks readable");
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].content, "inbox quick note");

    let disk = std::fs::read_to_string(tv.vault_path.join("Inbox.md")).expect("inbox md exists");
    assert!(disk.contains("inbox quick note"), "inbox .md has the block");
    assert!(disk.contains("title: Inbox"), "inbox frontmatter title");
}

#[test]
fn get_tray_status_roundtrips_custom_config() {
    let tv = common::create_test_vault();

    let config = pkm_core::Config {
        vault_path: tv.vault_path.clone(),
        tray: pkm_core::TrayConfig {
            enabled: false,
            close_to_tray: false,
            quick_capture_shortcut: "Ctrl+Alt+Q".to_string(),
            capture_target: pkm_core::CaptureTarget::Inbox,
            inbox_path: "pages/Inbox.md".to_string(),
        },
        ..Default::default()
    };
    let config_path = tv.vault_path.join(".pkm").join("config.toml");
    config.save(&config_path).expect("config saved");

    let app = build_app(&tv);
    let wv = webview(&app);

    let resp = invoke(&wv, "get_tray_status", json!({})).expect("get_tray_status resolves");
    // On desktop (test host is desktop), enabled = config value (false here).
    assert_eq!(resp["enabled"], false);
    assert_eq!(resp["close_to_tray"], false);
    assert_eq!(resp["shortcut"], "Ctrl+Alt+Q");
    assert_eq!(resp["capture_target"], "inbox");
    assert_eq!(resp["inbox_path"], "pages/Inbox.md");
}

#[test]
fn get_tray_status_without_config_returns_safe_defaults() {
    let tv = common::create_test_vault();
    let app = build_app(&tv);
    let wv = webview(&app);

    let resp = invoke(&wv, "get_tray_status", json!({})).expect("get_tray_status resolves");
    assert_eq!(resp["enabled"], true, "desktop default tray enabled");
    assert_eq!(resp["close_to_tray"], true, "desktop default close-to-tray");
    assert_eq!(resp["shortcut"], "CmdOrCtrl+Shift+Space");
    assert_eq!(resp["capture_target"], "journal");
    assert_eq!(resp["inbox_path"], "Inbox.md");
}
