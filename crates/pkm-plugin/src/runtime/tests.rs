//! Split out of `runtime.rs` (module-size lint); mirrors the inline test module.

use super::*;
use crate::permissions::{Permission, PermissionSet};
use crate::registry::PluginManifest;

/// Minimal valid WASM module (empty module with no exports).
const EMPTY_MODULE: &[u8] = &[0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];

/// WAT source for a module that exports `onSave` and `onOpen` functions.
///
/// Each function reads payload from memory offset 0, echoes the input
/// length, and returns it.
const HOOK_MODULE_WAT: &str = r#"
    (module
        (memory (export "memory") 1)
        (func (export "onSave") (param i32 i32) (result i32)
            local.get 1
        )
        (func (export "onOpen") (param i32 i32) (result i32)
            local.get 1
        )
    )
"#;

/// WAT source for a minimal module that exports memory only (no functions).
const MEMORY_MODULE_WAT: &str = "(module (memory (export \"memory\") 1))";

fn make_plugin_state(name: &str, wat: &str, enabled: bool) -> PluginState {
    make_plugin_state_with_perms(name, wat, enabled, PermissionSet::new())
}

fn make_plugin_state_with_perms(
    name: &str,
    wat: &str,
    enabled: bool,
    perms: PermissionSet,
) -> PluginState {
    let mut hooks = std::collections::HashMap::new();
    hooks.insert("onSave".to_string(), true);
    hooks.insert("onOpen".to_string(), true);
    PluginState::new(
        PluginManifest {
            schema_version: 1,
            id: format!("com.example.{name}"),
            name: name.to_string(),
            version: "0.2.0".to_string(),
            author: "test".to_string(),
            description: "test plugin".to_string(),
            permissions: perms,
            entry: "plugin.wasm".to_string(),
            hooks,
        },
        wat.as_bytes().to_vec(), // wasmtime accepts WAT text as input
        enabled,
    )
}

/// Build a runtime wired to a `VaultHost` rooted at `root`.
fn runtime_with_root(root: &std::path::Path) -> PluginRuntime {
    PluginRuntime::with_host(Box::new(crate::host::VaultHost::root(root.to_path_buf())))
        .expect("runtime")
}

#[test]
fn test_runtime_creation() {
    let runtime = PluginRuntime::new();
    assert!(runtime.is_ok());
}

#[test]
fn test_compile_invalid_bytes() {
    let runtime = PluginRuntime::new().unwrap();
    let result = runtime.compile(b"not a valid wasm module");
    assert!(result.is_err());
    let err = result.unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("compile"),
        "Error should mention compile: {msg}"
    );
}

#[test]
fn test_compile_empty_module() {
    let runtime = PluginRuntime::new().unwrap();
    let result = runtime.compile(EMPTY_MODULE);
    assert!(result.is_ok());
}

#[test]
fn test_compile_invalid_too_short() {
    let runtime = PluginRuntime::new().unwrap();
    let result = runtime.compile(&[0x00]);
    assert!(result.is_err());
}

#[test]
fn test_compile_invalid_wrong_magic() {
    let runtime = PluginRuntime::new().unwrap();
    let result = runtime.compile(&[0xFF, 0xFF, 0xFF, 0xFF, 0x01, 0x00, 0x00, 0x00]);
    assert!(result.is_err());
}

#[test]
fn test_run_disabled_plugin() {
    let runtime = PluginRuntime::new().unwrap();
    let plugin = make_plugin_state("disabled-test", HOOK_MODULE_WAT, false);

    let result = runtime.run_plugin(&plugin, "onSave", "{}");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("disabled"));
}

#[test]
fn test_run_plugin_missing_hook() {
    let runtime = PluginRuntime::new().unwrap();
    let plugin = make_plugin_state("no-hook", "(module)", true);

    let result = runtime.run_plugin(&plugin, "onSave", "{}");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("does not export"));
}

#[test]
fn test_run_plugin_unknown_import_namespace_fails() {
    // Contract §7.1: a module importing from an unknown namespace/name
    // fails instantiation and does not load. wasmtime's Linker rejects
    // unregistered imports; this test locks that behavior so the
    // normative documentation matches the implementation.
    let runtime = PluginRuntime::new().unwrap();
    let plugin = make_plugin_state(
        "unknown-import",
        r#"
        (module
            (import "env" "some_foreign_func" (func))
            (memory (export "memory") 1)
            (func (export "onSave") (param i32 i32) (result i32)
                local.get 1
            )
        )
        "#,
        true,
    );

    let result = runtime.run_plugin(&plugin, "onSave", "{}");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("instantiate"),
        "expected an instantiation error, got: {err}"
    );
}

#[test]
fn test_run_plugin_with_hook() {
    let runtime = PluginRuntime::new().unwrap();
    let plugin = make_plugin_state("hook-test", HOOK_MODULE_WAT, true);

    let result = runtime.run_plugin(&plugin, "onSave", r#"{ "file": "test.md" }"#);
    match &result {
        Ok(output) => {
            assert!(output.len() <= 128);
        }
        Err(e) => {
            panic!("run_plugin failed: {e}");
        }
    }
}

#[test]
fn test_run_plugin_on_open_hook() {
    let runtime = PluginRuntime::new().unwrap();
    let plugin = make_plugin_state("hook-test", HOOK_MODULE_WAT, true);

    let result = runtime.run_plugin(&plugin, "onOpen", r#"{"note":"welcome"}"#);
    assert!(result.is_ok());
}

#[test]
fn test_normalize_hook_names() {
    assert_eq!(PluginRuntime::normalize_hook_name("onSave"), "onSave");
    assert_eq!(PluginRuntime::normalize_hook_name("on_save"), "onSave");
    assert_eq!(PluginRuntime::normalize_hook_name("onOpen"), "onOpen");
    assert_eq!(PluginRuntime::normalize_hook_name("on_open"), "onOpen");
    assert_eq!(PluginRuntime::normalize_hook_name("onLink"), "onLink");
    assert_eq!(PluginRuntime::normalize_hook_name("on_link"), "onLink");
    assert_eq!(PluginRuntime::normalize_hook_name("onSearch"), "onSearch");
    assert_eq!(PluginRuntime::normalize_hook_name("on_search"), "onSearch");
    assert_eq!(PluginRuntime::normalize_hook_name("custom"), "custom");
}

#[test]
fn test_plugin_event() {
    let event = PluginEvent::new("onSave", "{}", "test");
    assert_eq!(event.hook, "onSave");
    assert_eq!(event.payload, "{}");
    assert_eq!(event.source, "test");
}

#[test]
fn test_host_function_import_names() {
    assert_eq!(HostFunction::Log.import_name(), ("pkm", "log"));
    assert_eq!(HostFunction::NoteRead.import_name(), ("pkm", "note_read"));
    assert_eq!(HostFunction::NoteWrite.import_name(), ("pkm", "note_write"));
    assert_eq!(
        HostFunction::HttpRequest.import_name(),
        ("pkm", "http_request")
    );
}

#[test]
fn test_host_function_permissions() {
    assert_eq!(
        HostFunction::NoteRead.required_permission(),
        Permission::FileRead
    );
    assert_eq!(
        HostFunction::NoteWrite.required_permission(),
        Permission::FileWrite
    );
    assert_eq!(
        HostFunction::HttpRequest.required_permission(),
        Permission::Network
    );
    // Log is always allowed
    assert_eq!(HostFunction::Log.required_permission(), Permission::All);
}

#[test]
fn test_memory_read_bounds() {
    let runtime = PluginRuntime::new().unwrap();
    let module = Module::new(runtime.engine(), MEMORY_MODULE_WAT).unwrap();

    let context = RuntimeContext {
        current_plugin: None,
        output: String::new(),
        wasi: WasiCtxBuilder::new().inherit_stdio().build_p1(),
    };
    let mut temp_store = Store::new(runtime.engine(), context);
    let instance = runtime
        .linker
        .instantiate(&mut temp_store, &module)
        .unwrap();
    let memory = instance.get_memory(&mut temp_store, "memory").unwrap();
    let mem_size = memory.data_size(&temp_store);
    assert_eq!(mem_size, 65536); // 1 page

    // Reading valid range should succeed
    let result = memory_read(&memory, &temp_store, 0, 16);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 16);

    // Reading beyond bounds should fail
    let result = memory_read(&memory, &temp_store, 0, (mem_size + 1) as i32);
    assert!(result.is_err());
}

#[test]
fn test_memory_write_bounds() {
    let runtime = PluginRuntime::new().unwrap();
    let module = Module::new(runtime.engine(), MEMORY_MODULE_WAT).unwrap();

    let mut store = Store::new(
        runtime.engine(),
        RuntimeContext {
            current_plugin: None,
            output: String::new(),
            wasi: WasiCtxBuilder::new().inherit_stdio().build_p1(),
        },
    );
    let instance = runtime.linker.instantiate(&mut store, &module).unwrap();
    let memory = instance.get_memory(&mut store, "memory").unwrap();

    // Writing valid range should succeed
    let result = memory_write(&memory, &mut store, 0, &[1, 2, 3]);
    assert!(result.is_ok());

    // Verify the data was written
    let mut buf = [0u8; 3];
    memory.read(&store, 0, &mut buf).unwrap();
    assert_eq!(buf, [1, 2, 3]);
}

#[test]
fn test_memory_write_oob() {
    let runtime = PluginRuntime::new().unwrap();
    let module = Module::new(runtime.engine(), MEMORY_MODULE_WAT).unwrap();

    let mut store = Store::new(
        runtime.engine(),
        RuntimeContext {
            current_plugin: None,
            output: String::new(),
            wasi: WasiCtxBuilder::new().inherit_stdio().build_p1(),
        },
    );
    let instance = runtime.linker.instantiate(&mut store, &module).unwrap();
    let memory = instance.get_memory(&mut store, "memory").unwrap();
    let mem_size = memory.data_size(&store);

    // Writing just past bounds should fail
    let result = memory_write(
        &memory,
        &mut store,
        (mem_size - 1) as i32,
        &[0u8, 0u8, 0u8], // 3 bytes starting at mem_size-1 goes past
    );
    assert!(result.is_err());
}

// -----------------------------------------------------------------------
// End-to-end host dispatch tests.
//
// These instantiate a real WASM module importing `pkm.note_read`,
// `pkm.note_write` and `pkm.http_request` through `runtime.linker` (the
// real dispatch path: permission gating, host backend, envelope
// encoding), write the request JSON to memory, call the imported hook,
// and read the envelope the host wrote at memory offset 0. No stubs.
// -----------------------------------------------------------------------

/// WAT for a plugin that exports `onSave`, forwarding its `(ptr, len)`
/// payload to the given `pkm.note_read` import and returning the host
/// response length.
const NOTE_READ_WAT: &str = r#"
    (module
        (import "pkm" "note_read" (func $note_read (param i32 i32) (result i32)))
        (memory (export "memory") 1)
        (func (export "onSave") (param $ptr i32) (param $len i32) (result i32)
            local.get $ptr
            local.get $len
            call $note_read
        )
    )
"#;

/// WAT for a plugin whose `onSave` forwards its payload to `pkm.note_write`.
const NOTE_WRITE_WAT: &str = r#"
    (module
        (import "pkm" "note_write" (func $note_write (param i32 i32) (result i32)))
        (memory (export "memory") 1)
        (func (export "onSave") (param $ptr i32) (param $len i32) (result i32)
            local.get $ptr
            local.get $len
            call $note_write
        )
    )
"#;

/// WAT for a plugin whose `onSave` forwards its payload to `pkm.http_request`.
const HTTP_REQ_WAT: &str = r#"
    (module
        (import "pkm" "http_request" (func $http (param i32 i32) (result i32)))
        (memory (export "memory") 1)
        (func (export "onSave") (param $ptr i32) (param $len i32) (result i32)
            local.get $ptr
            local.get $len
            call $http
        )
    )
"#;

/// Instantiate `wat` through `runtime.linker`, write `request_json` to
/// memory offset 0, invoke the exported `onSave`, and read the response
/// envelope the host wrote at offset 0. Returns the raw envelope bytes.
/// The store carries no executing plugin (host-test path → no permission
/// gate).
fn invoke_host(runtime: &PluginRuntime, wat: &str, request_json: &str) -> Vec<u8> {
    invoke_host_with_plugin(runtime, wat, request_json, None)
}

/// Like [`invoke_host`], but with an executing `plugin` in the store
/// context so the permission gate applies to the host call.
fn invoke_host_with_plugin(
    runtime: &PluginRuntime,
    wat: &str,
    request_json: &str,
    plugin: Option<PluginState>,
) -> Vec<u8> {
    let module = Module::new(runtime.engine(), wat).unwrap();
    let context = RuntimeContext {
        current_plugin: plugin,
        output: String::new(),
        wasi: WasiCtxBuilder::new().inherit_stdio().build_p1(),
    };
    let mut store = Store::new(runtime.engine(), context);
    let instance = runtime.linker.instantiate(&mut store, &module).unwrap();
    let memory = instance.get_memory(&mut store, "memory").unwrap();

    let req_bytes = request_json.as_bytes();
    memory.write(&mut store, 0, req_bytes).unwrap();

    let f: TypedFunc<(i32, i32), i32> = instance
        .get_typed_func(&mut store, "onSave")
        .map_err(|e| e.to_string())
        .expect("onSave export");
    let len = f.call(&mut store, (0, req_bytes.len() as i32)).unwrap();

    if len <= 0 {
        return Vec::new();
    }
    let mut buf = vec![0u8; len as usize];
    memory.read(&store, 0, &mut buf).unwrap();
    buf
}

/// Build a plugin state with the given permissions.
fn make_plugin_state_with(_root: &std::path::Path, wat: &str, enabled: bool) -> PluginState {
    let mut hooks = std::collections::HashMap::new();
    hooks.insert("onSave".to_string(), true);
    PluginState::new(
        PluginManifest {
            schema_version: 1,
            id: "com.example.perm-test".to_string(),
            name: "perm-test".to_string(),
            version: "0.2.0".to_string(),
            author: "test".to_string(),
            description: "permission test plugin".to_string(),
            permissions: PermissionSet::new(), // no grants
            entry: "plugin.wasm".to_string(),
            hooks,
        },
        wat.as_bytes().to_vec(),
        enabled,
    )
}

#[test]
fn dispatch_note_read_returns_real_content() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("notes")).unwrap();
    std::fs::write(dir.path().join("notes/real.md"), "# Real\nbody").unwrap();

    let runtime = runtime_with_root(dir.path());
    let bytes = invoke_host(&runtime, NOTE_READ_WAT, r#"{"path":"notes/real.md"}"#);

    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["kind"], "ok");
    assert_eq!(value["data"]["path"], "notes/real.md");
    assert_eq!(value["data"]["content"], "# Real\nbody");
    assert!(value["data"]["mtime"].is_string());
}

#[test]
fn dispatch_note_read_missing_yields_error_envelope() {
    let dir = tempfile::tempdir().unwrap();
    let runtime = runtime_with_root(dir.path());
    let bytes = invoke_host(&runtime, NOTE_READ_WAT, r#"{"path":"notes/absent.md"}"#);

    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["kind"], "err");
    assert_eq!(value["code"], "note_not_found");
}

#[test]
fn dispatch_note_read_empty_path_is_invalid_argument() {
    let dir = tempfile::tempdir().unwrap();
    let runtime = runtime_with_root(dir.path());
    let bytes = invoke_host(&runtime, NOTE_READ_WAT, r#"{"path":""}"#);

    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["kind"], "err");
    assert_eq!(value["code"], "invalid_argument");
}

#[test]
fn dispatch_note_write_creates_file() {
    let dir = tempfile::tempdir().unwrap();
    let runtime = runtime_with_root(dir.path());
    let bytes = invoke_host(
        &runtime,
        NOTE_WRITE_WAT,
        r#"{"path":"notes/written.md","content":"hello from plugin"}"#,
    );

    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["kind"], "ok");
    assert_eq!(value["data"]["path"], "notes/written.md");
    assert_eq!(value["data"]["written"], true);

    // The file must actually exist on disk with the real content.
    let on_disk = std::fs::read_to_string(dir.path().join("notes/written.md")).unwrap();
    assert_eq!(on_disk, "hello from plugin");
}

#[test]
fn dispatch_note_write_failure_propagates() {
    let dir = tempfile::tempdir().unwrap();
    let runtime = runtime_with_root(dir.path());
    // Traversal escapes the vault → write must error, never fake-success.
    let bytes = invoke_host(
        &runtime,
        NOTE_WRITE_WAT,
        r#"{"path":"../../outside.md","content":"x"}"#,
    );

    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["kind"], "err");
    assert_eq!(value["code"], "note_write_failed");
}

#[test]
fn dispatch_http_request_success() {
    let port = crate::host::tests::serve("200 OK", "body text");
    let dir = tempfile::tempdir().unwrap();
    let runtime = runtime_with_root(dir.path());
    let payload =
        format!(r#"{{"method":"GET","url":"http://127.0.0.1:{port}/x","timeout_ms":5000}}"#);
    let bytes = invoke_host(&runtime, HTTP_REQ_WAT, &payload);

    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    // Loopback is allowed by default → the request goes through.
    assert_eq!(value["kind"], "ok", "got: {value}");
    assert_eq!(value["status"], 200);
    assert_eq!(value["body"], "body text");
}

#[test]
fn dispatch_http_request_bad_method_is_invalid_argument() {
    let dir = tempfile::tempdir().unwrap();
    let runtime = runtime_with_root(dir.path());
    let bytes = invoke_host(
        &runtime,
        HTTP_REQ_WAT,
        r#"{"method":"TRACE","url":"http://127.0.0.1:1/"}"#,
    );

    // With no executing plugin (host-test path) the permission gate is
    // skipped, so method validation applies: TRACE is not in the allowed
    // set → invalid_argument.
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["kind"], "err");
    assert_eq!(value["code"], "invalid_argument");
}

#[test]
fn dispatch_http_request_denied_without_network_permission() {
    let dir = tempfile::tempdir().unwrap();
    let runtime = runtime_with_root(dir.path());
    // A plugin with NO permissions granted: the network host function is
    // denied before any network work happens.
    let plugin = make_plugin_state_with(dir.path(), HTTP_REQ_WAT, true);
    let bytes = invoke_host_with_plugin(
        &runtime,
        HTTP_REQ_WAT,
        r#"{"method":"GET","url":"http://127.0.0.1:1/"}"#,
        Some(plugin),
    );

    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["kind"], "err");
    assert_eq!(value["code"], "plugin_denied");
}

#[test]
fn dispatch_note_read_denied_without_file_read_permission() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("notes")).unwrap();
    std::fs::write(dir.path().join("notes/real.md"), "x").unwrap();
    let runtime = runtime_with_root(dir.path());
    let plugin = make_plugin_state_with(dir.path(), NOTE_READ_WAT, true);
    let bytes = invoke_host_with_plugin(
        &runtime,
        NOTE_READ_WAT,
        r#"{"path":"notes/real.md"}"#,
        Some(plugin),
    );

    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["kind"], "err");
    assert_eq!(value["code"], "plugin_denied");
}

#[test]
fn dispatch_http_request_ssrf_blocked() {
    // A public host with no allowlist and not private → http_ssid, and no
    // network round-trip is required.
    let dir = tempfile::tempdir().unwrap();
    let runtime = runtime_with_root(dir.path());
    let bytes = invoke_host(
        &runtime,
        HTTP_REQ_WAT,
        r#"{"method":"GET","url":"http://93.184.216.34/x","timeout_ms":5000}"#,
    );

    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["kind"], "err");
    assert_eq!(value["code"], "http_ssid");
}
