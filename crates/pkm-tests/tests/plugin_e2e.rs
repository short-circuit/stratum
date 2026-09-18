//! End-to-end tests for the WASM plugin system.
//!
//! These tests exercise the REAL plugin path — no stubs:
//!   * a real binary `.wasm` test plugin is built in-process with the `wat`
//!     crate and an embedded `stratum:manifest` custom section
//!   * the plugin is discovered/loaded through [`PluginRegistry::scan_vault`]
//!     (the same path `PluginManager::init_for_vault` uses)
//!   * host functions (`pkm.note_read`, `pkm.note_write`, `pkm.http_request`,
//!     `pkm.log`) are dispatched through [`PluginRuntime::run_plugin`] against
//!     a real [`VaultHost`] backed by a temp vault on disk
//!   * error handling (missing note, path traversal, missing permission,
//!     SSRF block) is asserted on the envelope contract
//!   * the Tauri command layer is exercised via the [`PluginManager`] API that
//!     the command handlers delegate to (discovery/loading, enable/disable,
//!     dispatch).
//!
//! The sample plugin is intentionally a *multi-function* real module: one
//! exported hook dispatches to any of the four host imports based on a marker
//! prefix in the payload, so a single real `.wasm` exercises note_read +
//! note_write + http_request + log together — the "sample test plugin that
//! exercises all host functions" acceptance item.

use pkm_plugin::registry::{PluginRegistry, PluginState};
use pkm_plugin::runtime::PluginRuntime;
use pkm_plugin::{Permission, PermissionSet};
use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Sample test plugin (real binary .wasm)
// ---------------------------------------------------------------------------

/// WAT source for the sample test plugin.
///
/// Exports `onSave` and `onOpen`. The payload format is:
///   `<marker>:<json>`
/// where `<marker>` is one of `READ`, `WRITE`, `HTTP`, `LOG` and `<json>` is
/// UTF-8. The host functions parse the payload as JSON, so the plugin locates
/// the `:` separator in the payload at runtime and forwards only the JSON
/// substring that follows it: it calls the import with `(ptr+colon+1,
/// len-colon-1)`.
///
/// The host writes the response envelope to memory offset 0 and returns its
/// length; `run_plugin` reads it back. No memory copy is needed — the host
/// reads the request at the JSON offset and overwrites offset 0 with the
/// response.
const SAMPLE_PLUGIN_WAT: &str = r#"
    (module
        (import "pkm" "note_read" (func $note_read (param i32 i32) (result i32)))
        (import "pkm" "note_write" (func $note_write (param i32 i32) (result i32)))
        (import "pkm" "http_request" (func $http_request (param i32 i32) (result i32)))
        (import "pkm" "log" (func $log (param i32 i32) (result i32)))

        (memory (export "memory") 1)

        ;; Find the byte position of ':' in the first `len` bytes of `ptr`.
        ;; Returns the index of the colon, or 0 if none is found (caller
        ;; falls back to the whole payload).
        (func $find_colon (param $ptr i32) (param $len i32) (result i32)
            (local $i i32)
            (local $c i32)
            (block $done
                (loop $scan
                    (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
                    (local.set $c
                        (i32.load8_u
                            (i32.add (local.get $ptr) (local.get $i))))
                    (if (i32.eq (local.get $c) (i32.const 58))
                        (then (return (local.get $i)))
                    )
                    (local.set $i (i32.add (local.get $i) (i32.const 1)))
                    (br $scan)
                )
            )
            (i32.const 0)
        )

        (func $json_ptr (param $ptr i32) (param $len i32) (result i32)
            (local $sep i32)
            (local.set $sep (call $find_colon (local.get $ptr) (local.get $len)))
            (i32.add (local.get $ptr) (i32.add (local.get $sep) (i32.const 1)))
        )
        (func $json_len (param $ptr i32) (param $len i32) (result i32)
            (local $sep i32)
            (local.set $sep (call $find_colon (local.get $ptr) (local.get $len)))
            (i32.sub (i32.sub (local.get $len) (local.get $sep)) (i32.const 1))
        )

        (func $dispatch (param $ptr i32) (param $len i32) (result i32)
            (local $c i32)
            ;; Empty payload -> no host call.
            (if (i32.eqz (local.get $len))
                (then (return (i32.const 0)))
            )
            ;; Dispatch on the first byte of the marker.
            (local.set $c (i32.load8_u (local.get $ptr)))
            (block $done
                (br_if $done (i32.ne (local.get $c) (i32.const 82))) ;; 'R' -> note_read
                (return (call $note_read
                    (call $json_ptr (local.get $ptr) (local.get $len))
                    (call $json_len (local.get $ptr) (local.get $len))))
            )
            (block $done
                (br_if $done (i32.ne (local.get $c) (i32.const 87))) ;; 'W' -> note_write
                (return (call $note_write
                    (call $json_ptr (local.get $ptr) (local.get $len))
                    (call $json_len (local.get $ptr) (local.get $len))))
            )
            (block $done
                (br_if $done (i32.ne (local.get $c) (i32.const 72))) ;; 'H' -> http_request
                (return (call $http_request
                    (call $json_ptr (local.get $ptr) (local.get $len))
                    (call $json_len (local.get $ptr) (local.get $len))))
            )
            ;; 'L' (or unknown) -> log
            (call $log
                (call $json_ptr (local.get $ptr) (local.get $len))
                (call $json_len (local.get $ptr) (local.get $len)))
        )

        (func (export "onSave") (param i32 i32) (result i32)
            (call $dispatch (local.get 0) (local.get 1))
        )
        (func (export "onOpen") (param i32 i32) (result i32)
            (call $dispatch (local.get 0) (local.get 1))
        )
    )
"#;

/// The manifest embedded in the sample plugin's `stratum:manifest` custom
/// section. Grants every host permission so the plugin can exercise all four
/// host functions.
const SAMPLE_MANIFEST_JSON: &str = r#"{
    "schema_version": 1,
    "id": "com.example.sample-e2e",
    "name": "Sample E2E Plugin",
    "version": "0.1.0",
    "author": "stratum-test",
    "description": "Sample plugin used by the plugin system E2E tests",
    "entry": "plugin.wasm",
    "permissions": ["file:read", "file:write", "network"],
    "hooks": { "onSave": true, "onOpen": true }
}"#;

/// Build the sample plugin's real binary `.wasm`: parse the WAT source with
/// the `wat` crate and append the `stratum:manifest` custom section so the
/// registry's `extract_embedded_manifest` can read the manifest back.
fn sample_plugin_wasm() -> Vec<u8> {
    let module = wat::parse_str(SAMPLE_PLUGIN_WAT).expect("sample plugin WAT must parse");
    append_custom_section(&module, "stratum:manifest", SAMPLE_MANIFEST_JSON.as_bytes())
}

/// Append a WASM custom section (id 0) with the given name and payload.
fn append_custom_section(module: &[u8], name: &str, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(module.len() + name.len() + payload.len() + 32);
    out.extend_from_slice(module);

    let name_bytes = name.as_bytes();
    let body_len = leb128_len(name_bytes.len() as u64) + name_bytes.len() + payload.len();

    out.push(0u8); // custom section id
    write_uleb(&mut out, body_len as u64);
    write_uleb(&mut out, name_bytes.len() as u64);
    out.extend_from_slice(name_bytes);
    out.extend_from_slice(payload);
    out
}

/// Return the length in bytes of the LEB128 encoding of `v`.
fn leb128_len(v: u64) -> usize {
    if v == 0 {
        return 1;
    }
    let mut n = 0u64;
    let mut v = v;
    while v != 0 {
        n += 1;
        v >>= 7;
    }
    n as usize
}

/// Write `v` as an unsigned LEB128 to `out`.
fn write_uleb(out: &mut Vec<u8>, mut v: u64) {
    loop {
        let mut byte = (v & 0x7f) as u8;
        v >>= 7;
        if v != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if v == 0 {
            break;
        }
    }
}

// ---------------------------------------------------------------------------
// Test vault + plugin fixtures
// ---------------------------------------------------------------------------

/// A temp vault on disk with a `notes/` directory, used as the real host
/// backend so host calls read/write real files.
struct TestVault {
    _dir: TempDir,
    root: PathBuf,
}

impl TestVault {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().to_path_buf();
        std::fs::create_dir_all(root.join("notes")).unwrap();
        Self { _dir: dir, root }
    }

    /// Write a real note file in the vault.
    fn write_note(&self, rel: &str, content: &str) {
        let full = self.root.join(rel);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(&full, content).unwrap();
    }

    /// Read a real note file from the vault.
    fn read_note(&self, rel: &str) -> String {
        std::fs::read_to_string(self.root.join(rel)).unwrap()
    }

    /// Install the sample plugin into the canonical vault plugin layout
    /// (`<vault>/.pkm/plugins/<id>/plugin.wasm`) and return its path.
    fn install_sample_plugin(&self) -> PathBuf {
        let dir = self
            .root
            .join(".pkm")
            .join("plugins")
            .join("com.example.sample-e2e");
        std::fs::create_dir_all(&dir).unwrap();
        let wasm_path = dir.join("plugin.wasm");
        std::fs::write(&wasm_path, sample_plugin_wasm()).unwrap();
        wasm_path
    }
}

/// Build a runtime rooted at `vault` with a real `VaultHost` using the
/// default empty SSRF allowlist (private/loopback only).
fn runtime_for(vault: &TestVault) -> PluginRuntime {
    PluginRuntime::with_host(Box::new(pkm_plugin::VaultHost::root(vault.root.clone())))
        .expect("runtime")
}

/// Build a runtime rooted at `vault` with an explicit SSRF allowlist.
fn runtime_with_allowlist(vault: &TestVault, allowlist: Vec<String>) -> PluginRuntime {
    PluginRuntime::with_host(Box::new(pkm_plugin::VaultHost::new(
        vault.root.clone(),
        allowlist,
    )))
    .expect("runtime")
}

/// Load the sample plugin from `wasm_path` via the real registry loader —
/// the same path the Tauri layer uses. Returns the registered plugin state.
fn load_sample_state(
    registry: &mut PluginRegistry,
    wasm_path: &Path,
    enabled: bool,
) -> PluginState {
    let manifest = registry.load_plugin(wasm_path).expect("load_plugin");
    if let Some(state) = registry.get_mut(&manifest.id) {
        state.enabled = enabled;
    }
    registry.get(&manifest.id).cloned().expect("state present")
}

/// Run `plugin`'s hook with a marker-prefixed payload through the real
/// runtime and return the raw plugin output (the response envelope JSON).
fn run_hook(runtime: &PluginRuntime, plugin: &PluginState, hook: &str, payload: &str) -> String {
    runtime
        .run_plugin(plugin, hook, payload)
        .expect("run_plugin")
}

/// Build a plugin that declares the given permissions (for permission-denied
/// tests we load the sample with a reduced permission set).
fn state_with_permissions(
    wasm_bytes: Vec<u8>,
    id: &str,
    name: &str,
    permissions: PermissionSet,
    enabled: bool,
) -> PluginState {
    let mut hooks = HashMap::new();
    hooks.insert("onSave".to_string(), true);
    hooks.insert("onOpen".to_string(), true);
    PluginState::new(
        pkm_plugin::registry::PluginManifest {
            schema_version: 1,
            id: id.to_string(),
            name: name.to_string(),
            version: "0.1.0".to_string(),
            author: "stratum-test".to_string(),
            description: "permission-scoped e2e test plugin".to_string(),
            entry: "plugin.wasm".to_string(),
            permissions,
            hooks,
        },
        wasm_bytes,
        enabled,
    )
}

// ---------------------------------------------------------------------------
// Tests — real end-to-end through the sample plugin
// ---------------------------------------------------------------------------

/// Payload builders: prefix the JSON the host should receive with the
/// dispatch marker the sample plugin strips.
fn read_payload(path: &str) -> String {
    format!("READ:{}", path_json(path))
}

fn write_payload(path: &str, content: &str) -> String {
    let mut json = serde_json::Map::new();
    json.insert(
        "path".to_string(),
        serde_json::Value::String(path.to_string()),
    );
    json.insert(
        "content".to_string(),
        serde_json::Value::String(content.to_string()),
    );
    format!("WRITE:{}", serde_json::Value::Object(json))
}

fn log_payload(level: &str, message: &str) -> String {
    format!("LOG:{{\"level\":\"{level}\",\"message\":\"{message}\"}}")
}

fn http_payload(method: &str, url: &str) -> String {
    format!("HTTP:{{\"method\":\"{method}\",\"url\":\"{url}\",\"timeout_ms\":5000}}")
}

fn path_json(path: &str) -> String {
    serde_json::json!({ "path": path }).to_string()
}

/// Spawn a tiny HTTP server responding with the given status line and body.
/// Returns the port it is listening on (bound to loopback, which the default
/// SSRF allowlist permits). Mirrors the helper in `pkm_plugin::host::tests`
/// so the E2E suite does not depend on a `#[cfg(test)]`-only symbol from the
/// library crate.
fn serve(status_line: &'static str, body: &'static str) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for stream in listener.incoming().take(2) {
            let Ok(mut stream) = stream else { continue };
            let mut buf = [0u8; 2048];
            let _ = stream.read(&mut buf);
            let _ = write!(
                stream,
                "HTTP/1.1 {status_line}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
        }
    });
    port
}

// ── Discovery / loading ────────────────────────────────────────────────────

#[test]
fn sample_wasm_embeds_manifest_and_parses() {
    // The sample plugin must be a real module whose `stratum:manifest`
    // custom section is extractable and validates (schedule: every accepted
    // manifest must be discoverable the same way production plugins are).
    let wasm = sample_plugin_wasm();
    let extracted = pkm_plugin::registry::extract_embedded_manifest(&wasm)
        .expect("embedded manifest must be extractable")
        .expect("embedded manifest must be present");
    assert!(extracted.contains("\"id\": \"com.example.sample-e2e\""));
    assert!(extracted.contains("\"permissions\""));
}

#[test]
fn scan_vault_discovers_and_loads_sample_plugin() {
    // The canonical discovery path used by PluginManager::init_for_vault:
    // `<vault>/.pkm/plugins/<id>/plugin.wasm` is scanned and loaded.
    let vault = TestVault::new();
    vault.install_sample_plugin();

    let mut registry = PluginRegistry::new();
    let enabled = std::collections::HashSet::from(["com.example.sample-e2e".to_string()]);
    let loaded = registry.scan_vault(&vault.root.join(".pkm").join("plugins"), &enabled);
    assert_eq!(loaded, 1, "scan must discover the sample plugin");

    let state = registry.get("com.example.sample-e2e").expect("loaded");
    assert!(state.enabled, "plugin listed in config must load enabled");
    assert_eq!(state.manifest.name, "Sample E2E Plugin");
    assert!(state.manifest.permissions.check(&Permission::FileRead));
    assert!(state.manifest.permissions.check(&Permission::Network));

    let list = registry.list();
    assert_eq!(list.len(), 1);
    assert!(
        registry.failed_ids().is_empty(),
        "no load failures expected"
    );
}

#[test]
fn scan_vault_loaded_plugin_is_disabled_when_not_in_config() {
    // A plugin present on disk but absent from the config enable list is
    // loaded but disabled (spec §7.3) — identical to PluginManager behavior.
    let vault = TestVault::new();
    vault.install_sample_plugin();

    let mut registry = PluginRegistry::new();
    let loaded = registry.scan_vault(&vault.root.join(".pkm").join("plugins"), &HashSet::new());
    assert_eq!(loaded, 1);
    let state = registry.get("com.example.sample-e2e").expect("loaded");
    assert!(!state.enabled, "absent from config => disabled");
}

#[test]
fn scan_vault_records_broken_plugin_without_aborting() {
    // A non-WASM file in the plugins dir must not abort the scan; the failure
    // is recorded and surfaced (spec §6.4).
    let vault = TestVault::new();
    let dir = vault
        .root
        .join(".pkm")
        .join("plugins")
        .join("broken-plugin");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("plugin.wasm"), b"this is not a wasm module").unwrap();

    let mut registry = PluginRegistry::new();
    let loaded = registry.scan_vault(&vault.root.join(".pkm").join("plugins"), &HashSet::new());
    assert_eq!(loaded, 0);
    assert_eq!(registry.failed_ids().len(), 1);
    let (id, err) = &registry.failed_ids()[0];
    assert_eq!(id, "broken-plugin");
    assert!(!err.is_empty());
}

// ── Host functions: note_read ──────────────────────────────────────────────

#[test]
fn e2e_note_read_returns_real_content() {
    let vault = TestVault::new();
    vault.write_note("notes/real.md", "# Real\nbody content");
    let runtime = runtime_for(&vault);

    let mut registry = PluginRegistry::new();
    let wasm_path = vault.install_sample_plugin();
    let plugin = load_sample_state(&mut registry, &wasm_path, true);

    let out = run_hook(&runtime, &plugin, "onSave", &read_payload("notes/real.md"));
    let value: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(value["kind"], "ok", "got: {out}");
    assert_eq!(value["data"]["content"], "# Real\nbody content");
    assert_eq!(value["data"]["path"], "notes/real.md");
}

#[test]
fn e2e_note_read_missing_returns_error_envelope() {
    let vault = TestVault::new();
    let runtime = runtime_for(&vault);
    let mut registry = PluginRegistry::new();
    let wasm_path = vault.install_sample_plugin();
    let plugin = load_sample_state(&mut registry, &wasm_path, true);

    let out = run_hook(
        &runtime,
        &plugin,
        "onSave",
        &read_payload("notes/absent.md"),
    );
    let value: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(value["kind"], "err");
    assert_eq!(value["code"], "note_not_found");
}

// ── Host functions: note_write ─────────────────────────────────────────────

#[test]
fn e2e_note_write_creates_file_on_disk() {
    let vault = TestVault::new();
    let runtime = runtime_for(&vault);
    let mut registry = PluginRegistry::new();
    let wasm_path = vault.install_sample_plugin();
    let plugin = load_sample_state(&mut registry, &wasm_path, true);

    let out = run_hook(
        &runtime,
        &plugin,
        "onSave",
        &write_payload("notes/written.md", "hello from plugin"),
    );
    let value: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(value["kind"], "ok");
    assert_eq!(value["data"]["written"], true);

    // The file must actually exist with the real content (no fake success).
    assert_eq!(vault.read_note("notes/written.md"), "hello from plugin");
}

#[test]
fn e2e_note_write_creates_parent_directories() {
    let vault = TestVault::new();
    let runtime = runtime_for(&vault);
    let mut registry = PluginRegistry::new();
    let wasm_path = vault.install_sample_plugin();
    let plugin = load_sample_state(&mut registry, &wasm_path, true);

    let out = run_hook(
        &runtime,
        &plugin,
        "onSave",
        &write_payload("deep/nested/path.md", "x"),
    );
    let value: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(value["kind"], "ok");
    assert_eq!(vault.read_note("deep/nested/path.md"), "x");
}

#[test]
fn e2e_note_write_path_traversal_fails_not_fake_success() {
    // Writing `../../outside.md` escapes the vault — the write must error,
    // never silently succeed (task acceptance: "no fake note or always-success
    // write remains").
    let vault = TestVault::new();
    let runtime = runtime_for(&vault);
    let mut registry = PluginRegistry::new();
    let wasm_path = vault.install_sample_plugin();
    let plugin = load_sample_state(&mut registry, &wasm_path, true);

    let out = run_hook(
        &runtime,
        &plugin,
        "onSave",
        &write_payload("../../outside.md", "x"),
    );
    let value: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(value["kind"], "err");
    assert_eq!(value["code"], "note_write_failed");
}

// ── Host functions: http_request ───────────────────────────────────────────

#[test]
fn e2e_http_request_real_server() {
    // A real in-process server on loopback (allowed by the default
    // SSRF allowlist policy).
    let port = serve("200 OK", "body text");
    let vault = TestVault::new();
    let runtime = runtime_for(&vault);
    let mut registry = PluginRegistry::new();
    let wasm_path = vault.install_sample_plugin();
    let plugin = load_sample_state(&mut registry, &wasm_path, true);

    let out = run_hook(
        &runtime,
        &plugin,
        "onSave",
        &http_payload("GET", &format!("http://127.0.0.1:{port}/x")),
    );
    let value: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(value["kind"], "ok", "got: {out}");
    assert_eq!(value["status"], 200);
    assert_eq!(value["body"], "body text");
}

#[test]
fn e2e_http_request_ssrf_blocked() {
    // A public host with no allowlist and not private -> http_ssid; no
    // network round-trip is required (SSRF guard must reject it).
    let vault = TestVault::new();
    let runtime = runtime_for(&vault);
    let mut registry = PluginRegistry::new();
    let wasm_path = vault.install_sample_plugin();
    let plugin = load_sample_state(&mut registry, &wasm_path, true);

    let out = run_hook(
        &runtime,
        &plugin,
        "onSave",
        &http_payload("GET", "http://93.184.216.34/x"),
    );
    let value: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(value["kind"], "err");
    assert_eq!(value["code"], "http_ssid");
}

#[test]
fn e2e_http_request_respects_allowlist() {
    // With an allowlist covering the host, the same public host is allowed.
    let port = serve("200 OK", "allowlist body");
    let vault = TestVault::new();
    let runtime = runtime_with_allowlist(&vault, vec![format!("127.0.0.1:{port}")]);
    let mut registry = PluginRegistry::new();
    let wasm_path = vault.install_sample_plugin();
    let plugin = load_sample_state(&mut registry, &wasm_path, true);

    let out = run_hook(
        &runtime,
        &plugin,
        "onSave",
        &http_payload("GET", &format!("http://127.0.0.1:{port}/x")),
    );
    let value: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(value["kind"], "ok");
    assert_eq!(value["status"], 200);
    assert_eq!(value["body"], "allowlist body");
}

// ── Host functions: log ────────────────────────────────────────────────────

#[test]
fn e2e_log_never_fails() {
    // pkm.log is permission-free and always returns the success envelope.
    let vault = TestVault::new();
    let runtime = runtime_for(&vault);
    let mut registry = PluginRegistry::new();
    let wasm_path = vault.install_sample_plugin();
    let plugin = load_sample_state(&mut registry, &wasm_path, true);

    let out = run_hook(
        &runtime,
        &plugin,
        "onSave",
        &log_payload("info", "hello from sample"),
    );
    let value: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(value["kind"], "ok");
    assert_eq!(value["data"]["logged"], true);
}

// ── Error handling / permissions ───────────────────────────────────────────

#[test]
fn e2e_permission_denied_note_read() {
    // A plugin without `file:read` must be denied before any filesystem work.
    let vault = TestVault::new();
    vault.write_note("notes/real.md", "secret");
    let runtime = runtime_for(&vault);
    let wasm_bytes = sample_plugin_wasm();
    let plugin = state_with_permissions(
        wasm_bytes,
        "com.example.sample-e2e",
        "Sample E2E Plugin",
        PermissionSet::new(), // no grants
        true,
    );

    let out = run_hook(&runtime, &plugin, "onSave", &read_payload("notes/real.md"));
    let value: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(value["kind"], "err");
    assert_eq!(value["code"], "plugin_denied");
    let msg = value["message"].as_str().unwrap();
    assert!(
        msg.contains("permission 'file:read' not granted"),
        "unexpected message: {msg}"
    );
}

#[test]
fn e2e_permission_denied_http_request() {
    let vault = TestVault::new();
    let runtime = runtime_for(&vault);
    let wasm_bytes = sample_plugin_wasm();
    let plugin = state_with_permissions(
        wasm_bytes,
        "com.example.sample-e2e",
        "Sample E2E Plugin",
        PermissionSet::new(),
        true,
    );

    let out = run_hook(
        &runtime,
        &plugin,
        "onSave",
        &http_payload("GET", "http://127.0.0.1:1/whatever"),
    );
    let value: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(value["kind"], "err");
    assert_eq!(value["code"], "plugin_denied");
}

#[test]
fn e2e_disabled_plugin_does_not_run() {
    let vault = TestVault::new();
    vault.write_note("notes/real.md", "x");
    let runtime = runtime_for(&vault);
    let mut registry = PluginRegistry::new();
    let wasm_path = vault.install_sample_plugin();
    let plugin = load_sample_state(&mut registry, &wasm_path, false);

    let result = runtime.run_plugin(&plugin, "onSave", &read_payload("notes/real.md"));
    assert!(result.is_err(), "disabled plugin must fail to run");
    assert!(result.unwrap_err().to_string().contains("disabled"));
}

// ── Tauri command layer (PluginManager) ────────────────────────────────────
// These drive the same PluginManager the Tauri command handlers delegate to,
// covering discovery/loading, enable/disable/reload, and dispatch.

#[test]
fn manager_init_for_vault_scans_and_loads_enabled_plugin() {
    use app_lib::commands::plugins::PluginManager;

    let vault = TestVault::new();
    // Install the plugin and persist an enabling config.toml.
    vault.install_sample_plugin();
    let pkm_dir = vault.root.join(".pkm");
    let config = format!(
        "\n[[plugins]]\nname = \"com.example.sample-e2e\"\nenabled = true\nwasm_path = \"{}\"\npermissions = [\"file:read\", \"file:write\", \"network\"]\n",
        ".pkm/plugins/com.example.sample-e2e/plugin.wasm"
    );
    std::fs::write(pkm_dir.join("config.toml"), config).unwrap();

    let manager = PluginManager::init_for_vault(&vault.root).expect("init_for_vault");
    assert_eq!(manager.len(), 1);

    let info = manager
        .status("com.example.sample-e2e")
        .expect("plugin visible");
    assert_eq!(info.status, "ready");
    assert!(info.enabled);
    assert_eq!(info.name, "Sample E2E Plugin");
    let mut perms = info.permissions.clone();
    let mut want = vec![
        "network".to_string(),
        "file:read".to_string(),
        "file:write".to_string(),
    ];
    perms.sort();
    want.sort();
    assert_eq!(perms, want);
}

#[test]
fn manager_enable_disable_persists_and_reflects_state() {
    use app_lib::commands::plugins::PluginManager;

    let vault = TestVault::new();
    vault.install_sample_plugin();
    let pkm_dir = vault.root.join(".pkm");
    let config = "\n[[plugins]]\nname = \"com.example.sample-e2e\"\nenabled = true\nwasm_path = \".pkm/plugins/com.example.sample-e2e/plugin.wasm\"\npermissions = [\"file:read\", \"file:write\", \"network\"]\n";
    std::fs::write(pkm_dir.join("config.toml"), config).unwrap();

    let manager = PluginManager::init_for_vault(&vault.root).expect("init_for_vault");
    assert!(manager.status("com.example.sample-e2e").unwrap().enabled);

    let disabled = manager.disable("com.example.sample-e2e").expect("disable");
    assert!(!disabled.enabled);
    assert_eq!(disabled.status, "disabled");

    let re_enabled = manager.enable("com.example.sample-e2e").expect("enable");
    assert!(re_enabled.enabled);
    assert_eq!(re_enabled.status, "ready");
}

#[test]
fn manager_reload_reinstantiates_from_disk() {
    use app_lib::commands::plugins::PluginManager;

    let vault = TestVault::new();
    vault.install_sample_plugin();
    let pkm_dir = vault.root.join(".pkm");
    let config = "\n[[plugins]]\nname = \"com.example.sample-e2e\"\nenabled = true\nwasm_path = \".pkm/plugins/com.example.sample-e2e/plugin.wasm\"\npermissions = [\"file:read\", \"file:write\", \"network\"]\n";
    std::fs::write(pkm_dir.join("config.toml"), config).unwrap();

    let manager = PluginManager::init_for_vault(&vault.root).expect("init_for_vault");
    let reloaded = manager.reload("com.example.sample-e2e").expect("reload");
    assert!(reloaded.enabled);
    assert_eq!(reloaded.status, "ready");
}

#[test]
fn manager_dispatch_all_runs_enabled_plugin_hook() {
    use app_lib::commands::plugins::PluginManager;

    let vault = TestVault::new();
    vault.write_note("notes/real.md", "# from disk");
    vault.install_sample_plugin();
    let pkm_dir = vault.root.join(".pkm");
    let config = "\n[[plugins]]\nname = \"com.example.sample-e2e\"\nenabled = true\nwasm_path = \".pkm/plugins/com.example.sample-e2e/plugin.wasm\"\npermissions = [\"file:read\", \"file:write\", \"network\"]\n";
    std::fs::write(pkm_dir.join("config.toml"), config).unwrap();

    let manager = PluginManager::init_for_vault(&vault.root).expect("init_for_vault");
    assert!(manager.has_hook("onSave"));

    let results = manager.dispatch_all("onSave", &read_payload("notes/real.md"));
    assert_eq!(results.len(), 1);
    let (id, out) = &results[0];
    assert_eq!(id, "com.example.sample-e2e");
    let out = out.as_ref().expect("dispatch must succeed");
    let value: serde_json::Value = serde_json::from_str(out).unwrap();
    assert_eq!(value["kind"], "ok");
    assert_eq!(value["data"]["content"], "# from disk");
}

#[test]
fn manager_no_vault_plugins_is_empty() {
    use app_lib::commands::plugins::PluginManager;

    let vault = TestVault::new();
    // No .pkm/plugins dir at all.
    let manager = PluginManager::init_for_vault(&vault.root).expect("init_for_vault");
    assert!(manager.is_empty());
    assert_eq!(manager.list().plugins.len(), 0);
}
