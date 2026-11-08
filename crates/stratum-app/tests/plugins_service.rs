//! Headless tests for the plugin service commands:
//! `plugins_list`, `plugins_enable`, `plugins_disable`, `plugins_reload`,
//! `plugins_status`, `plugins_install`, `plugins_install_url`,
//! `plugins_uninstall`, `plugin_note_read`, `plugin_http_request`, and the
//! hook-dispatch helpers.

mod common;

use common::{create_test_vault, drop_plugin_runtime};
use std::collections::HashSet;
use std::sync::Arc;
use stratum_app::plugins::{dispatch_on_link, dispatch_on_open, dispatch_on_search, PluginManager};

const PLUGIN_ID: &str = "com.example.cmd-e2e";

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

/// Write a plugin into the canonical `<vault>/.pkm/plugins/<id>/` layout.
fn write_echo_plugin(vault: &common::TestVault) {
    let dir = vault.vault_path.join(".pkm/plugins").join(PLUGIN_ID);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("plugin.wasm"),
        wat::parse_str(ECHO_PLUGIN_WAT).unwrap(),
    )
    .unwrap();
    std::fs::write(dir.join("plugin.wasm.manifest.json"), ECHO_SIDECAR_MANIFEST).unwrap();
}

/// Install a manager into the vault state, optionally enabling the echo plugin.
fn install_manager(vault: &common::TestVault, enabled: bool) -> Arc<PluginManager> {
    let mut ids = HashSet::new();
    if enabled {
        ids.insert(PLUGIN_ID.to_string());
    }
    let manager = PluginManager::new(vault.vault_path.clone(), vec![], ids).expect("manager");
    vault.service.lock().unwrap().plugin_manager = Some(manager.clone());
    manager
}

/// Write an installable source (wasm + sidecar) and return the wasm path.
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

#[tokio::test]
async fn plugins_list_is_empty_without_a_manager() {
    let vault = create_test_vault();

    let list = vault.service.plugins_list().await.unwrap();

    assert!(list.plugins.is_empty());
}

#[tokio::test]
async fn plugins_list_status_enable_disable_reload_roundtrip() {
    let vault = create_test_vault();
    write_echo_plugin(&vault);
    install_manager(&vault, true);

    let list = vault.service.plugins_list().await.unwrap();
    assert_eq!(list.plugins.len(), 1);
    assert_eq!(list.plugins[0].id, PLUGIN_ID);
    assert_eq!(list.plugins[0].status, "ready");
    assert!(list.plugins[0].enabled);

    let status = vault
        .service
        .plugins_status(PLUGIN_ID.into())
        .await
        .unwrap();
    assert_eq!(status.status, "ready");

    let disabled = vault
        .service
        .plugins_disable(PLUGIN_ID.into())
        .await
        .unwrap();
    assert!(!disabled.enabled);
    assert_eq!(disabled.status, "disabled");

    let enabled = vault
        .service
        .plugins_enable(PLUGIN_ID.into())
        .await
        .unwrap();
    assert!(enabled.enabled);
    assert_eq!(enabled.status, "ready");

    let reloaded = vault
        .service
        .plugins_reload(PLUGIN_ID.into())
        .await
        .unwrap();
    assert_eq!(reloaded.id, PLUGIN_ID);
    assert_eq!(reloaded.status, "ready");

    let missing = vault
        .service
        .plugins_status("ghost".into())
        .await
        .unwrap_err();
    assert!(missing.contains("plugin_not_found"), "err: {missing}");

    drop_plugin_runtime(&vault.service);
}

#[tokio::test]
async fn plugins_reload_reports_load_error_for_corrupted_wasm() {
    let vault = create_test_vault();
    write_echo_plugin(&vault);
    let wasm = vault
        .vault_path
        .join(".pkm/plugins")
        .join(PLUGIN_ID)
        .join("plugin.wasm");
    std::fs::write(&wasm, b"garbage wasm bytes").unwrap();
    install_manager(&vault, true);

    let err = vault
        .service
        .plugins_reload(PLUGIN_ID.into())
        .await
        .unwrap_err();
    assert!(err.contains("plugin_load_error"), "err: {err}");

    drop_plugin_runtime(&vault.service);
}

#[tokio::test]
async fn plugins_install_from_path_then_uninstall() {
    let vault = create_test_vault();
    install_manager(&vault, false);

    let src = write_install_source(&vault.vault_path.join("_src"));
    let installed = vault
        .service
        .plugins_install(src.to_string_lossy().to_string())
        .await
        .expect("install ok");
    assert_eq!(installed.id, "com.example.install-e2e");
    assert_eq!(installed.status, "disabled", "fresh install is disabled");

    let missing = vault
        .service
        .plugins_install(
            vault
                .vault_path
                .join("nope.wasm")
                .to_string_lossy()
                .to_string(),
        )
        .await
        .unwrap_err();
    assert!(missing.contains("plugin_not_found"), "err: {missing}");

    let after = vault
        .service
        .plugins_uninstall("com.example.install-e2e".into())
        .await
        .expect("uninstall ok");
    assert!(after.plugins.is_empty());
    assert!(!vault.exists(".pkm/plugins/com.example.install-e2e"));

    drop_plugin_runtime(&vault.service);
}

#[tokio::test]
async fn plugins_install_url_verifies_checksum_and_installs() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, ResponseTemplate};

    let vault = create_test_vault();
    install_manager(&vault, false);

    let wasm = wat::parse_str(INSTALL_PLUGIN_WAT).unwrap();
    let digest = {
        use sha2::{Digest, Sha256};
        format!("{:x}", Sha256::digest(&wasm))
    };
    let server = wiremock::MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/plugin.wasm"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(wasm)
                .insert_header("content-type", "application/wasm"),
        )
        .mount(&server)
        .await;

    let info = vault
        .service
        .plugins_install_url(
            format!("{}/plugin.wasm", server.uri()),
            Some(digest),
            "com.example.catalog".into(),
        )
        .await
        .expect("install_url ok");

    assert_eq!(info.id, "com.example.catalog");
    assert_eq!(info.status, "disabled");

    drop_plugin_runtime(&vault.service);
}

#[tokio::test]
async fn plugins_install_url_rejects_a_checksum_mismatch() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, ResponseTemplate};

    let vault = create_test_vault();
    install_manager(&vault, false);

    let wasm = wat::parse_str(INSTALL_PLUGIN_WAT).unwrap();
    let server = wiremock::MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/plugin.wasm"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(wasm))
        .mount(&server)
        .await;

    let err = vault
        .service
        .plugins_install_url(
            format!("{}/plugin.wasm", server.uri()),
            Some("deadbeef".repeat(8)),
            "com.example.catalog".into(),
        )
        .await
        .unwrap_err();

    assert!(err.contains("plugin_checksum_error"), "err: {err}");
    assert!(vault
        .service
        .plugins_list()
        .await
        .unwrap()
        .plugins
        .is_empty());

    drop_plugin_runtime(&vault.service);
}

#[tokio::test]
async fn plugin_note_read_returns_real_content_and_rejects_missing() {
    let vault = create_test_vault();
    vault.write_file("notes/real.md", "secret content");
    install_manager(&vault, false);

    let note = vault
        .service
        .plugin_note_read("notes/real.md".into())
        .await
        .unwrap();
    assert_eq!(note.path, "notes/real.md");
    assert_eq!(note.content, "secret content");
    assert!(!note.mtime.is_empty());

    let err = vault
        .service
        .plugin_note_read("notes/gone.md".into())
        .await
        .unwrap_err();
    assert!(err.contains("note_not_found"), "err: {err}");

    drop_plugin_runtime(&vault.service);
}

#[tokio::test]
async fn plugin_http_request_hits_the_backend_and_returns_the_body() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, ResponseTemplate};

    let vault = create_test_vault();
    install_manager(&vault, false);
    let server = wiremock::MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/probe"))
        .respond_with(ResponseTemplate::new(200).set_body_string("mock body"))
        .mount(&server)
        .await;

    let response = vault
        .service
        .plugin_http_request(
            Some("GET".into()),
            format!("{}/probe", server.uri()),
            None,
            None,
            None,
        )
        .await
        .unwrap();

    assert_eq!(response.status, 200);
    assert_eq!(response.body, "mock body");

    drop_plugin_runtime(&vault.service);
}

#[tokio::test]
async fn plugin_http_request_blocks_public_hosts_by_default() {
    let vault = create_test_vault();
    install_manager(&vault, false);

    let err = vault
        .service
        .plugin_http_request(
            Some("GET".into()),
            "http://8.8.8.8/x".into(),
            None,
            None,
            None,
        )
        .await
        .unwrap_err();

    assert!(err.contains("http_ssid"), "err: {err}");

    drop_plugin_runtime(&vault.service);
}

#[tokio::test]
async fn hook_dispatch_delivers_expected_payloads() {
    let vault = create_test_vault();
    write_echo_plugin(&vault);
    let manager = install_manager(&vault, true);

    let open = dispatch_on_open(&manager, "pages/a.md");
    assert_eq!(open.len(), 1);
    assert!(open[0].1.is_some(), "echo returns a payload");

    let links = vec!["b".to_string()];
    let linked = dispatch_on_link(&manager, "pages/a.md", &links);
    assert_eq!(linked.len(), 1);

    let search = dispatch_on_search(&manager, "quantum", 10);
    assert_eq!(search.len(), 1);

    // Release the local handle first: the state still holds a clone, so this
    // drop cannot destroy the embedded runtime. `drop_plugin_runtime` then
    // tears down the last reference on a dedicated thread.
    drop(manager);
    drop_plugin_runtime(&vault.service);
}
