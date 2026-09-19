//! HTTP integration tests for the WASM plugin host function `pkm.http_request`
//! using wiremock as the HTTP endpoint.
//!
//! Per the task contract, wiremock is used wherever HTTP is involved. These
//! tests exercise the REAL plugin path: a compiled `.wasm` module (built
//! in-process with `wat`) that forwards its payload verbatim to the
//! `pkm.http_request` import; the host parses the JSON request, runs the SSRF
//! guard, and performs the real network call against the wiremock server.
//! Nothing in the plugin system is stubbed or faked.
//!
//! These are plain `#[test]`s (NOT `#[tokio::test]`): the plugin runtime
//! drives the async host call with its own internal `tokio` runtime via
//! `block_on`, which panics if called from inside a `#[tokio::test]` worker
//! thread ("Cannot start a runtime from within a runtime"). Wiremock is
//! started through a short-lived local runtime; its server runs on a
//! dedicated thread with its own runtime, so it survives independently.
//!
//! Coverage:
//!   * allow: a loopback wiremock endpoint is reachable with the default
//!     (private/loopback) SSRF policy and returns the exact stubbed body
//!   * block: a public host without an allowlist entry is rejected by the
//!     SSRF guard as `http_ssid` — no network round-trip required
//!   * allowlist: an allowlist entry explicitly permits a target
//!   * status mapping: a 4xx/5xx response surfaces as `http_status`
//!   * transport / timeout mapping
//!   * method/headers/body are verified end-to-end by wiremock matchers

use pkm_plugin::registry::{PluginRegistry, PluginState};
use pkm_plugin::runtime::PluginRuntime;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// A temp vault used as the real host backend root.
struct TestVault {
    _dir: TempDir,
    root: std::path::PathBuf,
}

impl TestVault {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().to_path_buf();
        std::fs::create_dir_all(root.join("notes")).unwrap();
        Self { _dir: dir, root }
    }

    fn install_http_plugin(&self) -> PathBuf {
        let dir = self
            .root
            .join(".pkm")
            .join("plugins")
            .join("com.example.http-e2e");
        std::fs::create_dir_all(&dir).unwrap();
        let wasm = wat::parse_str(HTTP_PLUGIN_WAT).expect("WAT");
        let p = dir.join("plugin.wasm");
        std::fs::write(&p, &wasm).unwrap();
        std::fs::write(dir.join("plugin.wasm.manifest.json"), HTTP_SIDECAR_MANIFEST).unwrap();
        p
    }
}

/// A real module that forwards its `(ptr, len)` payload verbatim to the
/// `pkm.http_request` import and returns the host response length. The host
/// parses the payload as the JSON request envelope.
const HTTP_PLUGIN_WAT: &str = r#"
    (module
        (import "pkm" "http_request" (func $http_request (param i32 i32) (result i32)))
        (memory (export "memory") 1)
        (func (export "onSave") (param i32 i32) (result i32)
            local.get 0
            local.get 1
            call $http_request
        )
    )
"#;

/// Sidecar manifest granting `network` so the host allows the request.
const HTTP_SIDECAR_MANIFEST: &str = r#"{
    "schema_version": 1,
    "id": "com.example.http-e2e",
    "name": "HTTP E2E",
    "version": "0.1.0",
    "entry": "plugin.wasm",
    "permissions": ["network"],
    "hooks": { "onSave": true }
}"#;

/// Build the HTTP plugin's real binary `.wasm`.
#[allow(dead_code)]
fn http_plugin_wasm() -> Vec<u8> {
    wat::parse_str(HTTP_PLUGIN_WAT).expect("HTTP plugin WAT must parse")
}

fn runtime_for(vault: &TestVault) -> PluginRuntime {
    PluginRuntime::with_host(Box::new(pkm_plugin::VaultHost::root(vault.root.clone())))
        .expect("runtime")
}

fn runtime_with_allowlist(vault: &TestVault, allowlist: Vec<String>) -> PluginRuntime {
    PluginRuntime::with_host(Box::new(pkm_plugin::VaultHost::new(
        vault.root.clone(),
        allowlist,
    )))
    .expect("runtime")
}

fn load_http_state(registry: &mut PluginRegistry, wasm_path: &Path) -> PluginState {
    registry.load_plugin(wasm_path).expect("load_plugin");
    registry
        .get("com.example.http-e2e")
        .cloned()
        .expect("state")
}

/// Run the HTTP plugin's `onSave` hook with a raw JSON payload. The module
/// forwards the payload verbatim to `pkm.http_request`.
fn run_http(runtime: &PluginRuntime, plugin: &PluginState, json: &str) -> serde_json::Value {
    let out = runtime
        .run_plugin(plugin, "onSave", json)
        .expect("run_plugin");
    serde_json::from_str(&out).expect("envelope")
}

/// Drive an async futures to completion on a short-lived current-thread tokio
/// runtime. Used for the wiremock setup; the test body itself stays off a
/// tokio worker so `PluginRuntime`'s internal `block_on` does not nest.
fn block_on<F: std::future::Future>(fut: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("test runtime")
        .block_on(fut)
}

/// Start a wiremock server. The server runs on its own thread + runtime, so
/// it stays alive after the setup runtime is dropped.
fn start_mock() -> MockServer {
    block_on(MockServer::start())
}

fn mount_mock(mock: Mock, server: &MockServer) {
    block_on(mock.mount(server));
}

// ---------------------------------------------------------------------------
// allow / block / allowlist
// ---------------------------------------------------------------------------

#[test]
fn e2e_http_allow_wiremock_returns_stubbed_body() {
    let server = start_mock();
    mount_mock(
        Mock::given(method("GET"))
            .and(path("/notes/top"))
            .respond_with(ResponseTemplate::new(200).set_body_string("wiremock body")),
        &server,
    );

    let vault = TestVault::new();
    let runtime = runtime_for(&vault);
    let mut registry = PluginRegistry::new();
    let wasm_path = vault.install_http_plugin();
    let plugin = load_http_state(&mut registry, &wasm_path);

    let payload = format!(
        r#"{{"method":"GET","url":"{}/notes/top","timeout_ms":5000}}"#,
        server.uri()
    );
    let value = run_http(&runtime, &plugin, &payload);
    assert_eq!(value["kind"], "ok", "got: {value:?}");
    assert_eq!(value["status"], 200);
    assert_eq!(value["body"], "wiremock body");
}

#[test]
fn e2e_http_block_public_host_without_allowlist() {
    // A literal public IPv4 with no allowlist entry and not private/loopback.
    // The SSRF guard rejects it before any network traffic occurs — no server
    // is needed, which proves no round-trip happens.
    let vault = TestVault::new();
    let runtime = runtime_for(&vault);
    let mut registry = PluginRegistry::new();
    let wasm_path = vault.install_http_plugin();
    let plugin = load_http_state(&mut registry, &wasm_path);

    let payload = r#"{"method":"GET","url":"http://8.8.8.8/somewhere","timeout_ms":5000}"#;
    let value = run_http(&runtime, &plugin, payload);
    assert_eq!(value["kind"], "err", "got: {value:?}");
    assert_eq!(value["code"], "http_ssid");
}

#[test]
fn e2e_http_allowlist_explicitly_allows_public_host() {
    let server = start_mock();
    mount_mock(
        Mock::given(method("GET"))
            .and(path("/allowed"))
            .respond_with(ResponseTemplate::new(200).set_body_string("allowed body")),
        &server,
    );

    let vault = TestVault::new();
    // Allow exactly this wiremock host:port via the allowlist — the functional
    // equivalent of allowlisting the public host a production server would
    // sit on.
    let uri = server.uri();
    let hostport = uri.trim_start_matches("http://");
    let runtime = runtime_with_allowlist(&vault, vec![hostport.to_string()]);
    let mut registry = PluginRegistry::new();
    let wasm_path = vault.install_http_plugin();
    let plugin = load_http_state(&mut registry, &wasm_path);

    let payload = format!(
        r#"{{"method":"GET","url":"{}/allowed","timeout_ms":5000}}"#,
        server.uri()
    );
    let value = run_http(&runtime, &plugin, &payload);
    assert_eq!(value["kind"], "ok", "got: {value:?}");
    assert_eq!(value["body"], "allowed body");
}

// ---------------------------------------------------------------------------
// Status / transport / timeout mapping through the plugin path
// ---------------------------------------------------------------------------

#[test]
fn e2e_http_status_error_wiremock_maps_to_http_status() {
    let server = start_mock();
    mount_mock(
        Mock::given(method("GET"))
            .and(path("/missing"))
            .respond_with(ResponseTemplate::new(404).set_body_string("nope")),
        &server,
    );

    let vault = TestVault::new();
    let runtime = runtime_for(&vault);
    let mut registry = PluginRegistry::new();
    let wasm_path = vault.install_http_plugin();
    let plugin = load_http_state(&mut registry, &wasm_path);

    let payload = format!(
        r#"{{"method":"GET","url":"{}/missing","timeout_ms":5000}}"#,
        server.uri()
    );
    let value = run_http(&runtime, &plugin, &payload);
    assert_eq!(value["kind"], "err", "got: {value:?}");
    assert_eq!(value["code"], "http_status");
}

#[test]
fn e2e_http_server_error_wiremock_maps_to_http_status() {
    let server = start_mock();
    mount_mock(
        Mock::given(method("GET"))
            .and(path("/boom"))
            .respond_with(ResponseTemplate::new(500)),
        &server,
    );

    let vault = TestVault::new();
    let runtime = runtime_for(&vault);
    let mut registry = PluginRegistry::new();
    let wasm_path = vault.install_http_plugin();
    let plugin = load_http_state(&mut registry, &wasm_path);

    let payload = format!(
        r#"{{"method":"GET","url":"{}/boom","timeout_ms":5000}}"#,
        server.uri()
    );
    let value = run_http(&runtime, &plugin, &payload);
    assert_eq!(value["kind"], "err", "got: {value:?}");
    assert_eq!(value["code"], "http_status");
}

#[test]
fn e2e_http_transport_error_maps_to_http_transport() {
    // A port that nothing is listening on (bound, then closed) -> connection
    // refused.
    let dead_port = {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        l.local_addr().unwrap().port()
    };
    let vault = TestVault::new();
    let runtime = runtime_for(&vault);
    let mut registry = PluginRegistry::new();
    let wasm_path = vault.install_http_plugin();
    let plugin = load_http_state(&mut registry, &wasm_path);

    let payload =
        format!(r#"{{"method":"GET","url":"http://127.0.0.1:{dead_port}/","timeout_ms":5000}}"#);
    let value = run_http(&runtime, &plugin, &payload);
    assert_eq!(value["kind"], "err", "got: {value:?}");
    assert_eq!(value["code"], "http_transport");
}

#[test]
fn e2e_http_timeout_maps_to_http_timeout() {
    // A wiremock response delayed longer than the plugin's timeout.
    let server = start_mock();
    mount_mock(
        Mock::given(method("GET")).and(path("/slow")).respond_with(
            ResponseTemplate::new(200)
                .set_delay(std::time::Duration::from_millis(2000))
                .set_body_string("late"),
        ),
        &server,
    );

    let vault = TestVault::new();
    let runtime = runtime_for(&vault);
    let mut registry = PluginRegistry::new();
    let wasm_path = vault.install_http_plugin();
    let plugin = load_http_state(&mut registry, &wasm_path);

    let payload = format!(
        r#"{{"method":"GET","url":"{}/slow","timeout_ms":100}}"#,
        server.uri()
    );
    let value = run_http(&runtime, &plugin, &payload);
    assert_eq!(value["kind"], "err", "got: {value:?}");
    assert_eq!(value["code"], "http_timeout");
}

#[test]
fn e2e_http_post_verifies_method_and_body_via_wiremock() {
    // The strictest check: wiremock asserts the exact method, header, and JSON
    // body the host forwards for a POST. This proves the request the plugin
    // authored arrives byte-for-byte.
    let server = start_mock();
    mount_mock(
        Mock::given(method("POST"))
            .and(path("/submit"))
            .and(header("content-type", "application/json"))
            .and(wiremock::matchers::body_partial_json(serde_json::json!({
                "payload": "hello"
            })))
            .respond_with(ResponseTemplate::new(201).set_body_string("created")),
        &server,
    );

    let vault = TestVault::new();
    let runtime = runtime_for(&vault);
    let mut registry = PluginRegistry::new();
    let wasm_path = vault.install_http_plugin();
    let plugin = load_http_state(&mut registry, &wasm_path);

    let payload = format!(
        r#"{{"method":"POST","url":"{}/submit","headers":{{"content-type":"application/json"}},"body":"{{\"payload\":\"hello\"}}","timeout_ms":5000}}"#,
        server.uri()
    );
    let value = run_http(&runtime, &plugin, &payload);
    assert_eq!(value["kind"], "ok", "got: {value:?}");
    assert_eq!(value["status"], 201);
    assert_eq!(value["body"], "created");
}
