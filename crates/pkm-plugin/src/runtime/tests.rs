use super::*;
use crate::permissions::PermissionSet;
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
    PluginState::new(
        PluginManifest {
            name: name.to_string(),
            version: "0.2.0".to_string(),
            author: "test".to_string(),
            description: "test plugin".to_string(),
            permissions: PermissionSet::new(),
            entry_point: "plugin.wasm".to_string(),
            hooks: vec!["onSave".to_string(), "onOpen".to_string()],
        },
        wat.as_bytes().to_vec(), // wasmtime accepts WAT text as input
        enabled,
    )
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
fn test_run_plugin_with_hook() {
    let runtime = PluginRuntime::new().unwrap();
    let plugin = make_plugin_state("hook-test", HOOK_MODULE_WAT, true);

    let result = runtime.run_plugin(&plugin, "onSave", r#"{"file":"test.md"}"#);
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

    let context = RuntimeContext {
        current_plugin: None,
        output: String::new(),
    };
    let mut store = Store::new(runtime.engine(), context);
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

    let context = RuntimeContext {
        current_plugin: None,
        output: String::new(),
    };
    let mut store = Store::new(runtime.engine(), context);
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
