//! Example Stratum plugin: title-case note content on every save.
//!
//! This is the canonical reference for writing a real Rust plugin. It:
//!
//! 1. Builds with `cargo build --target wasm32-wasip1` (stable Rust, `std`).
//! 2. Embeds its manifest in the `stratum:manifest` custom section, so the
//!    resulting `plugin.wasm` is complete and self-describing — copy it into a
//!    vault and it is discovered, validated, and enabled from the Plugins UI.
//! 3. Implements the `onSave` hook (the host ABI: `(i32, i32) -> i32`).
//! 4. Uses the `pkm.*` host API for all vault I/O — never the host process's
//!    filesystem directly. `pkm.note_read`/`pkm.note_write` (and `pkm.log`,
//!    `pkm.http_request`) are the only sanctioned way to touch the vault.
//!
//! The WASI preview1 runtime (fed by the same host) makes `std`'s CRT
//! happy — heap allocation, `eprintln!`, clocks all work. No filesystem is
//! preopened: raw `std::fs` does not see the vault. Use the host API instead.

use std::slice;

// ---------------------------------------------------------------------------
// Manifest (embedded in the wasm as a custom section)
// ---------------------------------------------------------------------------
//
// The manifest is the plugin's contract with the host: schema version, unique
// reverse-DNS id, display name, semver version, the capabilities it needs
// (`permissions`), and the hooks it wants to receive. `#[link_section]` emits
// this literal as the `stratum:manifest` custom section in the compiled wasm —
// the same artifact that installs also declares itself, so a two-step
// "build then inject a manifest" workflow is never needed.
//
// The byte-array length below MUST match the literal exactly (wasm custom
// sections cannot carry a `&str` indirection). Keep them in lockstep.
#[used]
#[link_section = "stratum:manifest"]
static MANIFEST: [u8; 152] = *b"{\"schema_version\":1,\"id\":\"com.example.titlecase\",\"name\":\"Title Case\",\"version\":\"0.1.0\",\"permissions\":[\"file:read\",\"file:write\"],\"hooks\":{\"onSave\":true}}";

// ---------------------------------------------------------------------------
// Host API imports (documented in docs/advanced/plugins.md §3)
// ---------------------------------------------------------------------------
//
// Every host function has the type (i32, i32) -> i32. The first pair is a
// pointer + byte length locating a UTF-8 JSON request in *linear memory*. The
// host writes the JSON response envelope at memory offset 0 and returns its
// byte length.
//
// The wasm import name is the Rust identifier, so the externs below are named
// exactly `log` / `note_write` — matching the `pkm.*` imports the host linker
// registers. `note_read` and `http_request` exist too; this example only needs
// the write path, the save payload already carries the note's content.
#[link(wasm_import_module = "pkm")]
extern "C" {
    /// `pkm.log` — no permission required. Payload: `{"level","message"}`.
    fn log(ptr: i32, len: i32) -> i32;
    /// `pkm.note_write` — requires `file:write`. Payload: `{"path","content"}`.
    fn note_write(ptr: i32, len: i32) -> i32;
}

/// A host call site: bytes at `result[0..result.len()]` after the call return
/// the host's JSON response envelope in linear memory at offset 0.
struct HostResp {
    len: i32,
}

fn call_host(f: unsafe extern "C" fn(i32, i32) -> i32, req: &[u8]) -> HostResp {
    let len = unsafe { f(req.as_ptr() as i32, req.len() as i32) };
    HostResp { len }
}

/// Read `len` bytes of linear memory starting at offset 0 (the host response
/// area) into an owned `Vec`.
///
/// On wasm32 there is no host null page: linear-memory address 0 is valid and
/// dereferenceable, so `0 as *const u8` is correct despite rustc's
/// `invalid_null_arguments` lint (a host-address-space concept that does not
/// apply here).
#[allow(invalid_null_arguments)]
fn read_linear(len: i32) -> Vec<u8> {
    if len <= 0 {
        return Vec::new();
    }
    let src = unsafe { slice::from_raw_parts(0 as *const u8, len as usize) };
    src.to_vec()
}

/// Write `data` to linear memory starting at offset 0 (the host reads the
/// hook's return length bytes from here).
#[allow(invalid_null_arguments)]
fn write_linear(data: &[u8]) {
    if data.is_empty() {
        return;
    }
    let dst = unsafe { slice::from_raw_parts_mut(0 as *mut u8, data.len()) };
    dst.copy_from_slice(data);
}

/// Uppercase the first alphabetic character of every whitespace-delimited
/// word; leave the rest of the word lower-cased. Pure ASCII example logic.
fn title_case(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut at_word_start = true;
    for ch in s.chars() {
        if ch.is_ascii_whitespace() || ch.is_digit(10) {
            at_word_start = true;
            out.push(ch);
        } else if at_word_start {
            for c in ch.to_uppercase() {
                out.push(c);
            }
            at_word_start = false;
        } else {
            for c in ch.to_lowercase() {
                out.push(c);
            }
        }
    }
    out
}

/// The `onSave` hook. The host calls this synchronously after a note is
/// saved, with a JSON payload `{"path": "<vault-relative path>", "content":
/// "<full text>"}` located at `(ptr, len)` in linear memory.
///
/// Returns the byte length of the JSON response the plugin left at offset 0.
#[no_mangle]
pub extern "C" fn onSave(ptr: i32, len: i32) -> i32 {
    // Linear-memory offset 0 is the ABI scratch area: the host delivers the
    // hook payload there, and any host callback overwrites it with its
    // response. Copy the payload into our own heap before calling anything.
    let payload_src = unsafe { slice::from_raw_parts(ptr as *const u8, len as usize) };
    let payload = serde_json::from_slice::<serde_json::Value>(payload_src);

    // Parse the payload.
    let (path, content) = match payload {
        Ok(v) => {
            let path = v
                .get("path")
                .and_then(|p| p.as_str())
                .unwrap_or("")
                .to_string();
            let content = v
                .get("content")
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .to_string();
            (path, content)
        }
        Err(_) => {
            let out = serde_json::json!({ "error": "invalid_onSave_payload" });
            let bytes = serde_json::to_vec(&out).unwrap_or_default();
            write_linear(&bytes);
            return bytes.len() as i32;
        }
    };

    if path.is_empty() {
        let out = serde_json::json!({ "error": "missing_path" });
        let bytes = serde_json::to_vec(&out).unwrap_or_default();
        write_linear(&bytes);
        return bytes.len() as i32;
    }

    // Transform.
    let transformed = title_case(&content);

    // Persist through the host API. `note_write` requires `file:write`, which
    // the manifest declares. The host writes its response envelope at offset 0
    // — the value returned here is that response's length.
    let req = serde_json::json!({ "path": path, "content": transformed });
    let req_bytes = match serde_json::to_vec(&req) {
        Ok(b) => b,
        Err(_) => {
            let out = serde_json::json!({ "error": "serialization_failure" });
            let bytes = serde_json::to_vec(&out).unwrap_or_default();
            write_linear(&bytes);
            return bytes.len() as i32;
        }
    };
    let note_write_len = call_host(note_write, &req_bytes).len;
    let note_write_resp = read_linear(note_write_len);
    // The host success envelope is `{"kind":"ok","data":{...}}`; failures carry
    // `{"kind":"err","code":..,"message":..}`.
    let note_write_ok = serde_json::from_slice::<serde_json::Value>(&note_write_resp)
        .map(|v| v.get("kind").and_then(|k| k.as_str()) == Some("ok"))
        .unwrap_or(false);

    // Log through the host API (always allowed, no permission required).
    let log_req = serde_json::json!({
        "level": if note_write_ok { "info" } else { "warn" },
        "message": format!("[titlecase] {} -> note_write {}", path, if note_write_ok { "ok" } else { "failed" }),
    });
    let log_bytes = serde_json::to_vec(&log_req).unwrap_or_default();
    call_host(log, &log_bytes);

    // Hook response — written to offset 0 (the host reads `return` bytes from
    // offset 0). Small, diagnostic-only; payload I/O already happened through
    // the host API.
    let out = serde_json::json!({
        "path": path,
        "note_write_status": if note_write_ok { "ok" } else { "failed" },
        "chars": transformed.chars().count(),
    });
    let out_bytes = serde_json::to_vec(&out).unwrap_or_default();
    write_linear(&out_bytes);
    out_bytes.len() as i32
}

// No Rust `main` — this is a library module the host drives by calling the
// exported hook. The `std` runtime (allocator, panic handler) is linked in by
// the wasm32-wasip1 target; a panic unwinds through the ABI boundary.
