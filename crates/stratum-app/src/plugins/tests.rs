//! Plugin manager lifecycle + catalog-install tests.
//!
//! These run headlessly (no Tauri runtime): they drive [`PluginManager`] and
//! [`install_url`](super::install::install_url) directly.

use super::*;

/// Minimal valid WASM module (empty, no exports) — sufficient for
/// `scan_vault` to load/register a plugin. Manifest comes from the sidecar.
const EMPTY_MODULE: &[u8] = &[0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];

/// Sidecar manifest for a plugin with `file:read` + `network` and an
/// `onSave` hook. Placed as `<id>.wasm.manifest.json` beside the wasm.
const SIDECAR_MANIFEST: &str = r#"{
    "schema_version": 1,
    "id": "com.example.scan-test",
    "name": "Scan Test",
    "version": "0.1.0",
    "description": "integration test",
    "author": "test",
    "entry": "scan-test.wasm",
    "permissions": ["file:read", "network"],
    "hooks": { "onSave": true }
}"#;

/// Build `<vault>/.pkm/plugins/<id>/` containing a wasm module and its
/// sidecar manifest, mirroring the legacy layout `scan_vault` accepts.
/// Returns the plugin directory, or panics.
fn write_plugin_dir(
    vault: &std::path::Path,
    wasm_bytes: &[u8],
    manifest: &str,
) -> std::path::PathBuf {
    let plugin_dir = vault.join(".pkm").join("plugins").join("scan-test");
    std::fs::create_dir_all(&plugin_dir).expect("mkdir plugin dir");
    std::fs::write(plugin_dir.join("scan-test.wasm"), wasm_bytes).expect("write wasm");
    std::fs::write(plugin_dir.join("scan-test.wasm.manifest.json"), manifest)
        .expect("write manifest");
    plugin_dir
}

/// Write `<vault>/.pkm/config.toml` with the plugin enabled. Enablement
/// keys off the manifest `id` via the `[[plugins]]` list (§7.3).
fn write_enabling_config(vault: &std::path::Path) {
    let pkm_dir = vault.join(".pkm");
    std::fs::create_dir_all(&pkm_dir).expect("mkdir .pkm");
    let toml = r#"
[[plugins]]
name = "com.example.scan-test"
enabled = true
wasm_path = ".pkm/plugins/scan-test/scan-test.wasm"
permissions = ["file:read", "network"]
"#;
    std::fs::write(pkm_dir.join("config.toml"), toml).expect("write config");
}

#[test]
fn init_for_vault_scans_plugins_dir_and_loads_plugins() {
    let vault = tempfile::tempdir().expect("tempdir");
    write_plugin_dir(vault.path(), EMPTY_MODULE, SIDECAR_MANIFEST);
    write_enabling_config(vault.path());

    let manager = PluginManager::init_for_vault(vault.path()).expect("init_for_vault");
    assert_eq!(manager.len(), 1, "scan must register one plugin");
    assert!(!manager.is_empty());

    let info = manager
        .status("com.example.scan-test")
        .expect("plugin must be visible after scan");
    assert_eq!(info.status, "ready", "enabled plugin must load ready");
    assert!(info.enabled);
    assert_eq!(info.name, "Scan Test");
    let mut perms = info.permissions.clone();
    let mut want = vec!["file:read".to_string(), "network".to_string()];
    perms.sort();
    want.sort();
    assert_eq!(perms, want, "permission set must be surfaced");
}

#[test]
fn init_for_vault_scanned_plugin_defaults_to_disabled() {
    // A plugin present on disk but absent from the config `[[plugins]]`
    // enable list is loaded but disabled (spec §7.3).
    let vault = tempfile::tempdir().expect("tempdir");
    write_plugin_dir(vault.path(), EMPTY_MODULE, SIDECAR_MANIFEST);

    let manager = PluginManager::init_for_vault(vault.path()).expect("init_for_vault");
    let info = manager
        .status("com.example.scan-test")
        .expect("plugin must be visible after scan");
    assert_eq!(info.status, "disabled");
    assert!(!info.enabled);
}

#[test]
fn init_for_vault_with_no_plugins_dir_is_empty() {
    // A vault with no `.pkm/plugins/` directory must not fail or panic.
    let vault = tempfile::tempdir().expect("tempdir");
    let manager = PluginManager::init_for_vault(vault.path()).expect("init_for_vault");
    assert!(manager.is_empty());
    assert_eq!(manager.len(), 0);
    assert!(manager.list().plugins.is_empty());
}

#[test]
fn init_for_vault_records_broken_plugin_as_error() {
    let vault = tempfile::tempdir().expect("tempdir");
    // A wasm that is not a valid module — the plugin must not abort the
    // scan; it surfaces as status "error". The failure is keyed by the
    // on-disk directory name (scan_vault cannot read an id from a module
    // that fails to load).
    let _plugin_dir = write_plugin_dir(vault.path(), b"not a wasm module at all", SIDECAR_MANIFEST);

    let manager = PluginManager::init_for_vault(vault.path()).expect("init_for_vault");
    let info = manager
        .status("scan-test")
        .expect("failed plugin still visible");
    assert_eq!(info.status, "error");
    assert!(info.error.is_some(), "error detail must be surfaced");
}

/// Write a source `.wasm` + sidecar manifest into a temp dir so it can be
/// installed via `PluginManager::install`. Returns the wasm path.
fn write_source_plugin(src_dir: &std::path::Path) -> std::path::PathBuf {
    std::fs::create_dir_all(src_dir).expect("mkdir src dir");
    let wasm_path = src_dir.join("to-install.wasm");
    std::fs::write(&wasm_path, EMPTY_MODULE).expect("write source wasm");
    std::fs::write(
        src_dir.join("to-install.wasm.manifest.json"),
        SIDECAR_MANIFEST,
    )
    .expect("write source manifest");
    wasm_path
}

#[test]
fn install_registers_disabled_and_persists_to_config() {
    let vault = tempfile::tempdir().expect("tempdir");
    let manager = PluginManager::init_for_vault(vault.path()).expect("init_for_vault");
    assert!(manager.is_empty());

    let src = write_source_plugin(&vault.path().join("_src"));
    let info = manager.install(&src).expect("install must succeed");

    assert_eq!(info.id, "com.example.scan-test");
    assert_eq!(info.status, "disabled", "fresh install is disabled");
    assert!(!info.enabled);
    assert_eq!(manager.len(), 1);

    // Files landed in the canonical layout.
    let dest = vault
        .path()
        .join(".pkm")
        .join("plugins")
        .join("com.example.scan-test")
        .join("plugin.wasm");
    assert!(
        dest.is_file(),
        "installed wasm must exist at the canonical path"
    );

    // Config now lists the plugin as disabled.
    let config = pkm_core::Config::load(vault.path().join(".pkm").join("config.toml")).unwrap();
    let entry = config
        .plugins
        .iter()
        .find(|p| p.name == "com.example.scan-test")
        .expect("config must contain the installed plugin");
    assert!(!entry.enabled);
}

#[test]
fn install_missing_source_errors() {
    let vault = tempfile::tempdir().expect("tempdir");
    let manager = PluginManager::init_for_vault(vault.path()).expect("init_for_vault");
    let missing = vault.path().join("does-not-exist.wasm");
    let err = manager.install(&missing).expect_err("must fail");
    assert!(err.contains("plugin_not_found"), "err: {err}");
}

#[test]
fn install_invalid_wasm_errors() {
    let vault = tempfile::tempdir().expect("tempdir");
    let manager = PluginManager::init_for_vault(vault.path()).expect("init_for_vault");
    let src_dir = vault.path().join("_src");
    std::fs::create_dir_all(&src_dir).unwrap();
    let bad = src_dir.join("bad.wasm");
    std::fs::write(&bad, b"not wasm").unwrap();
    std::fs::write(
        src_dir.join("bad.wasm.manifest.json"),
        r#"{
            "schema_version": 1,
            "id": "com.example.bad",
            "name": "Bad",
            "version": "0.1.0",
            "entry": "bad.wasm",
            "permissions": [],
            "hooks": {}
        }"#,
    )
    .unwrap();
    let err = manager.install(&bad).expect_err("invald module must fail");
    assert!(err.contains("plugin_load_error"), "err: {err}");
}

#[test]
fn uninstall_removes_plugin_dir_and_config_entry() {
    let vault = tempfile::tempdir().expect("tempdir");
    let manager = PluginManager::init_for_vault(vault.path()).expect("init_for_vault");
    let src = write_source_plugin(&vault.path().join("_src"));
    manager.install(&src).expect("install");
    assert_eq!(manager.len(), 1);

    let result = manager
        .uninstall("com.example.scan-test")
        .expect("uninstall");
    assert!(manager.is_empty(), "registry must be empty after uninstall");
    assert!(result.plugins.is_empty(), "updated list must be empty");

    let dest_dir = vault
        .path()
        .join(".pkm")
        .join("plugins")
        .join("com.example.scan-test");
    assert!(!dest_dir.exists(), "plugin dir must be removed");

    let config = pkm_core::Config::load(vault.path().join(".pkm").join("config.toml")).unwrap();
    assert!(
        config
            .plugins
            .iter()
            .all(|p| p.name != "com.example.scan-test"),
        "config entry must be removed"
    );
}

// -----------------------------------------------------------------------
// install_url — catalog install (download → sha256 verify → stage → install)
// -----------------------------------------------------------------------

/// Run a futures-style future to completion on a throwaway current-thread
/// runtime (mirrors `tests/plugin_commands.rs::block_on`). Sync `#[test]`
/// fns are used because dropping the wasmtime-backed `PluginRuntime` from
/// within an async context panics ("Cannot drop a runtime …").
fn block_on<F: std::future::Future>(fut: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime")
        .block_on(fut)
}

/// Wiremock-backed URL for `install_url` serving `body` at `/plugin.wasm`.
/// The server is boxed + leaked and reclaimed at process exit (a wiremock
/// server owns a runtime, which must not be dropped in async contexts).
fn serve_bytes(body: Vec<u8>) -> (String, &'static wiremock::MockServer) {
    let server: &'static wiremock::MockServer =
        Box::leak(Box::new(block_on(wiremock::MockServer::start())));
    block_on(
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/plugin.wasm"))
            .respond_with(
                wiremock::ResponseTemplate::new(200)
                    .set_body_bytes(body)
                    .insert_header("content-type", "application/wasm"),
            )
            .mount(server),
    );
    (format!("{}/plugin.wasm", server.uri()), server)
}

#[test]
fn install_url_downloads_verifies_and_installs() {
    let vault = tempfile::tempdir().expect("tempdir");
    let manager = PluginManager::init_for_vault(vault.path()).expect("init_for_vault");

    // Build a source plugin (wasm + sidecar) and read its bytes + digest
    // so the URL handler receives exactly what install would receive.
    let src = write_source_plugin(&vault.path().join("_src"));
    let bytes = std::fs::read(&src).unwrap();
    let digest = {
        use sha2::{Digest, Sha256};
        format!("{:x}", Sha256::digest(&bytes))
    };
    let (url, _server) = serve_bytes(bytes);

    let info = block_on(install_url(
        &manager,
        &url,
        Some(&digest),
        "com.example.scan-test",
    ))
    .expect("install_url must succeed");
    assert_eq!(info.id, "com.example.scan-test");
    assert_eq!(info.status, "disabled", "fresh catalog install is disabled");
    assert_eq!(manager.len(), 1);

    let dest = vault
        .path()
        .join(".pkm")
        .join("plugins")
        .join("com.example.scan-test")
        .join("plugin.wasm");
    assert!(
        dest.is_file(),
        "installed wasm must exist at canonical path"
    );
}

#[test]
fn install_url_rejects_checksum_mismatch() {
    let vault = tempfile::tempdir().expect("tempdir");
    let manager = PluginManager::init_for_vault(vault.path()).expect("init_for_vault");

    let src = write_source_plugin(&vault.path().join("_src"));
    let bytes = std::fs::read(&src).unwrap();
    let (url, _server) = serve_bytes(bytes);

    let wrong = "deadbeef".repeat(8); // 64 hex chars, all wrong
    let err = block_on(install_url(
        &manager,
        &url,
        Some(&wrong),
        "com.example.scan-test",
    ))
    .expect_err("mismatch must fail");
    assert!(err.contains("plugin_checksum_error"), "err: {err}");
    assert!(manager.is_empty(), "nothing must be installed on mismatch");
}

#[test]
fn install_url_without_expected_checksum_installs() {
    let vault = tempfile::tempdir().expect("tempdir");
    let manager = PluginManager::init_for_vault(vault.path()).expect("init_for_vault");

    let src = write_source_plugin(&vault.path().join("_src"));
    let bytes = std::fs::read(&src).unwrap();
    let (url, _server) = serve_bytes(bytes);

    let info = block_on(install_url(&manager, &url, None, "com.example.scan-test"))
        .expect("must install without checksum");
    assert_eq!(info.id, "com.example.scan-test");
}

#[test]
fn install_url_reports_download_error_for_missing_server() {
    // A bound-then-closed port: connecting is refused → download error.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    drop(listener);

    let vault = tempfile::tempdir().expect("tempdir");
    let manager = PluginManager::init_for_vault(vault.path()).expect("init_for_vault");
    let err = block_on(install_url(
        &manager,
        &format!("http://{addr}/plugin.wasm"),
        None,
        "com.example.scan-test",
    ))
    .expect_err("connection refused must fail");
    assert!(err.contains("plugin_download_error"), "err: {err}");
    assert!(manager.is_empty());
}
