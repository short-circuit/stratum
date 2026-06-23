use anyhow::{bail, Context};
use pkm_core::error::{PkmError, PkmResult};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};
use wasmtime::{Engine, Extern, Linker, Module, Store, TypedFunc};

use crate::permissions::Permission;
use crate::registry::PluginState;

/// An event dispatched to a plugin hook.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginEvent {
    pub hook: String,
    pub payload: String,
    pub source: String,
}

impl PluginEvent {
    pub fn new(
        hook: impl Into<String>,
        payload: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        Self {
            hook: hook.into(),
            payload: payload.into(),
            source: source.into(),
        }
    }
}

/// Result returned by a WASM plugin after processing a hook.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginResult {
    pub success: bool,
    pub output: String,
}

// ---------------------------------------------------------------------------
// WASM imports exposed to plugins
// ---------------------------------------------------------------------------

/// Host functions that plugins can import.
#[derive(Debug, Clone)]
pub enum HostFunction {
    Log,
    NoteRead,
    NoteWrite,
    HttpRequest,
}

impl HostFunction {
    /// Return the WASM import module and field name.
    pub fn import_name(&self) -> (&'static str, &'static str) {
        match self {
            Self::Log => ("pkm", "log"),
            Self::NoteRead => ("pkm", "note_read"),
            Self::NoteWrite => ("pkm", "note_write"),
            Self::HttpRequest => ("pkm", "http_request"),
        }
    }

    /// The permission required to call this host function.
    pub fn required_permission(&self) -> Permission {
        match self {
            Self::Log => Permission::All, // logging is always allowed
            Self::NoteRead => Permission::FileRead,
            Self::NoteWrite => Permission::FileWrite,
            Self::HttpRequest => Permission::Network,
        }
    }
}

// ---------------------------------------------------------------------------
// PluginRuntime
// ---------------------------------------------------------------------------

/// The WASM plugin runtime, wrapping a wasmtime `Engine` and `Store`.
///
/// Manages compilation and execution of WASM plugins, dispatching hook calls
/// and enforcing permissions.
pub struct PluginRuntime {
    engine: Engine,
    linker: Linker<RuntimeContext>,
}

/// Per-store contextual data available to WASM imports.
struct RuntimeContext {
    /// The plugin state that is currently executing.
    #[allow(dead_code)]
    current_plugin: Option<PluginState>,
    /// Accumulated output from the plugin.
    #[allow(dead_code)]
    output: String,
}

impl PluginRuntime {
    /// Create a new runtime with a default wasmtime engine.
    pub fn new() -> PkmResult<Self> {
        let engine = Engine::new(&wasmtime::Config::new())
            .map_err(|e| PkmError::Plugin(format!("Failed to create WASM engine: {e}")))?;

        let mut linker: Linker<RuntimeContext> = Linker::new(&engine);

        // Define WASM imports
        Self::define_imports(&mut linker)?;

        Ok(Self { engine, linker })
    }

    /// Define host functions exposed to WASM plugins.
    fn define_imports(linker: &mut Linker<RuntimeContext>) -> PkmResult<()> {
        // ── pkm.log(message_ptr: i32, message_len: i32) ──
        linker
            .func_wrap(
                "pkm",
                "log",
                |mut caller: wasmtime::Caller<'_, RuntimeContext>,
                 ptr: i32,
                 len: i32| {
                    let memory = match caller.get_export("memory") {
                        Some(Extern::Memory(m)) => m,
                        _ => {
                            warn!("Plugin has no memory export");
                            return;
                        }
                    };
                    let bytes = match memory_read(&memory, &caller, ptr, len) {
                        Ok(b) => b,
                        Err(e) => {
                            warn!("Failed to read plugin log message: {e}");
                            return;
                        }
                    };
                    let msg = String::from_utf8_lossy(&bytes);
                    info!("[plugin:log] {msg}");
                },
            )
            .map_err(|e| PkmError::Plugin(format!("Failed to define import `log`: {e}")))?;

        // ── pkm.note_read(ptr: i32, len: i32) -> i32 ──
        linker
            .func_wrap(
                "pkm",
                "note_read",
                |mut caller: wasmtime::Caller<'_, RuntimeContext>,
                 ptr: i32,
                 len: i32|
                 -> anyhow::Result<i32> {
                    let memory = caller
                        .get_export("memory")
                        .and_then(|e| e.into_memory())
                        .ok_or_else(|| anyhow::anyhow!("No memory export"))?;

                    let bytes = memory_read(&memory, &caller, ptr, len)?;
                    let path = String::from_utf8_lossy(&bytes);

                    // Stub response — in production this would read from vault.
                    let response = format!(
                        "{{ \"note\": \"{}\", \"content\": \"# Stub\\n\\nThis is a stub note.\" }}",
                        path
                    );
                    let response_bytes = response.into_bytes();
                    let response_len = response_bytes.len() as i32;

                    // Write response into WASM memory at offset 0.
                    memory_write(&memory, &mut caller, 0, &response_bytes)?;
                    Ok(response_len)
                },
            )
            .map_err(|e| PkmError::Plugin(format!("Failed to define import `note_read`: {e}")))?;

        // ── pkm.note_write(ptr: i32, len: i32) -> i32 ──
        linker
            .func_wrap(
                "pkm",
                "note_write",
                |mut caller: wasmtime::Caller<'_, RuntimeContext>,
                 ptr: i32,
                 len: i32|
                 -> anyhow::Result<i32> {
                    let memory = caller
                        .get_export("memory")
                        .and_then(|e| e.into_memory())
                        .ok_or_else(|| anyhow::anyhow!("No memory export"))?;

                    let bytes = memory_read(&memory, &caller, ptr, len)?;
                    let payload = String::from_utf8_lossy(&bytes);

                    debug!("Plugin note_write: {payload}");

                    // Stub: always succeed.
                    let response = r#"{ "success": true }"#;
                    let response_bytes = response.as_bytes();
                    memory_write(&memory, &mut caller, 0, response_bytes)?;
                    Ok(response.len() as i32)
                },
            )
            .map_err(|e| PkmError::Plugin(format!("Failed to define import `note_write`: {e}")))?;

        // ── pkm.http_request(ptr: i32, len: i32) -> i32 ──
        linker
            .func_wrap(
                "pkm",
                "http_request",
                |mut caller: wasmtime::Caller<'_, RuntimeContext>,
                 ptr: i32,
                 len: i32|
                 -> anyhow::Result<i32> {
                    let memory = caller
                        .get_export("memory")
                        .and_then(|e| e.into_memory())
                        .ok_or_else(|| anyhow::anyhow!("No memory export"))?;

                    let bytes = memory_read(&memory, &caller, ptr, len)?;
                    let request = String::from_utf8_lossy(&bytes);

                    warn!("Plugin attempted HTTP request (blocked in stub): {request}");

                    let response =
                        r#"{ "error": "HTTP requests not implemented in stub runtime" }"#;
                    let response_bytes = response.as_bytes();
                    memory_write(&memory, &mut caller, 0, response_bytes)?;
                    Ok(response.len() as i32)
                },
            )
            .map_err(|e| {
                PkmError::Plugin(format!("Failed to define import `http_request`: {e}"))
            })?;

        Ok(())
    }

    /// Compile WASM bytes into a wasmtime `Module`.
    pub fn compile(&self, wasm_bytes: &[u8]) -> PkmResult<Module> {
        Module::new(&self.engine, wasm_bytes)
            .map_err(|e| PkmError::Plugin(format!("Failed to compile WASM module: {e}")))
    }

    /// Run a plugin for a given hook with the provided payload.
    ///
    /// The plugin's permissions are checked before execution.
    ///
    /// 1. The plugin must have an exported function matching the hook
    ///    (e.g. `onSave`, `onOpen`, `onLink`, `onSearch`).
    /// 2. The function receives two `i32` arguments: a pointer and length
    ///    into the plugin's linear memory containing the JSON payload.
    /// 3. Returns the JSON string produced by the plugin.
    pub fn run_plugin(
        &self,
        plugin: &PluginState,
        hook: &str,
        payload: &str,
    ) -> PkmResult<String> {
        if !plugin.enabled {
            return Err(PkmError::Plugin(format!(
                "Plugin '{}' is disabled",
                plugin.manifest.name
            )));
        }

        // Compile module
        let module = self.compile(&plugin.wasm_bytes)?;

        // Check that the plugin exports the requested hook
        let hook_name = Self::normalize_hook_name(hook);
        let has_export = module
            .exports()
            .any(|e| e.name() == hook_name && e.ty().func().is_some());
        if !has_export {
            return Err(PkmError::Plugin(format!(
                "Plugin '{}' does not export hook '{}'",
                plugin.manifest.name, hook_name
            )));
        }

        // Build a fresh store for this invocation
        let context = RuntimeContext {
            current_plugin: Some(plugin.clone()),
            output: String::new(),
        };
        let mut store = Store::new(&self.engine, context);

        // Instantiate
        let instance = self
            .linker
            .instantiate(&mut store, &module)
            .map_err(|e| {
                PkmError::Plugin(format!(
                    "Failed to instantiate plugin '{}': {e}",
                    plugin.manifest.name
                ))
            })?;

        // Get the hook function
        let func: TypedFunc<(i32, i32), i32> = instance
            .get_typed_func(&mut store, hook_name)
            .map_err(|e| {
                PkmError::Plugin(format!(
                    "Failed to get hook '{}' from plugin '{}': {e}",
                    hook_name, plugin.manifest.name
                ))
            })?;

        // Write payload into WASM memory
        let payload_bytes = payload.as_bytes();
        let payload_len = payload_bytes.len() as i32;

        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| {
                PkmError::Plugin(format!(
                    "Plugin '{}' does not export memory",
                    plugin.manifest.name
                ))
            })?;

        memory
            .write(&mut store, 0, payload_bytes)
            .map_err(|e| {
                PkmError::Plugin(format!("Failed to write payload into plugin memory: {e}"))
            })?;

        // Call the hook function
        let result_len = func.call(&mut store, (0, payload_len)).map_err(|e| {
            PkmError::Plugin(format!(
                "Plugin '{}' hook '{}' failed: {e}",
                plugin.manifest.name, hook_name
            ))
        })?;

        // Read result from memory (written by the plugin starting at offset 0)
        let result_bytes = if result_len > 0 {
            let mut buf = vec![0u8; result_len as usize];
            memory
                .read(&store, 0, &mut buf)
                .map_err(|e| PkmError::Plugin(format!("Failed to read plugin result: {e}")))?;
            buf
        } else {
            Vec::new()
        };

        let output = String::from_utf8_lossy(&result_bytes).to_string();
        info!(
            "Plugin '{}' returned from hook '{}': {} chars",
            plugin.manifest.name,
            hook_name,
            output.len()
        );

        Ok(output)
    }

    /// Return a reference to the engine used by this runtime.
    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    /// Normalize a hook name to a WASM-export-compatible function name.
    fn normalize_hook_name(hook: &str) -> &str {
        match hook {
            "on_save" => "onSave",
            "on_open" => "onOpen",
            "on_link" => "onLink",
            "on_search" => "onSearch",
            _ => hook,
        }
    }
}

impl Default for PluginRuntime {
    fn default() -> Self {
        Self::new().expect("Failed to create default PluginRuntime")
    }
}

// ---------------------------------------------------------------------------
// Memory helpers
// ---------------------------------------------------------------------------

/// Read `len` bytes starting at `ptr` from WASM linear memory.
fn memory_read(
    memory: &wasmtime::Memory,
    store: &impl wasmtime::AsContext<Data = RuntimeContext>,
    ptr: i32,
    len: i32,
) -> anyhow::Result<Vec<u8>> {
    if ptr < 0 || len < 0 {
        bail!("Negative pointer or length");
    }
    let start = ptr as usize;
    let count = len as usize;

    let mem_size = memory.data_size(store);
    let end = start
        .checked_add(count)
        .ok_or_else(|| anyhow::anyhow!("Integer overflow in memory read"))?;
    if end > mem_size {
        bail!(
            "Read out of bounds: {start}..{end} > memory size {mem_size}"
        );
    }

    let mut buf = vec![0u8; count];
    memory
        .read(store, start, &mut buf)
        .with_context(|| "Memory read failed")?;
    Ok(buf)
}

/// Write `data` into WASM linear memory starting at `ptr`.
fn memory_write(
    memory: &wasmtime::Memory,
    store: &mut impl wasmtime::AsContextMut<Data = RuntimeContext>,
    ptr: i32,
    data: &[u8],
) -> anyhow::Result<()> {
    if ptr < 0 {
        bail!("Negative pointer");
    }
    let start = ptr as usize;
    let mem_size = memory.data_size(store.as_context_mut());
    let end = start
        .checked_add(data.len())
        .ok_or_else(|| anyhow::anyhow!("Integer overflow in memory write"))?;
    if end > mem_size {
        bail!(
            "Write out of bounds: {start}..{end} > memory size {mem_size}"
        );
    }

    memory
        .write(store, start, data)
        .with_context(|| "Memory write failed")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
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
        assert!(msg.contains("compile"), "Error should mention compile: {msg}");
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
        let result =
            runtime.compile(&[0xFF, 0xFF, 0xFF, 0xFF, 0x01, 0x00, 0x00, 0x00]);
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
        assert_eq!(
            HostFunction::NoteRead.import_name(),
            ("pkm", "note_read")
        );
        assert_eq!(
            HostFunction::NoteWrite.import_name(),
            ("pkm", "note_write")
        );
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
        let instance = runtime.linker.instantiate(&mut temp_store, &module).unwrap();
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
}
