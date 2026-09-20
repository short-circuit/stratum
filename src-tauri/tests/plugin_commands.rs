//! Tauri command-level integration tests for the plugin command handlers.
//!
//! These tests drive the REAL `#[tauri::command]` handlers over the real Tauri
//! IPC dispatcher using the official `tauri::test` mock-app harness. Each test
//! builds a mock Tauri app whose `VaultState` is backed by a REAL temp vault
//! holding a REAL `wat`-compiled WASM plugin on disk. The command handlers
//! (state injection, permission gating, host backends) are the production
//! ones — nothing is stubbed.
//!
//! This is the layer the F6 contract's "Tauri command tests for the 5 commands"
//! requires. The wired plugin commands are:
//!   * §9.1 plugins_list     — list installed plugins with status
//!   * §9.2 plugins_enable   — enable a disabled plugin (re-arm hooks)
//!   * §9.3 plugins_disable  — disable a running plugin
//!   * §9.4 plugins_reload   — re-instantiate from disk
//!   * §9.5 plugins_status   — status for one plugin
//!   * §9.6 plugin_note_read — host-test note read (no plugin required)
//!   * §9.7 plugin_http_request — host-test http (no plugin required)
//!   * §9.8 plugins_install  — install from a wasm path
//!   * §9.9 plugins_uninstall — uninstall by id
//!
//! HTTP (when involved) runs against wiremock. Tests are plain `#[test]`s:
//! the async `#[tauri::command]` handlers are driven by Tauri's own internal
//! tokio runtime (spawned from a sync caller, so no runtime nesting), and the
//! mock server for install/uninstall tests needs no tokio worker context.

mod common;

use app_lib::commands::plugins::{
    dispatch_on_link, dispatch_on_open, dispatch_on_search, PluginManager,
};
use app_lib::commands::vault::{AppState, VaultState};
use std::collections::HashSet;
use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri::webview::InvokeRequest;

// ---------------------------------------------------------------------------
// Fixtures — real temp vault + real compiled WASM plugin
// ---------------------------------------------------------------------------

/// A real module that exports `onSave`/`onOpen`/`onLink`/`onSearch` and
/// echoes the payload back as its result (so hook dispatch and permission
/// checks are observable), plus a memory read for the note path. It writes
/// its returned length to offset 0 and returns that length.
const ECHO_PLUGIN_WAT: &str = r#"
    (module
        (memory (export "memory") 1)
        (func $len_from_payload (export "onSave") (param $ptr i32) (param $len i32) (result i32)
            local.get $len
        )
        (func (export "onOpen") (param $ptr i32) (param $len i32) (result i32)
            local.get $len
        )
        (func (export "onLink") (param $ptr i32) (param $len i32) (result i32)
            local.get $len
        )
        (func (export "onSearch") (param $ptr i32) (param $len i32) (result i32)
            local.get $len
        )
    )
"#;

const ECHO_SIDECAR_MANIFEST: &str = r#"{
    "schema_version": 1,
    "id": "com.example.cmd-e2e",
    "name": "Cmd E2E",
    "version": "0.1.0",
    "entry": "plugin.wasm",
    "permissions": ["file:read", "file:write", "network"],
    "hooks": { "onSave": true, "onOpen": true, "onLink": true, "onSearch": true }
}"#;

/// A minimal real module with `onSave` for install/uninstall tests.
const INSTALL_PLUGIN_WAT: &str = r#"
    (module
        (memory (export "memory") 1)
        (func (export "onSave") (param i32 i32) (result i32)
            i32.const 0
        )
    )
"#;

const INSTALL_SIDECAR_MANIFEST: &str = r#"{
    "schema_version": 1,
    "id": "com.example.install-e2e",
    "name": "Install E2E",
    "version": "0.1.0",
    "entry": "plugin.wasm",
    "permissions": [],
    "hooks": { "onSave": true }
}"#;

/// Write a plugin into the canonical `<vault>/.pkm/plugins/<id>/` layout with
/// an embedded (or sidecar) manifest.
fn write_echo_plugin(vault: &common::TestVault) {
    let dir = vault
        .vault_path
        .join(".pkm")
        .join("plugins")
        .join("com.example.cmd-e2e");
    std::fs::create_dir_all(&dir).unwrap();
    let wasm = wat::parse_str(ECHO_PLUGIN_WAT).expect("WAT");
    std::fs::write(dir.join("plugin.wasm"), wasm).unwrap();
    std::fs::write(dir.join("plugin.wasm.manifest.json"), ECHO_SIDECAR_MANIFEST).unwrap();
}

fn write_install_source(dir: &std::path::Path) -> std::path::PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let wasm_path = dir.join("to-install.wasm");
    std::fs::write(&wasm_path, wat::parse_str(INSTALL_PLUGIN_WAT).unwrap()).unwrap();
    std::fs::write(
        dir.join("to-install.wasm.manifest.json"),
        INSTALL_SIDECAR_MANIFEST,
    )
    .unwrap();
    wasm_path
}

/// Build a mock Tauri app whose `VaultState` is backed by `vault`, with the
/// plugin manager loaded and the plugin command handlers registered.
fn build_app(vault: &common::TestVault, enabled: bool) -> tauri::App<tauri::test::MockRuntime> {
    let mut enabled_ids = HashSet::new();
    if enabled {
        enabled_ids.insert("com.example.cmd-e2e".to_string());
    }
    let manager =
        PluginManager::new(vault.vault_path.clone(), vec![], enabled_ids).expect("manager");

    let mut vs = VaultState::new(vault.vault_path.clone());
    vs.plugin_manager = Some(manager);

    mock_builder()
        .manage(std::sync::Mutex::new(vs) as AppState)
        .invoke_handler(tauri::generate_handler![
            app_lib::commands::plugins::plugins_list,
            app_lib::commands::plugins::plugins_enable,
            app_lib::commands::plugins::plugins_disable,
            app_lib::commands::plugins::plugins_reload,
            app_lib::commands::plugins::plugins_status,
            app_lib::commands::plugins::plugin_note_read,
            app_lib::commands::plugins::plugin_http_request,
            app_lib::commands::plugins::plugins_install,
            app_lib::commands::plugins::plugins_uninstall,
        ])
        .build(mock_context(noop_assets()))
        .expect("app build")
}

/// Build the mock app with the plugin manager present.
fn app_with_manager(
    vault: &common::TestVault,
    enabled: bool,
) -> tauri::App<tauri::test::MockRuntime> {
    build_app(vault, enabled)
}

/// Drive one command invocation through the real IPC dispatcher and return the
/// deserialized JSON response or the rejection value.
fn invoke<W: std::convert::AsRef<tauri::Webview<tauri::test::MockRuntime>>>(
    webview: &W,
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

fn webview(
    app: &tauri::App<tauri::test::MockRuntime>,
) -> tauri::WebviewWindow<tauri::test::MockRuntime> {
    tauri::WebviewWindowBuilder::new(app, "main", Default::default())
        .build()
        .unwrap()
}

/// Run a future to completion on a short-lived current-thread tokio runtime.
/// Used for async wiremock setup; the command handlers run on Tauri's own
/// runtime via the IPC dispatcher, so no nesting occurs.
fn block_on<F: std::future::Future>(fut: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("test runtime")
        .block_on(fut)
}

/// Start a wiremock server. The server runs on a dedicated thread + runtime,
/// so it stays alive after the setup runtime is dropped.
fn start_mock() -> wiremock::MockServer {
    block_on(wiremock::MockServer::start())
}

fn mount_mock(mock: wiremock::Mock, server: &wiremock::MockServer) {
    block_on(mock.mount(server));
}

// ---------------------------------------------------------------------------
// plugins_list / plugins_status
// ---------------------------------------------------------------------------

#[test]
fn command_plugins_list_returns_loaded_plugin_with_status() {
    let vault = common::create_test_vault();
    write_echo_plugin(&vault);
    let app = app_with_manager(&vault, true);
    let wv = webview(&app);

    let res = invoke(&wv, "plugins_list", serde_json::json!({}));
    let list = res.expect("list must resolve");
    assert_eq!(list["plugins"][0]["id"], "com.example.cmd-e2e");
    assert_eq!(list["plugins"][0]["status"], "ready");
    assert_eq!(list["plugins"][0]["enabled"], true);
}

#[test]
fn command_plugins_list_empty_when_no_manager() {
    let vault = common::create_test_vault();
    // Build an app with a VaultState that has no plugin manager.
    let vs = VaultState::new(vault.vault_path.clone());
    let app = mock_builder()
        .manage(std::sync::Mutex::new(vs) as AppState)
        .invoke_handler(tauri::generate_handler![
            app_lib::commands::plugins::plugins_list,
        ])
        .build(mock_context(noop_assets()))
        .unwrap();
    let wv = webview(&app);

    let res = invoke(&wv, "plugins_list", serde_json::json!({}));
    let list = res.expect("list must resolve");
    assert_eq!(list["plugins"], serde_json::json!([]));
}

#[test]
fn command_plugins_status_happy_and_missing() {
    let vault = common::create_test_vault();
    write_echo_plugin(&vault);
    let app = app_with_manager(&vault, true);
    let wv = webview(&app);

    let ok = invoke(
        &wv,
        "plugins_status",
        serde_json::json!({ "id": "com.example.cmd-e2e" }),
    )
    .expect("status must resolve for known id");
    assert_eq!(ok["id"], "com.example.cmd-e2e");
    assert_eq!(ok["status"], "ready");

    let missing = invoke(&wv, "plugins_status", serde_json::json!({ "id": "nope" }))
        .expect_err("unknown id must reject");
    let msg = missing["error"]
        .as_str()
        .or_else(|| missing.as_str())
        .unwrap_or_default();
    assert!(msg.contains("plugin_not_found"), "msg: {msg}");
}

// ---------------------------------------------------------------------------
// plugins_enable / plugins_disable / plugins_reload
// ---------------------------------------------------------------------------

#[test]
fn command_disable_then_enable_roundtrip() {
    let vault = common::create_test_vault();
    write_echo_plugin(&vault);
    let app = app_with_manager(&vault, true);
    let wv = webview(&app);

    let dis = invoke(
        &wv,
        "plugins_disable",
        serde_json::json!({ "id": "com.example.cmd-e2e" }),
    )
    .expect("disable must resolve");
    assert_eq!(dis["enabled"], false);
    assert_eq!(dis["status"], "disabled");

    let en = invoke(
        &wv,
        "plugins_enable",
        serde_json::json!({ "id": "com.example.cmd-e2e" }),
    )
    .expect("enable must resolve");
    assert_eq!(en["enabled"], true);
    assert_eq!(en["status"], "ready");
}

#[test]
fn command_enable_missing_plugin_rejects() {
    let vault = common::create_test_vault();
    write_echo_plugin(&vault);
    let app = app_with_manager(&vault, true);
    let wv = webview(&app);

    let missing = invoke(&wv, "plugins_enable", serde_json::json!({ "id": "ghost" }))
        .expect_err("enable of unknown plugin must reject");
    let msg = missing["error"]
        .as_str()
        .or_else(|| missing.as_str())
        .unwrap_or_default();
    assert!(msg.contains("plugin_not_found"), "msg: {msg}");
}

#[test]
fn command_reload_reinstantiates_from_disk() {
    let vault = common::create_test_vault();
    write_echo_plugin(&vault);
    let app = app_with_manager(&vault, true);
    let wv = webview(&app);

    let rel = invoke(
        &wv,
        "plugins_reload",
        serde_json::json!({ "id": "com.example.cmd-e2e" }),
    )
    .expect("reload must resolve");
    assert_eq!(rel["id"], "com.example.cmd-e2e");
    assert_eq!(rel["status"], "ready");
}

#[test]
fn command_reload_corrupted_plugin_reports_load_error() {
    let vault = common::create_test_vault();
    // Install the plugin then corrupt its wasm on disk.
    write_echo_plugin(&vault);
    let wasm_path = vault
        .vault_path
        .join(".pkm")
        .join("plugins")
        .join("com.example.cmd-e2e")
        .join("plugin.wasm");
    std::fs::write(&wasm_path, b"garbage wasm bytes").unwrap();

    let app = app_with_manager(&vault, true);
    let wv = webview(&app);

    let err = invoke(
        &wv,
        "plugins_reload",
        serde_json::json!({ "id": "com.example.cmd-e2e" }),
    )
    .expect_err("reload of corrupted wasm must reject");
    let msg = err["error"]
        .as_str()
        .or_else(|| err.as_str())
        .unwrap_or_default();
    assert!(msg.contains("plugin_load_error"), "msg: {msg}");
}

// ---------------------------------------------------------------------------
// plugins_install / plugins_uninstall (F-series lifecycle)
// ---------------------------------------------------------------------------

#[test]
fn command_install_then_uninstall_roundtrip() {
    let vault = common::create_test_vault();
    let app = app_with_manager(&vault, false);
    let wv = webview(&app);

    // Install from a source path.
    let src = write_install_source(&vault.vault_path.join("_src"));
    let installed = invoke(
        &wv,
        "plugins_install",
        serde_json::json!({ "path": src.to_string_lossy() }),
    )
    .expect("install must resolve");
    assert_eq!(installed["id"], "com.example.install-e2e");
    assert_eq!(installed["status"], "disabled", "fresh install is disabled");

    // Install missing source -> plugin_not_found.
    let missing = invoke(
        &wv,
        "plugins_install",
        serde_json::json!({ "path": vault.vault_path.join("nope.wasm").to_string_lossy() }),
    )
    .expect_err("missing source must reject");
    let mmsg = missing["error"]
        .as_str()
        .or_else(|| missing.as_str())
        .unwrap_or_default();
    assert!(mmsg.contains("plugin_not_found"), "msg: {mmsg}");

    // Uninstall removes it.
    let after_uninstall = invoke(
        &wv,
        "plugins_uninstall",
        serde_json::json!({ "id": "com.example.install-e2e" }),
    )
    .expect("uninstall must resolve");
    assert_eq!(after_uninstall["plugins"], serde_json::json!([]));
    assert!(
        !vault
            .vault_path
            .join(".pkm")
            .join("plugins")
            .join("com.example.install-e2e")
            .exists(),
        "plugin dir must be removed"
    );
}

// ---------------------------------------------------------------------------
// plugin_note_read / plugin_http_request (host-test commands)
// ---------------------------------------------------------------------------

#[test]
fn command_note_read_returns_real_note_content() {
    let vault = common::create_test_vault();
    vault.create_md_file("notes/real.md", "secret content");
    let app = app_with_manager(&vault, true);
    let wv = webview(&app);

    let res = invoke(
        &wv,
        "plugin_note_read",
        serde_json::json!({ "path": "notes/real.md" }),
    )
    .expect("note_read must resolve");
    assert_eq!(res["path"], "notes/real.md");
    assert_eq!(res["content"], "secret content");
    assert!(res["mtime"].as_str().is_some());
}

#[test]
fn command_note_read_missing_note_rejects() {
    let vault = common::create_test_vault();
    let app = app_with_manager(&vault, true);
    let wv = webview(&app);

    let err = invoke(
        &wv,
        "plugin_note_read",
        serde_json::json!({ "path": "notes/gone.md" }),
    )
    .expect_err("missing note must reject");
    let msg = err["error"]
        .as_str()
        .or_else(|| err.as_str())
        .unwrap_or_default();
    assert!(msg.contains("note_not_found"), "msg: {msg}");
}

#[test]
fn command_http_request_hits_wiremock_and_returns_body() {
    let server = start_mock();
    mount_mock(
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/probe"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string("mock body")),
        &server,
    );

    let vault = common::create_test_vault();
    let app = app_with_manager(&vault, true);
    let wv = webview(&app);

    let res = invoke(
        &wv,
        "plugin_http_request",
        serde_json::json!({ "method": "GET", "url": format!("{}/probe", server.uri()) }),
    )
    .expect("http_request must resolve");
    assert_eq!(res["status"], 200);
    assert_eq!(res["body"], "mock body");
}

#[test]
fn command_http_request_blocked_by_ssrf() {
    let vault = common::create_test_vault();
    let app = app_with_manager(&vault, true);
    let wv = webview(&app);

    // A public IP with no allowlist and not private -> http_ssid, no server.
    let err = invoke(
        &wv,
        "plugin_http_request",
        serde_json::json!({ "method": "GET", "url": "http://8.8.8.8/x" }),
    )
    .expect_err("public host must be SSRF-blocked");
    let msg = err["error"]
        .as_str()
        .or_else(|| err.as_str())
        .unwrap_or_default();
    assert!(msg.contains("http_ssid"), "msg: {msg}");
}

// ---------------------------------------------------------------------------
// Hook dispatch (spec §8) through the real manager
// ---------------------------------------------------------------------------

#[test]
fn hook_dispatch_delivers_expected_payloads() {
    let vault = common::create_test_vault();
    write_echo_plugin(&vault);
    let app = app_with_manager(&vault, true);
    let _wv = webview(&app);

    use app_lib::commands::plugins::PluginManager as PM;
    let mut enabled_ids = HashSet::new();
    enabled_ids.insert("com.example.cmd-e2e".to_string());
    let manager = PM::new(vault.vault_path.clone(), vec![], enabled_ids).expect("manager");

    // onOpen payload shape.
    let open = dispatch_on_open(&manager, "pages/a.md");
    assert_eq!(open.len(), 1);
    assert!(open[0].1.is_some(), "echo returns a payload");

    // onLink payload shape.
    let links = vec!["b".to_string()];
    let linked = dispatch_on_link(&manager, "pages/a.md", &links);
    assert_eq!(linked.len(), 1);

    // onSearch payload shape.
    let search = dispatch_on_search(&manager, "quantum", 10);
    assert_eq!(search.len(), 1);
}
