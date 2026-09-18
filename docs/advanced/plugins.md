# Plugins API Specification

> **Status: NORMATIVE.** This document is the authoritative contract for the Stratum
> plugin system (ADR-0004). Backend, frontend, and QA build against it. The
> implementation and integration tasks make the runtime match this document exactly;
> do not treat any described behavior as aspirational.

## 1. Overview

Stratum plugins are WebAssembly modules executed in an embedded wasmtime runtime,
fully sandboxed from the host filesystem and network. A plugin declares the
capabilities it needs; the host enforces them. Plugins interact with the vault
exclusively through the host function ABI in this document.

The plugin system has **three surfaces**:

| Surface | Owner | Artifact |
|---------|-------|----------|
| Plugin manifest + host ABI (this doc) | architect | contract |
| Runtime host function implementations | backend | `crates/pkm-plugin` |
| Tauri commands + vault loading | backend | `src-tauri` |
| Management UI | frontend | `src/` (React) |

## 2. Plugin packaging

### 2.1 Single-file format (canonical)

A plugin is one file:

- `plugin.wasm` — wasm32 core module (unsuffixed; **not** a `component`), with an
  embedded JSON manifest in the custom section named `stratum:manifest`.

The file is placed in a vault at `<vault>/.pkm/plugins/<name>/plugin.wasm`.
Load order inside that directory is undefined; a plugin must not depend on it.

### 2.2 Legacy sidecar manifest

For compatibility, a manifest is also accepted from a sibling file
`<entry>.wasm.manifest.json` beside the `.wasm` file. This is **read-only
compatibility**: new plugins should embed the manifest. When both exist, the
embedded manifest wins and a `WARN` is logged.

### 2.3 Manifest schema

The manifest is a UTF-8 JSON object. Its schema is v1.

```json
{
  "schema_version": 1,
  "id": "com.example.my-plugin",
  "name": "My Plugin",
  "version": "0.1.0",
  "description": "Optional one-line description",
  "author": "Optional author or org",
  "entry": "plugin.wasm",
  "permissions": ["file:read", "file:write", "network"],
  "hooks": {
    "onSave": true,
    "onOpen": true,
    "onLink": false,
    "onSearch": true,
    "onMyHook": true
  }
}
```

| Field | Type | Required | Rules |
|-------|------|----------|-------|
| `schema_version` | integer | yes | Must be `1`. Reject otherwise with `ConfigError`. |
| `id` | string | yes | Unique reverse-DNS id. Regex `^[A-Za-z0-9][A-Za-z0-9._-]*$`, ≤128 chars. |
| `name` | string | yes | Display name, ≤64 chars. |
| `version` | string | yes | Semver, ≤32 chars. |
| `description` | string | no | ≤512 chars. |
| `author` | string | no | ≤128 chars. |
| `entry` | string | yes | File name of the wasm module. Default `plugin.wasm`. |
| `permissions` | array<string> | yes | See §5 Permission model. Unknown permission ⇒ `ConfigError`. |
| `hooks` | object | no | Default: no hooks. Keys `on*`, values boolean. See §8. |

Unknown fields are ignored (forward compatibility). A manifest that fails JSON
parse, schema validation, or permission validation yields
`PluginLoadError`/`ConfigError` and the plugin does not load.

### 2.4 Example manifest (embedded custom section, JSON)

```json
{
  "schema_version": 1,
  "id": "com.example.exclaimer",
  "name": "Exclaimer",
  "version": "0.1.0",
  "description": "Appends a signature block to every saved note.",
  "permissions": ["file:read", "file:write"],
  "hooks": { "onSave": true }
}
```

### 2.5 Rust manifest type

The Rust type is exactly:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    #[serde(default = "default_entry")]
    pub entry: String,
    pub permissions: Vec<String>,
    #[serde(default)]
    pub hooks: HashMap<String, bool>,
}

fn default_entry() -> String { "plugin.wasm".to_string() }
```

## 3. Host function ABI

### 3.1 Module layout

Plugins are instantiated as **core modules** with **no WASI context**. The host
linker provides a single import namespace `"pkm"` with the imports below, plus the
reserved no-op `"wasi_snapshot_preview1"` module (see §7.1).

### 3.2 Imported functions

Every host function has the WASM type `(param i32 i32) (result i32)`:

- `(param i32 i32)` — a pointer and byte length locating a UTF-8 JSON request in
  the plugin's linear memory.
- `(result i32)` — the byte length of the JSON response that the host wrote at
  **offset 0** of the plugin's linear memory (the host grows the memory as needed).

| Import name | WASM type | Permission |
|-------------|-----------|------------|
| `pkm.log` | `(i32,i32)->i32` | none (always allowed) |
| `pkm.note_read` | `(i32,i32)->i32` | `file:read` |
| `pkm.note_write` | `(i32,i32)->i32` | `file:write` |
| `pkm.http_request` | `(i32,i32)->i32` | `network` |

There are no other host functions. `HostFunction` is an ABI-frozen enum:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HostFunction { Log, NoteRead, NoteWrite, HttpRequest }
```

### 3.3 Envelope

All requests and responses are JSON strings, each bounded at `HOST_PAYLOAD_MAX`
= 1 MiB (host → guest and guest → host). A request larger than the bound yields
`InvalidArgument`.

Every host callback returns the **success envelope**:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum HostResponse {
    Ok { data: serde_json::Value },
    Err { code: PluginErrorCode, message: String },
}
```

`pkm.http_request` returns a dedicated envelope:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum HttpResponseEnvelope {
    Ok { status: u16, headers: Vec<(String, String)>, body: String },
    Err { code: PluginErrorCode, message: String },
}
```

### 3.4 Guest-side contract

- The request payload is caller-owned guest memory; the host must not read beyond
  the given `(ptr, len)`.
- The response is written beginning at guest linear-memory offset **0**. The host
  grows the memory if needed and returns the response byte length.
- The guest reads `memory[0..responselen]` as the response. The returned length is
  an upper bound — the response may be shorter; bytes beyond `responselen` are
  undefined.
- A host callback that throws (trap, guest OOM, missing permission, invalid
  request) is converted by the host into `Err` in the response envelope; it never
  propagates a trap to the guest if the guest followed the ABI.

## 4. Host function specifications

### 4.1 `pkm.log`

Logs a message through the host `tracing` logger, preserving the `[stratum]`
context-prefix convention.

Request:

```json
{ "level": "info", "message": "hello from plugin" }
```

| Field | Type | Required | Rules |
|-------|------|----------|-------|
| `level` | string | no | One of `trace, debug, info, warn, error`. Default `info`. Unknown level ⇒ `info`. |
| `message` | string | yes | ≤4096 chars; longer truncated. |

Response (always succeeds): `{"kind":"ok","data":{"logged":true}}`.

Errors: none.

### 4.2 `pkm.note_read`

Reads the raw content of a note **relative to the vault root**, with an
SSRF-style path-traversal guard.

Request:

```json
{ "path": "notes/foo.md" }
```

| Field | Type | Required | Rules |
|-------|------|----------|-------|
| `path` | string | yes | Vault-relative POSIX path. Empty ⇒ `InvalidArgument`. |

Resolution rules (mirror `resolve_safe_path` in `src-tauri/src/commands/page.rs`):

1. Reject on `..` escaping the vault root ⇒ **must** stay under the canonicalized
   vault root.
2. The note must exist; otherwise `Err { code: "note_not_found", ... }`.
3. Reads are UTF-8; non-UTF-8 ⇒ `PluginRuntimeError`.

Response:

```json
{
  "kind": "ok",
  "data": {
    "path": "notes/foo.md",
    "content": "raw bytes of the note",
    "mtime": "2026-09-17T00:00:00Z"
  }
}
```

| Field | Type | Notes |
|-------|------|-------|
| `path` | string | The vault-relative path that was read (normalized). |
| `content` | string | Full text content (frontmatter + body, exactly as on disk). |
| `mtime` | string | RFC 3339 UTC `modified` timestamp, or an empty string if unavailable. |

Errors: `note_not_found`, `invalid_argument`, `plugin_runtime_error`.

**There is no fake note.** The content is read from the vault filesystem.

### 4.3 `pkm.note_write`

Writes (create or overwrite) a note relative to the vault root, creating parent
directories as needed.

Request:

```json
{ "path": "notes/foo.md", "content": "text to write" }
```

| Field | Type | Required | Rules |
|-------|------|----------|-------|
| `path` | string | yes | Vault-relative POSIX path. Empty ⇒ `InvalidArgument`. Traversal ⇒ `NoteWriteFailed`. |
| `content` | string | yes | New file content (raw, UTF-8). |

Behavior:

1. Resolve the path safely under the vault root (write variant of the safe-path
   check, which does **not** require the target to pre-exist).
2. `create_dir_all` the parent directory.
3. Write atomically (write to temp file in the same directory, then rename).
4. On success the note is on disk, plain markdown, no block-store side effects.

Response:

```json
{ "kind": "ok", "data": { "path": "notes/foo.md", "written": true } }
```

Errors: `note_write_failed` (including traversal and any I/O failure). A failed
write must **propagate** — it is never reported as success.

### 4.4 `pkm.http_request`

Performs a single asynchronous HTTP request with an enforced timeout and an
SSRF guard against the vault's `network.allowlist`.

Request:

```json
{
  "method": "GET",
  "url": "https://example.com/data.json",
  "headers": { "Accept": "application/json" },
  "body": null,
  "timeout_ms": 100
}
```

| Field | Type | Required | Rules |
|-------|------|----------|-------|
| `method` | string | yes | `GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS`. Default `GET`. Unknown ⇒ `InvalidArgument`. |
| `url` | string | yes | Absolute http(s) URL (scheme required). Else `InvalidArgument`. |
| `headers` | object | no | String values; folded, repeated headers not supported. |
| `body` | string\|null | no | Request body, UTF-8. Omitted ⇒ `null`. |
| `timeout_ms` | number | no | Default `10000`. Bounded to `[1, 60000]`; out of range ⇒ `InvalidArgument`. |

Behavior:

1. **SSRF guard (always on).** Host resolves the URL host. It is allowed if it
   (a) matches the vault `network.allowlist` (host or CIDR entries from
   `pkm_core::Config`), or (b) resolves to a loopback/private address
   (RFC 1918, link-local, loopback, unique-local IPv6). Otherwise the call is
   rejected without a network round-trip ⇒ `Err { code: "http_ssid", ... }`.
2. **Timeout.** The request is aborted at `timeout_ms` ⇒ `Err { code: "http_timeout" }`.
3. **Transport errors** (DNS, connect, TLS, abort) ⇒ `Err { code: "http_transport" }`.
4. **HTTP response.** Body is read fully (capped at 1 MiB; larger truncated).
   Response headers returned as received.

Response (HTTP success — any status):

```json
{
  "kind": "ok",
  "status": 200,
  "headers": [["content-type", "application/json"]],
  "body": "..."
}
```

HTTP status 400..=599 is returned as an **error** to the plugin:

```json
{ "kind": "err", "code": "http_status", "message": "HTTP 404 from https://example.com/x" }
```

`2xx`/`3xx` are success; all other statuses are errors as above.

Errors: `http_transport`, `http_timeout`, `http_ssid`, `http_status`,
`invalid_argument` (bad method/URL/timeout), `plugin_denied` (missing `network`).

## 5. Permission model

### 5.1 Permission vocabulary

Only the following are valid manifest entries:

| Permission | Protects | Host functions |
|------------|----------|----------------|
| `file:read` | reading vault notes | `pkm.note_read` |
| `file:write` | writing vault notes | `pkm.note_write` |
| `network` | HTTP egress | `pkm.http_request` |
| `git` | Git operations (2|2) | lifecycle-gated |
| `exec` | Running subprocesses | lifecycle-gated |
| `all` | every capability above | all host functions |

Any other string ⇒ manifest rejected with `ConfigError`.

### 5.2 Enforcement

- A host function invoked without its required permission is aborted and surfaces
  `Err { code: "plugin_denied", message: "permission 'file:read' not granted" }`
  to the plugin.
- Grants come **only** from the plugin's embedded manifest (or legacy sidecar).
  `pkm_core::PluginConfig.permissions` is an enablement/config override layer only
  and must not be required for the manifest-derived grant to take effect.
- `all` implies `file:read`, `file:write`, `network`, `git`, `exec`.

## 6. Error handling

### 6.1 Canonical error codes

`PluginErrorCode` is a string enum serialized snake_case. It is ABI-frozen for the
v0.7.x series.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginErrorCode {
    PluginDenied,       // permission not granted
    PluginNotFound,     // no plugin with this id is loaded
    PluginLoadError,    // manifest/compile/instantiation failure
    PluginRuntimeError, // guest or host trap / runtime failure
    NoteNotFound,       // note_read: target missing
    NoteWriteFailed,    // note_write: I/O or traversal failure
    HttpTransport,      // http: dns/connect/tls/abort
    HttpTimeout,        // http: deadline exceeded
    HttpSsid,           // http: blocked by SSRF guard
    HttpStatus,         // http: HTTP status 400..=599
    InvalidArgument,    // malformed request payload
    ConfigError,        // manifest/config invalid
    Internal,           // unexpected host error
}
```

### 6.2 Envelope error shape

All error envelopes are `Err { code, message }`. `code` is always one of
§6.1; `message` is a human-readable string, ≤1024 chars, and SHOULD contain
`[stratum]` so logs remain greppable.

### 6.3 Mapping from crate errors

| Crate error (pkm_core) | PluginErrorCode |
|------------------------|-----------------|
| `PkmError::NoteNotFound(..)` | `note_not_found` |
| `PkmError::Plugin(..)` | `plugin_load_error` |
| `PkmError::Config(..)` | `config_error` |
| `PkmError::Io(..)` on write | `note_write_failed` |
| any other `PkmError` | `plugin_runtime_error` / `internal` |

### 6.4 Logging & visibility

- Every host call is logged at `trace` (request envelope) and `debug` (outcome).
- Host call failures are logged at `warn` with the plugin id, function, and error code.
- A plugin that fails to load logs `error` with `PluginLoadError` details; the
  error is also visible through the Tauri `plugins_list` command output.

## 7. Runtime instantiation & lifecycle

### 7.1 Instantiation

- Engine: wasmtime, `wasm32-unknown-unknown` target, **no** WASI context.
- The linker registers:
  - the `pkm` module with the four imports in §3.2, and
  - the reserved `wasi_snapshot_preview1` module, whose `fd_write` is a no-op,
    so toolchains that reference it (e.g. `wasm32-wasi`-built modules) instantiate
    without error.
- A module that imports any other namespace/name fails instantiation with
  `PluginLoadError` and a descriptive message; the plugin does not load.

### 7.2 Lifecycle states

```
        ┌───┐   scan    ┌────────────┐   validate/compile   ┌────────┐
        │   │ ────────▶ │ Discovered │ ───────────────────▶ │ Loading│
        └───┘           └────────────┘                      └───┬────┘
                                              success/failure   │
                                     ┌──────────────────────────┘
                                     ▼
                              ┌──────────────┐   enable   ┌───────────┐
                              │    Ready     │ ◀────────── │ Disabled  │
                              └──────┬───────┘   disable  └───────────┘
                                     │
                                     ▼ reload (re-instantiate from disk)
```

- **Discovered:** manifest found on disk (embedded or sidecar), not yet validated.
- **Loading:** manifest validated, module compiled and instantiated.
- **Ready:** host functions registered, hooks armed, plugin operational.
- **Failed:** manifest/compile/instantiation failure → `PluginLoadError`, no partial
  plugin is ever active. `status` reports `"error"` plus the load error message.
- **Disabled:** loaded but hooks and host dispatch are skipped; state retained.
- **Reload:** re-read manifest + re-instantiate from disk (used after an update).

Transitions are only reachable through the Tauri command surface or the vault
scan at startup; they are not implicit.

### 7.3 Startup scan

On vault open, the host scans `<vault>/.pkm/plugins/` for entries containing a
`plugin.wasm` (embedded manifest) landing at the root of each subdirectory, plus
any `*.wasm.manifest.json` sibling (legacy). Every discovered plugin is loaded
(failure → `Failed` state, logged). Enablement respects the `plugins.enabled`
list in `pkm_core::Config`.

## 8. Hooks

Hooks are defined by the manifest `hooks` map (key `on<Name>: boolean`). The host
dispatches the hook by invoking a guest-exported function whose name is derived as:

```
call <module>.<hook_name>  (e.g. register_hook("onSave") ⇒ guest export "onSave")
```

A registered hook whose guest export is missing is a no-op (logged at `debug`).
Presently recognized hook names and their host call signatures:

| Hook | Host payload (JSON string) | Guest export |
|------|----------------------------|--------------|
| `onSave` | `{"path": "…", "content": "…"}` | `onSave` |
| `onOpen` | `{"path": "…"}` | `onOpen` |
| `onLink` | `{"path": "…", "links": ["…"]}` | `onLink` |
| `onSearch` | `{"query": "…", "limit": N}` | `onSearch` |
| any other `on*` | `{"args": []}` | `<name>` |

Hook dispatch runs synchronously within the triggering host operation; a hook
that traps logs the error and does not abort the surrounding operation. **Hook
delivery:** `onSave` and `onLink` are dispatched from the page save flow
(`dispatch_on_save` / `dispatch_on_link` — the link targets are extracted from
the saved content via `pkm_markdown::linker::extract_links`). `onOpen` is
dispatched from the page open flow (`dispatch_on_open` in `open_page`), and
`onSearch` from the full-text search flow (`dispatch_on_search` in
`search_blocks`). Any hook that traps or fails is logged and skipped — it never
aborts the surrounding operation.

## 9. Tauri command surface

The frontend calls only the commands below. Names, argument names, and DTO shapes
are normative.

All commands return `Result<_, String>` where the `Err` string is the
`message` of a `PluginErrorCode` mapped per §6.3. Command modules live in
`src-tauri/src/commands/plugins.rs`.

Common DTOs:

```rust
#[derive(Debug, Clone, Serialize)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub status: String,       // "ready" | "disabled" | "error"
    pub enabled: bool,
    pub permissions: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>, // set when status == "error"
}

#[derive(Debug, Clone, Serialize)]
pub struct PluginListResult {
    pub plugins: Vec<PluginInfo>,
}
```

### 9.1 `plugins_list`

List installed plugins with status. No arguments.

```
invoke("plugins_list") -> PluginListResult
```

Scope: Tauri core (accessible from any window). No vault open ⇒ empty list.

### 9.2 `plugins_enable`

Enables a disabled plugin and re-arms hooks.

```
invoke("plugins_enable", { id: string }) -> PluginInfo
```

Errors: `plugin_not_found` (not loaded/known), `plugin_load_error` (re-load fails).

### 9.3 `plugins_disable`

Disables a running plugin (hooks + dispatch skipped, state retained).

```
invoke("plugins_disable", { id: string }) -> PluginInfo
```

Errors: `plugin_not_found`.

### 9.4 `plugins_reload`

Re-instantiates the plugin from disk (re-reads manifest, recompiles).

```
invoke("plugins_reload", { id: string }) -> PluginInfo
```

Errors: `plugin_not_found`, `plugin_load_error` (new manifest invalid → plugin
enters `Failed`).

### 9.5 `plugins_status`

Returns full status for one plugin (alias of the per-plugin entry in
`plugins_list`).

```
invoke("plugins_status", { id: string }) -> PluginInfo
```

Errors: `plugin_not_found`.

### 9.6 `plugin_note_read` (host test)

Runs the **same** `note_read` backend used by plugins, without requiring a plugin,
so authors/QA can verify path behavior.

```
invoke("plugin_note_read", { path: string }) -> { "path": string, "content": string, "mtime": string }
```

Errors: `note_not_found`, `invalid_argument`, `plugin_runtime_error`.

### 9.7 `plugin_http_request` (host test)

Runs the **same** `http_request` backend used by plugins, without a plugin.

```
invoke("plugin_http_request", { method?, url, headers?, body?, timeout_ms? })
   -> { "status": u16, "headers": [string,string][], "body": string }
```

Errors: `http_transport`, `http_timeout`, `http_ssid`, `http_status`,
`invalid_argument`, `plugin_denied`.

## 10. SSRF guard details

The allowlist is read from `pkm_core::Config` field
`network.allowlist` (list of host strings or CIDR strings). Evaluation order:

1. If the URL host is in the allowlist → allow.
2. If the URL host resolves to an address in a private/loopback/link-local range
   → allow (this makes `localhost` and LAN access work by default).
3. Otherwise → `http_ssid`.

Resolution is performed once per request. Misleading DNS rebinding is out of
scope for this contract (documented limitation).

## 11. Out of scope (v0.7.x)

- WASM components / WASI preview 2 and 3.
- Plugin-to-plugin imports.
- Host-function `pkm.db_query` (future).
- Marketplace / remote plugin registry.
- Sandbox CPU/memory accounting beyond wasmtime defaults.

## 12. Rust type/trait definitions (reference)

The following drive the host implementations; they are normative.

```rust
// crates/pkm-plugin/src/abi.rs
use serde::{Deserialize, Serialize};

pub const HOST_PAYLOAD_MAX: usize = 1 << 20; // 1 MiB
pub const HTTP_TIMEOUT_MS_DEFAULT: u64 = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HostFunction { Log, NoteRead, NoteWrite, HttpRequest }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginErrorCode {
    PluginDenied, PluginNotFound, PluginLoadError, PluginRuntimeError,
    NoteNotFound, NoteWriteFailed, HttpTransport, HttpTimeout, HttpSsid,
    HttpStatus, InvalidArgument, ConfigError, Internal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum HostResponse {
    Ok { data: serde_json::Value },
    Err { code: PluginErrorCode, message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum HttpResponseEnvelope {
    Ok { status: u16, headers: Vec<(String, String)>, body: String },
    Err { code: PluginErrorCode, message: String },
}

/// The notes/filesystem backend the plugin runtime talks to.
/// Implemented by the host (src-tauri) and injected into the runtime.
#[async_trait::async_trait]
pub trait HostApi: Send + Sync {
    async fn read_note(&self, rel_path: &str) -> Result<HostNote, PluginErrorCode>;
    async fn write_note(&self, rel_path: &str, content: &str) -> Result<(), PluginErrorCode>;
    async fn http_request(
        &self,
        req: HostHttpRequest,
    ) -> Result<HostHttpResponse, PluginErrorCode>;
}

pub struct HostNote {
    pub path: String,
    pub content: String,
    pub mtime: String,
}

pub struct HostHttpRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
    pub timeout_ms: u64,
}

pub struct HostHttpResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
}
```

## 13. Glossary

- **Envelope:** the `{kind, ...}` JSON wrapper every host callback returns.
- **Host function / import:** a function the host provides to the guest (`pkm.*`).
- **Guest export:** a WASM function a plugin exports (e.g. `onSave`).
- **SSRF guard:** the allowlist/private-address check on `http_request`.
- **Vault root:** the canonical root of the currently open vault.
