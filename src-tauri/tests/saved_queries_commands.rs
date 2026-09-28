//! Integration tests for the saved-query persistence Tauri commands.
//!
//! Drives the REAL `#[tauri::command]` handlers (`list_saved_queries`,
//! `save_saved_query`, `rename_saved_query`, `delete_saved_query`) over the
//! real Tauri IPC dispatcher using `tauri::test` mock-app harness, backed by a
//! REAL temp vault. Nothing is stubbed — the command handlers and the
//! pkm-query persistence store are all production code.
//!
//! Coverage maps to the task ACs:
//!   * persisted to `.pkm/saved_queries.json` inside the vault (and therefore
//!     Git-trackable / vault-synced)
//!   * missing file ⇒ empty list
//!   * name validation (empty rejected, trimmed, uniqueness enforced)
//!   * persistence across a simulated restart by reading the file back fresh
//!   * error surfacing (duplicates, not-found)

mod common;

use app_lib::commands::vault::{AppState, VaultState};
use serde_json::json;
use std::sync::Mutex;
use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri::webview::InvokeRequest;
use tauri::WebviewWindow;

/// Build a mock Tauri app whose `VaultState` is backed by `vault`, with the
/// saved-query commands registered.
fn build_app(vault: &common::TestVault) -> tauri::App<tauri::test::MockRuntime> {
    let vs = VaultState::new(vault.vault_path.clone());
    mock_builder()
        .manage(Mutex::new(vs) as AppState)
        .invoke_handler(tauri::generate_handler![
            app_lib::commands::query::list_saved_queries,
            app_lib::commands::query::save_saved_query,
            app_lib::commands::query::rename_saved_query,
            app_lib::commands::query::delete_saved_query,
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

/// Extract a user-facing error message from a command rejection payload.
fn error_message(err: &serde_json::Value) -> String {
    err["error"]
        .as_str()
        .or_else(|| err.as_str())
        .unwrap_or_default()
        .to_string()
}

const SAVED_QUERIES_FILE: &str = ".pkm/saved_queries.json";

#[test]
fn list_saved_queries_missing_file_returns_empty() {
    let tv = common::create_test_vault();
    let app = build_app(&tv);
    let wv = webview(&app);

    let resp =
        invoke(&wv, "list_saved_queries", json!({})).expect("list_saved_queries must resolve");
    assert_eq!(resp.as_array().expect("array").len(), 0);
}

#[test]
fn save_list_restart_roundtrip_persists_to_file() {
    let tv = common::create_test_vault();
    let app = build_app(&tv);
    let wv = webview(&app);

    let saved = invoke(
        &wv,
        "save_saved_query",
        json!({ "name": "Top tasks", "query": "{:query [:find ?b :where [?b :block/marker \"TODO\"]]}" }),
    )
    .expect("save_saved_query must resolve");
    assert_eq!(saved["name"], "Top tasks");
    assert!(saved["updated_at"].as_str().is_some(), "updated_at set");

    // The file is written into the vault's .pkm/ directory.
    let file = tv.vault_path.join(SAVED_QUERIES_FILE);
    assert!(file.exists(), "saved_queries.json must be written");
    assert!(
        file.starts_with(tv.vault_path.join(".pkm")),
        "file must live under .pkm/"
    );

    // Simulated restart: a brand-new app/handle reads the same file back cold.
    let app2 = build_app(&tv);
    let wv2 = webview(&app2);
    let resp = invoke(&wv2, "list_saved_queries", json!({})).expect("list must resolve");
    let list = resp.as_array().expect("array");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["name"], "Top tasks");
    assert_eq!(
        list[0]["query"],
        "{:query [:find ?b :where [?b :block/marker \"TODO\"]]}"
    );
}

#[test]
fn save_duplicate_name_overwrites_and_keeps_single_entry() {
    let tv = common::create_test_vault();
    let app = build_app(&tv);
    let wv = webview(&app);

    invoke(
        &wv,
        "save_saved_query",
        json!({ "name": "q", "query": "first" }),
    )
    .expect("first save resolves");
    let second = invoke(
        &wv,
        "save_saved_query",
        json!({ "name": "q", "query": "second" }),
    )
    .expect("overwrite resolves");
    assert_eq!(second["query"], "second");

    let list = invoke(&wv, "list_saved_queries", json!({})).expect("list resolves");
    let arr = list.as_array().expect("array");
    assert_eq!(arr.len(), 1, "overwrite must not create duplicates");
    assert_eq!(arr[0]["query"], "second");
}

#[test]
fn save_empty_name_reports_clear_error() {
    let tv = common::create_test_vault();
    let app = build_app(&tv);
    let wv = webview(&app);

    let err = invoke(
        &wv,
        "save_saved_query",
        json!({ "name": "   ", "query": "q" }),
    )
    .expect_err("empty name must reject");
    let msg = error_message(&err);
    assert!(msg.contains("empty"), "clear error expected, got: {msg}");

    // Nothing was written.
    assert!(!tv.vault_path.join(SAVED_QUERIES_FILE).exists());
}

#[test]
fn delete_removes_entry_and_persists() {
    let tv = common::create_test_vault();
    let app = build_app(&tv);
    let wv = webview(&app);

    invoke(
        &wv,
        "save_saved_query",
        json!({ "name": "a", "query": "q1" }),
    )
    .unwrap();
    invoke(
        &wv,
        "save_saved_query",
        json!({ "name": "b", "query": "q2" }),
    )
    .unwrap();

    invoke(&wv, "delete_saved_query", json!({ "name": "a" })).expect("delete must resolve");

    let list = invoke(&wv, "list_saved_queries", json!({})).expect("list resolves");
    let arr = list.as_array().expect("array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["name"], "b");

    // Delete is persisted — a fresh handle does not see the removed entry.
    let list2 = invoke(&wv, "list_saved_queries", json!({})).expect("list resolves");
    assert_eq!(list2.as_array().unwrap().len(), 1);
}

#[test]
fn rename_works_and_errors_on_conflict() {
    let tv = common::create_test_vault();
    let app = build_app(&tv);
    let wv = webview(&app);

    invoke(
        &wv,
        "save_saved_query",
        json!({ "name": "old", "query": "q" }),
    )
    .unwrap();

    let renamed = invoke(
        &wv,
        "rename_saved_query",
        json!({ "oldName": "old", "newName": "new" }),
    )
    .expect("rename resolves");
    assert_eq!(renamed["name"], "new");

    let list = invoke(&wv, "list_saved_queries", json!({})).expect("list resolves");
    assert_eq!(list.as_array().unwrap()[0]["name"], "new");

    // Conflict: renaming onto an existing name errors clearly.
    invoke(
        &wv,
        "save_saved_query",
        json!({ "name": "other", "query": "q2" }),
    )
    .unwrap();
    let err = invoke(
        &wv,
        "rename_saved_query",
        json!({ "oldName": "new", "newName": "other" }),
    )
    .expect_err("conflict must reject");
    assert!(
        error_message(&err).contains("already exists"),
        "clear conflict error expected, got: {}",
        error_message(&err)
    );
}

#[test]
fn rename_missing_source_reports_error() {
    let tv = common::create_test_vault();
    let app = build_app(&tv);
    let wv = webview(&app);

    let err = invoke(
        &wv,
        "rename_saved_query",
        json!({ "oldName": "ghost", "newName": "x" }),
    )
    .expect_err("missing source must reject");
    assert!(
        error_message(&err).contains("not found"),
        "clear not-found error expected, got: {}",
        error_message(&err)
    );
}

#[test]
fn special_characters_persist_through_the_command_layer() {
    let tv = common::create_test_vault();
    let app = build_app(&tv);
    let wv = webview(&app);

    let weird = "任务 & \"quotes\" \u{1F4DD}";
    invoke(
        &wv,
        "save_saved_query",
        json!({ "name": weird, "query": "q" }),
    )
    .expect("save resolves");

    // Read the raw file back — round-trips UTF-8 unchanged.
    let raw = std::fs::read_to_string(tv.vault_path.join(SAVED_QUERIES_FILE)).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(parsed[0]["name"], weird);
}

#[test]
fn corrupt_file_treated_as_empty_list() {
    let tv = common::create_test_vault();
    // Write a corrupt JSON file ahead of time.
    std::fs::write(tv.vault_path.join(SAVED_QUERIES_FILE), "{ not json").unwrap();

    let app = build_app(&tv);
    let wv = webview(&app);
    let resp = invoke(&wv, "list_saved_queries", json!({})).expect("list must resolve");
    assert_eq!(
        resp.as_array().expect("array").len(),
        0,
        "corrupt file must read as empty"
    );
}
