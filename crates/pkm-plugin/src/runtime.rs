use std::sync::Arc;

use anyhow::{bail, Context};
use pkm_core::error::{PkmError, PkmResult};
use serde::Serialize;
use tracing::{debug, info, warn};
use wasmtime::{Engine, Linker, Module, Store, TypedFunc};
use wasmtime_wasi::preview1::{self, WasiP1Ctx};
use wasmtime_wasi::WasiCtxBuilder;

use crate::abi::{
    HostApi, HostFunction, HostResponse, HttpResponseEnvelope, PluginErrorCode, HOST_PAYLOAD_MAX,
};
use crate::registry::PluginState;

/// An event dispatched to a plugin hook.
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
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
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct PluginResult {
    pub success: bool,
    pub output: String,
}

// ---------------------------------------------------------------------------
// PluginRuntime
// ---------------------------------------------------------------------------

/// The WASM plugin runtime, wrapping a wasmtime `Engine`, `Store`, and the
/// real host backend.
///
/// Host functions are dispatched through an injected [`HostApi`] implementation
/// (see [`PluginRuntime::with_host`]). Default construction uses a
/// [`crate::abi::NoopHost`]; the Tauri/CLI layer injects a vault-backed host
/// via [`crate::host::VaultHost`].
pub struct PluginRuntime {
    engine: Engine,
    linker: Linker<RuntimeContext>,
    host: Arc<dyn HostApi>,
}

/// Per-store contextual data available to WASM imports.
struct RuntimeContext {
    /// The plugin state that is currently executing.
    #[allow(dead_code)]
    current_plugin: Option<PluginState>,
    /// Accumulated output from the plugin.
    #[allow(dead_code)]
    output: String,
    /// Real WASI preview1 context (stdio, env, clocks, random, proc_exit).
    ///
    /// This is what lets genuine `wasm32-wasip1` Rust `std` builds instantiate
    /// with their real import signatures (fd_write, environ_get, proc_exit,
    /// random_get, …). No filesystem is preopened by default — plugins access
    /// the vault exclusively through the `pkm.*` host API (contract §4), which
    /// keeps the sandbox exact.
    wasi: WasiP1Ctx,
}

impl PluginRuntime {
    /// Create a new runtime with a default wasmtime engine and a no-op host.
    pub fn new() -> PkmResult<Self> {
        Self::with_host(Box::new(crate::abi::NoopHost))
    }

    /// Create a runtime that dispatches host calls through `host`.
    ///
    /// The host is `Send + Sync` as required by [`HostApi`]. The runtime owns a
    /// shared `tokio` runtime used to drive async host operations from the
    /// synchronous WASM import callbacks.
    pub fn with_host(host: Box<dyn HostApi>) -> PkmResult<Self> {
        let engine = Engine::new(&wasmtime::Config::new())
            .map_err(|e| PkmError::Plugin(format!("Failed to create WASM engine: {e}")))?;

        let rt = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .map_err(|e| {
                    PkmError::Plugin(format!("Failed to create plugin async runtime: {e}"))
                })?,
        );

        let host: Arc<dyn HostApi> = Arc::from(host);

        let mut linker: Linker<RuntimeContext> = Linker::new(&engine);

        // Define WASM imports. The closures capture `host`/`rt` clones, keeping
        // the async runtime alive for the lifetime of the linker.
        Self::define_imports(&mut linker, host.clone(), rt.clone())?;

        Ok(Self {
            engine,
            linker,
            host,
        })
    }

    /// Access the injected host backend.
    pub fn host(&self) -> &dyn HostApi {
        &*self.host
    }

    /// Define host functions exposed to WASM plugins.
    ///
    /// The closures capture shared references to the host backend and the
    /// async runtime so that each host call is dispatched to the injected
    /// backend. The `log` function requires no permission and always succeeds.
    fn define_imports(
        linker: &mut Linker<RuntimeContext>,
        host: Arc<dyn HostApi>,
        rt: Arc<tokio::runtime::Runtime>,
    ) -> PkmResult<()> {
        // ── pkm.log(ptr: i32, len: i32) -> i32 ──
        {
            let host = host.clone();
            let rt = rt.clone();
            linker
                .func_wrap(
                    "pkm",
                    "log",
                    move |mut caller: wasmtime::Caller<'_, RuntimeContext>,
                          ptr: i32,
                          len: i32|
                          -> anyhow::Result<i32> {
                        // Route through the same envelope-producing path as the
                        // other host functions so the caller can read back the
                        // contract §4.1 response
                        // `{"kind":"ok","data":{"logged":true}}`. The log
                        // function needs no permission (always allowed).
                        let (memory, response) =
                            dispatch_host(&mut caller, ptr, len, HostFunction::Log, &host, &rt)?;
                        write_response(&memory, &mut caller, &response)?;
                        Ok(response.len() as i32)
                    },
                )
                .map_err(|e| PkmError::Plugin(format!("Failed to define import `log`: {e}")))?;
        }

        // ── pkm.note_read(ptr: i32, len: i32) -> i32 ──
        {
            let host = host.clone();
            let rt = rt.clone();
            linker
                .func_wrap(
                    "pkm",
                    "note_read",
                    move |mut caller: wasmtime::Caller<'_, RuntimeContext>,
                          ptr: i32,
                          len: i32|
                          -> anyhow::Result<i32> {
                        let (memory, response) = dispatch_host(
                            &mut caller,
                            ptr,
                            len,
                            HostFunction::NoteRead,
                            &host,
                            &rt,
                        )?;
                        write_response(&memory, &mut caller, &response)?;
                        Ok(response.len() as i32)
                    },
                )
                .map_err(|e| {
                    PkmError::Plugin(format!("Failed to define import `note_read`: {e}"))
                })?;
        }

        // ── pkm.note_write(ptr: i32, len: i32) -> i32 ──
        {
            let host = host.clone();
            let rt = rt.clone();
            linker
                .func_wrap(
                    "pkm",
                    "note_write",
                    move |mut caller: wasmtime::Caller<'_, RuntimeContext>,
                          ptr: i32,
                          len: i32|
                          -> anyhow::Result<i32> {
                        let (memory, response) = dispatch_host(
                            &mut caller,
                            ptr,
                            len,
                            HostFunction::NoteWrite,
                            &host,
                            &rt,
                        )?;
                        write_response(&memory, &mut caller, &response)?;
                        Ok(response.len() as i32)
                    },
                )
                .map_err(|e| {
                    PkmError::Plugin(format!("Failed to define import `note_write`: {e}"))
                })?;
        }

        // ── pkm.http_request(ptr: i32, len: i32) -> i32 ──
        {
            let host = host.clone();
            let rt = rt.clone();
            linker
                .func_wrap(
                    "pkm",
                    "http_request",
                    move |mut caller: wasmtime::Caller<'_, RuntimeContext>,
                          ptr: i32,
                          len: i32|
                          -> anyhow::Result<i32> {
                        let (memory, response) = dispatch_host(
                            &mut caller,
                            ptr,
                            len,
                            HostFunction::HttpRequest,
                            &host,
                            &rt,
                        )?;
                        write_response(&memory, &mut caller, &response)?;
                        Ok(response.len() as i32)
                    },
                )
                .map_err(|e| {
                    PkmError::Plugin(format!("Failed to define import `http_request`: {e}"))
                })?;
        }

        // Real WASI preview1 imports (fd_write, environ_get, proc_exit,
        // random_get, clocks, …) so genuine wasm32-wasip1 builds — including
        // Rust `std` cdylibs that link the WASI CRT — instantiate with their
        // real import signatures (contract §7.1).
        preview1::add_to_linker_sync(linker, |cx| &mut cx.wasi).map_err(|e| {
            PkmError::Plugin(format!("Failed to define WASI preview1 imports: {e}"))
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
    pub fn run_plugin(&self, plugin: &PluginState, hook: &str, payload: &str) -> PkmResult<String> {
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
            wasi: WasiCtxBuilder::new().inherit_stdio().build_p1(),
        };
        let mut store = Store::new(&self.engine, context);

        // Instantiate
        let instance = self.linker.instantiate(&mut store, &module).map_err(|e| {
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

        let memory = instance.get_memory(&mut store, "memory").ok_or_else(|| {
            PkmError::Plugin(format!(
                "Plugin '{}' does not export memory",
                plugin.manifest.name
            ))
        })?;

        memory.write(&mut store, 0, payload_bytes).map_err(|e| {
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
            "[stratum] Plugin '{}' returned from hook '{}': {} chars",
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
// Host dispatch
// ---------------------------------------------------------------------------

/// The outcome of a host call prior to envelope encoding.
enum HostOutcome {
    Ok(String),
    Err {
        code: PluginErrorCode,
        message: String,
    },
}

/// Generic host-call dispatcher.
///
/// 1. Reads the request bytes from WASM memory (bounded at `HOST_PAYLOAD_MAX`).
/// 2. Validates the request envelope.
/// 3. Checks the plugin's permission for the function.
/// 4. Calls the real backend and encodes the result as an envelope.
///
/// Errors that occur *outside* the envelope protocol (e.g. no memory export,
/// unreadable memory) are returned as `anyhow` errors that trap the host call;
/// per contract §3.4, a plugin that followed the ABI never sees a trap because
/// all protocol-level failures are returned in the envelope.
fn dispatch_host(
    caller: &mut wasmtime::Caller<'_, RuntimeContext>,
    ptr: i32,
    len: i32,
    func: HostFunction,
    host: &Arc<dyn HostApi>,
    rt: &Arc<tokio::runtime::Runtime>,
) -> anyhow::Result<(wasmtime::Memory, Vec<u8>)> {
    let memory = caller
        .get_export("memory")
        .and_then(|e| e.into_memory())
        .ok_or_else(|| anyhow::anyhow!("No memory export"))?;

    let bytes = memory_read(&memory, caller, ptr, len)?;

    // Enforce the 1 MiB outbound-payload bound (contract §3.3): a request
    // larger than the bound yields `InvalidArgument`.
    if bytes.len() > HOST_PAYLOAD_MAX {
        let envelope = encode_err(
            PluginErrorCode::InvalidArgument,
            "[stratum] request payload exceeds 1 MiB",
        );
        return Ok((memory, envelope));
    }

    let plugin = caller.data().current_plugin.clone();
    let plugin_id = plugin
        .as_ref()
        .map(|p| p.manifest.name.clone())
        .unwrap_or_else(|| "<host>".to_string());

    let outcome = dispatch(&func, &plugin, &bytes, host, rt, &plugin_id);

    let envelope = match outcome {
        HostOutcome::Ok(data) => data.into_bytes(),
        HostOutcome::Err { code, message } => {
            // Every host call failure is logged at `warn` with plugin id,
            // function, and error code (contract §6.4).
            warn!(
                "[stratum] plugin {plugin_id} host call {:?} failed: {:?} — {message}",
                func, code
            );
            encode_err(code, &message)
        }
    };

    Ok((memory, envelope))
}

fn encode_err(code: PluginErrorCode, message: &str) -> Vec<u8> {
    let resp = HostResponse::Err {
        code,
        message: truncate_msg(message, crate::abi::ERROR_MESSAGE_MAX),
    };
    serde_json::to_vec(&resp).unwrap_or_else(|_| {
        br#"{"kind":"err","code":"internal","message":"[stratum] serialization failure"}"#.to_vec()
    })
}

fn encode_ok_data(data: serde_json::Value) -> Vec<u8> {
    let resp = HostResponse::Ok { data };
    serde_json::to_vec(&resp).unwrap_or_else(|_| {
        br#"{"kind":"err","code":"internal","message":"[stratum] serialization failure"}"#.to_vec()
    })
}

fn encode_http_ok(status: u16, headers: Vec<(String, String)>, body: String) -> Vec<u8> {
    let envelope = HttpResponseEnvelope::Ok {
        status,
        headers,
        body,
    };
    serde_json::to_vec(&envelope).unwrap_or_else(|_| {
        br#"{"kind":"err","code":"internal","message":"[stratum] serialization failure"}"#.to_vec()
    })
}

fn truncate_msg(msg: &str, max: usize) -> String {
    if msg.len() <= max {
        msg.to_string()
    } else {
        let mut s = msg.chars().take(max).collect::<String>();
        s.push('…');
        s
    }
}

/// Route a validated request to the correct host backend.
///
/// Permission gating is performed here using the executing plugin's manifest.
/// When no plugin is executing (host-test path through the Tauri command), the
/// caller (the user invoking the command) is implicitly permitted, so no gate
/// applies.
fn dispatch(
    func: &HostFunction,
    plugin: &Option<PluginState>,
    bytes: &[u8],
    host: &Arc<dyn HostApi>,
    rt: &Arc<tokio::runtime::Runtime>,
    plugin_id: &str,
) -> HostOutcome {
    // The reserved `log` function needs no permission (always allowed).
    if *func != HostFunction::Log {
        let required = func.required_permission();
        if let Some(p) = plugin {
            if let Err(e) = p.manifest.permissions.check_strict(&required) {
                return HostOutcome::Err {
                    code: PluginErrorCode::PluginDenied,
                    message: format!(
                        "[stratum] permission '{}' not granted for plugin '{plugin_id}' ({e})",
                        required
                    ),
                };
            }
        }
    }

    match func {
        HostFunction::Log => handle_log(bytes),
        HostFunction::NoteRead => handle_note_read(bytes, host, rt, plugin_id),
        HostFunction::NoteWrite => handle_note_write(bytes, host, rt, plugin_id),
        HostFunction::HttpRequest => handle_http_request(bytes, host, rt, plugin_id),
    }
}

// --- individual host handler bodies -----------------------------------------

/// `pkm.log` — always succeeds; unknown levels fall back to `info`.
fn handle_log(bytes: &[u8]) -> HostOutcome {
    if let Ok(req) = serde_json::from_slice::<serde_json::Value>(bytes) {
        let level = req.get("level").and_then(|v| v.as_str()).unwrap_or("info");
        let message = req
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let message = truncate_msg(message, crate::abi::LOG_MESSAGE_MAX);
        match level {
            "trace" => tracing::trace!("[stratum][plugin] {message}"),
            "debug" => tracing::debug!("[stratum][plugin] {message}"),
            "warn" => tracing::warn!("[stratum][plugin] {message}"),
            "error" => tracing::error!("[stratum][plugin] {message}"),
            _ => info!("[stratum][plugin] {message}"),
        }
    } else {
        warn!("[stratum] pkm.log received non-JSON payload");
    }
    // Contract §4.1: log response is always `{ "kind": "ok", "data": { "logged": true } }`.
    static OK: &str = "{\"kind\":\"ok\",\"data\":{\"logged\":true}}";
    HostOutcome::Ok(OK.to_string())
}

fn handle_note_read(
    bytes: &[u8],
    host: &Arc<dyn HostApi>,
    rt: &Arc<tokio::runtime::Runtime>,
    plugin_id: &str,
) -> HostOutcome {
    #[derive(serde::Deserialize)]
    struct NoteReadReq {
        path: String,
    }
    let Ok(req) = serde_json::from_slice::<NoteReadReq>(bytes) else {
        return HostOutcome::Err {
            code: PluginErrorCode::InvalidArgument,
            message: "[stratum] note_read: invalid request payload".to_string(),
        };
    };
    let host = host.clone();
    rt.block_on(async move {
        match host.read_note(&req.path).await {
            Ok(note) => {
                debug!(
                    "[stratum] plugin {plugin_id} read note '{}' ({} bytes)",
                    note.path,
                    note.content.len()
                );
                HostOutcome::Ok(
                    String::from_utf8(encode_ok_data(serde_json::json!({
                        "path": note.path,
                        "content": note.content,
                        "mtime": note.mtime,
                    })))
                    .unwrap_or_default(),
                )
            }
            Err(code) => HostOutcome::Err {
                code,
                message: format!("[stratum] note_read('{}') failed", req.path),
            },
        }
    })
}

fn handle_note_write(
    bytes: &[u8],
    host: &Arc<dyn HostApi>,
    rt: &Arc<tokio::runtime::Runtime>,
    plugin_id: &str,
) -> HostOutcome {
    #[derive(serde::Deserialize)]
    struct NoteWriteReq {
        path: String,
        content: String,
    }
    let Ok(req) = serde_json::from_slice::<NoteWriteReq>(bytes) else {
        return HostOutcome::Err {
            code: PluginErrorCode::InvalidArgument,
            message: "[stratum] note_write: invalid request payload".to_string(),
        };
    };
    let host = host.clone();
    rt.block_on(async move {
        match host.write_note(&req.path, &req.content).await {
            Ok(()) => {
                debug!(
                    "[stratum] plugin {plugin_id} wrote note '{}' ({} bytes)",
                    req.path,
                    req.content.len()
                );
                // Contract §4.3: `{ "kind": "ok", "data": { "path": ..., "written": true } }`.
                HostOutcome::Ok(
                    String::from_utf8(encode_ok_data(serde_json::json!({
                        "path": req.path,
                        "written": true,
                    })))
                    .unwrap_or_default(),
                )
            }
            Err(code) => HostOutcome::Err {
                code,
                message: format!("[stratum] note_write('{}') failed", req.path),
            },
        }
    })
}

fn handle_http_request(
    bytes: &[u8],
    host: &Arc<dyn HostApi>,
    rt: &Arc<tokio::runtime::Runtime>,
    plugin_id: &str,
) -> HostOutcome {
    #[derive(serde::Deserialize)]
    struct HttpReq {
        method: Option<String>,
        url: String,
        headers: Option<serde_json::Map<String, serde_json::Value>>,
        body: Option<String>,
        timeout_ms: Option<u64>,
    }

    let Ok(req) = serde_json::from_slice::<HttpReq>(bytes) else {
        return HostOutcome::Err {
            code: PluginErrorCode::InvalidArgument,
            message: "[stratum] http_request: invalid request payload".to_string(),
        };
    };

    let method_label = req.method.as_deref().unwrap_or("GET").to_string();
    let url_label = req.url.clone();

    let host_req = match crate::abi::to_host_request(
        req.method,
        &req.url,
        req.headers,
        req.body,
        req.timeout_ms,
    ) {
        Ok(r) => r,
        Err(code) => {
            return HostOutcome::Err {
                code,
                message: "[stratum] http_request: invalid request".to_string(),
            };
        }
    };

    let host = host.clone();

    rt.block_on(async move {
        match host.http_request(host_req).await {
            Ok(resp) => {
                debug!(
                    "[stratum] plugin {plugin_id} http_request -> status {}",
                    resp.status
                );
                HostOutcome::Ok(
                    String::from_utf8(encode_http_ok(resp.status, resp.headers, resp.body))
                        .unwrap_or_default(),
                )
            }
            Err(code) => HostOutcome::Err {
                code,
                message: format!("[stratum] http_request({method_label} {url_label}) failed"),
            },
        }
    })
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
        bail!("Read out of bounds: {start}..{end} > memory size {mem_size}");
    }

    let mut buf = vec![0u8; count];
    memory
        .read(store, start, &mut buf)
        .with_context(|| "Memory read failed")?;
    Ok(buf)
}

/// Write `data` into WASM linear memory starting at `ptr`.
#[cfg(test)]
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
        bail!("Write out of bounds: {start}..{end} > memory size {mem_size}");
    }

    memory
        .write(store, start, data)
        .with_context(|| "Memory write failed")?;
    Ok(())
}

/// Write the response envelope to WASM memory at offset 0, growing the memory
/// if needed (contract §3.2: "the host grows the memory as needed").
fn write_response(
    memory: &wasmtime::Memory,
    caller: &mut wasmtime::Caller<'_, RuntimeContext>,
    response: &[u8],
) -> anyhow::Result<()> {
    let avail = memory.data_size(&caller);
    if response.len() > avail {
        const PAGE_SIZE: usize = 64 * 1024;
        let pages = (response.len() - avail).div_ceil(PAGE_SIZE) as u64;
        memory.grow(&mut *caller, pages)?;
    }
    memory.write(&mut *caller, 0, response)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
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
}
