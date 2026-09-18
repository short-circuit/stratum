# Plugins

Stratum plugins are WebAssembly modules that extend the app without a rebuild.
They run in an embedded, sandboxed runtime and interact with your vault only
through the host functions they import (`pkm.note_read`, `pkm.note_write`,
`pkm.http_request`, `pkm.log`). The full contract — manifest schema, host ABI,
error codes, and the Tauri command surface — is the normative
[Plugins API Specification](../advanced/plugins.md).

A plugin is a single file, `<vault>/.pkm/plugins/<id>/plugin.wasm`, with an
embedded JSON manifest in the custom section named `stratum:manifest`. A legacy
sidecar manifest (`plugin.wasm.manifest.json` beside the `.wasm`) is still
accepted for reading, but new plugins must embed the manifest.

<!-- SCREENSHOT: [plugins-panel] Plugins panel listing installed plugins with status -->

## Installing a Plugin

1. Build your plugin as a `wasm32` core module (see
   [Writing a Plugin](#writing-a-plugin)).
2. Create the directory `<vault>/.pkm/plugins/<id>/` where `<id>` is the
   reverse-DNS id from the manifest (e.g. `com.example.my-plugin`).
3. Copy the `.wasm` file into that directory as `plugin.wasm`.
4. In the app, open **:material-puzzle: Plugins** and click **Refresh**. The
   plugin is discovered, loaded, and listed.

A newly discovered plugin is loaded but **disabled** unless it is enabled in
the vault config `plugins` list (see [Configuring Plugins](#configuring-plugins)).
Enable it from the panel to arm its hooks.

## Opening Plugins

Click **:material-puzzle: Plugins** in the sidebar or navigate to `/plugins`.

## Viewing Installed Plugins

The panel lists every plugin discovered in your vault plugin directory
(`<vault>/.pkm/plugins/`). Each entry shows:

| Field | Description |
|-------|-------------|
| Name / version | Display name and version from the plugin manifest |
| ID | The unique plugin identifier (manifest `id`) |
| Status | `Ready`, `Disabled`, or `Error` |
| Permissions | The capabilities the plugin requested (`file:read`, `file:write`, `network`, …) |

## Status

| Status | Meaning |
|--------|---------|
| **Ready** | The plugin loaded and its hooks are armed. |
| **Disabled** | The plugin is installed but disabled — hooks and dispatch are skipped, state is retained. |
| **Error** | Loading or a runtime call failed. Click the status chip to expand the error detail. |

A plugin that fails to load never aborts the scan of the rest of the directory:
it is reported with status `Error` and the load error message, and the other
plugins still load.

## Enabling, Disabling & Reloading

- **Enable** — re-arms a disabled plugin's hooks. The change is persisted to the
  vault config.
- **Disable** — stops a running plugin without uninstalling it. The change is
  persisted to the vault config.
- **Reload** — re-instantiates the plugin from disk (re-reads the manifest,
  recompiles). Use after updating a `.wasm` file. If the new file is invalid,
  the plugin enters the `Error` state.

## Testing Host Functions

Each entry has **note_read** and **http_request** buttons. These run the *same*
host-function backend used by plugins, without needing a plugin, so you can
verify path handling and network behavior (e.g. tune the SSRF guard). Results
and errors are shown inline.

## Writing a Plugin

### 1. Choose a toolchain

Any toolchain that produces a `wasm32` **core module** works:

- AssemblyScript / C / Rust targeting `wasm32-unknown-unknown` — recommended.
- A wasm32-wasi build also works: the linker registers a reserved
  `wasi_snapshot_preview1` no-op so such modules instantiate. WASI is not used
  at runtime; use the `pkm.*` imports for all I/O.

### 2. Define the manifest

The manifest is a UTF-8 JSON object embedded in the `stratum:manifest` custom
section of the module. Its schema is version 1 and its fields are validated at
load time.

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
    "onSave": true
  }
}
```

| Field | Required | Rules |
|-------|----------|-------|
| `schema_version` | yes | Must be `1`. |
| `id` | yes | Unique reverse-DNS id; `^[A-Za-z0-9][A-Za-z0-9._-]*$`, ≤128 chars. |
| `name` | yes | Display name, ≤64 chars. |
| `version` | yes | Semver, ≤32 chars. |
| `description` | no | ≤512 chars. |
| `author` | no | ≤128 chars. |
| `entry` | no | File name of the wasm module. Default `plugin.wasm`. |
| `permissions` | yes | See [Permissions](#permissions). Unknown entries make the manifest invalid. |
| `hooks` | no | Keys `on*` with boolean values. See [Git hooks](#hooks). |

A manifest that fails JSON parse, schema validation, or permission validation is
rejected and the plugin does not load.

### 3. Import the host functions

The host linker provides one import namespace, `pkm`, with exactly four
functions. Every host function has the WASM type `(param i32 i32) (result i32)`:

- `(i32, i32)` — a pointer and byte length locating a UTF-8 JSON request in the
  plugin's linear memory.
- `(result i32)` — the byte length of the JSON response that the host wrote at
  **offset 0** of the plugin's linear memory (the host grows the memory as
  needed).

| Import | Type | Needs permission |
|--------|------|------------------|
| `pkm.log` | `(i32, i32) -> i32` | none (always allowed) |
| `pkm.note_read` | `(i32, i32) -> i32` | `file:read` |
| `pkm.note_write` | `(i32, i32) -> i32` | `file:write` |
| `pkm.http_request` | `(i32, i32) -> i32` | `network` |

A module that imports any other namespace or function fails instantiation and
does not load.

### 4. Export hook functions

The host dispatches each hook declared in the manifest by calling the matching
guest export. The guest export has the same signature `(param i32 i32)
(result i32)` and receives the hook payload as a JSON string in linear memory.

| Hook | Host payload (JSON string) | Guest export |
|------|----------------------------|--------------|
| `onSave` | `{"path": "…", "content": "…"}` | `onSave` |
| `onOpen` | `{"path": "…"}` | `onOpen` |
| `onLink` | `{"path": "…", "links": ["…"]}` | `onLink` |
| `onSearch` | `{"query": "…", "limit": N}` | `onSearch` |
| any other `on*` | `{"args": []}` | `<name>` |

A registered hook whose guest export is missing is a no-op (logged at debug).
`onSave` and `onLink` are delivered from the note save flow, `onOpen` from the
note open flow, and `onSearch` from the full-text search flow. A hook that
traps or fails logs the error and never aborts the surrounding operation.

### 5. Guest call pattern

Every host callback returns a JSON response envelope. Success:

```json
{ "kind": "ok", "data": { ... } }
```

Error (with one of the canonical codes from the
[API Specification](../advanced/plugins.md)):

```json
{ "kind": "err", "code": "note_not_found", "message": "..." }
```

`pkm.http_request` returns a dedicated envelope:

```json
{ "kind": "ok", "status": 200, "headers": [["content-type", "application/json"]], "body": "..." }
```

Request payloads must not exceed 1 MiB; responses are capped the same way.
`pkm.log` always succeeds and needs no permission.

### Worked example (WAT)

The following minimal module imports all four host functions and exposes an
`onSave` and `onOpen` hook. It dispatches on a marker prefix in the payload
(`READ:`, `WRITE:`, `HTTP:`, `LOG:` followed by a JSON payload): it locates the
`:` separator at runtime and forwards only the JSON substring that follows it to
the matching host function, then returns the response length the host wrote at
memory offset 0. This is the exact module shape the plugin E2E suite
(`crates/pkm-tests/tests/plugin_e2e.rs`) builds, loads through the real
registry, and runs end-to-end:

```wat
(module
    (import "pkm" "note_read" (func $note_read (param i32 i32) (result i32)))
    (import "pkm" "note_write" (func $note_write (param i32 i32) (result i32)))
    (import "pkm" "http_request" (func $http_request (param i32 i32) (result i32)))
    (import "pkm" "log" (func $log (param i32 i32) (result i32)))

    (memory (export "memory") 1)

    ;; Find the byte position of ':' in the first `len` bytes of `ptr`,
    ;; or 0 if none is found.
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

    ;; Pointer and length of the JSON that follows the first ':'.
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
```

Assemble with `wat2wasm` (from the [wabt](https://github.com/WebAssembly/wabt)
toolkit) and append the `stratum:manifest` custom section containing the JSON
manifest above. The resulting file is the distributable `plugin.wasm`.

### Validating your plugin

The plugin E2E suite in `crates/pkm-tests/tests/plugin_e2e.rs` builds a real
`plugin.wasm` in-process, embeds the manifest, and exercises discovery, loading,
all four host functions against a real on-disk vault, permission denials, the
SSRF guard, and the PluginManager lifecycle. Use it as a runnable reference for
the exact wire contract.

## Permissions

| Permission | Protects | Host functions |
|------------|----------|----------------|
| `file:read` | reading vault notes | `pkm.note_read` |
| `file:write` | writing vault notes | `pkm.note_write` |
| `network` | HTTP egress | `pkm.http_request` |
| `git` | Git operations | lifecycle-gated |
| `exec` | Running subprocesses | lifecycle-gated |
| `all` | every capability above | all host functions |

A host function invoked without its required permission is aborted and surfaces
`{"kind":"err","code":"plugin_denied","message":"permission 'file:read' not
granted"}` to the plugin. Grants come only from the plugin's embedded manifest;
the `permissions` field in the vault config is an enablement/override layer and
is not required for the manifest-derived grant to take effect.

## Configuring Plugins

The vault config at `<vault>/.pkm/config.toml` contains a `[[plugins]]` list
that controls enablement. A plugin present on disk but absent from this list is
loaded disabled. Enabling/disabling from the panel rewrites this list.

```toml
[network]
# Host or CIDR entries the plugin HTTP SSRF guard permits.
allowlist = ["api.example.com", "10.0.0.0/8"]

[[plugins]]
name = "com.example.my-plugin"
enabled = true
wasm_path = ".pkm/plugins/com.example.my-plugin/plugin.wasm"
permissions = ["file:read", "file:write", "network"]
```

The `network.allowlist` field is the SSRF guard for `pkm.http_request`: a target
is allowed if its host matches an entry (host or CIDR), or if it resolves to a
private/loopback/link-local address. `localhost` and LAN addresses work by
default; public hosts must be allow-listed or they are rejected with
`http_ssid` before any network round-trip.

## Hooks

Hooks are defined by the manifest `hooks` map (key `on<Name>: boolean`).
`onSave` and `onLink` are delivered from the note save flow, `onOpen` from the
note open flow, and `onSearch` from the full-text search flow. A hook that
traps logs the error and does not abort the surrounding operation. The
hook signature and payloads are defined in the
[API Specification](../advanced/plugins.md) §8.

## Troubleshooting

### Plugin shows status Error

- Open the status chip for the error detail.
- A manifest that fails validation lists the exact rule violated (e.g. an
  unknown permission string, a bad `schema_version`, or an invalid `id`).
- A module that imports an unknown namespace fails instantiation;
  rebuild it against only the four `pkm.*` imports.
- Verify the file is a real wasm32 core module (`wasmtime`/`wasm2wat` can
  validate it outside Stratum).

### note_read / note_write return errors

- `note_not_found` — the note does not exist under the vault root.
- `note_write_failed` — the write failed (I/O, or the path escapes the vault).
- `plugin_denied` — the plugin lacks `file:read` or `file:write`.

### http_request returns errors

- `http_ssid` — the hostname is not in `network.allowlist` and does not resolve
  to a private/loopback address. Add it to the allowlist.
- `http_timeout` — the request exceeded `timeout_ms` (default 10000 ms, max
  60000 ms).
- `http_status` — the server returned HTTP 400..=599.
- `http_transport` — DNS, connect, or TLS failure.

### Change to a plugin has no effect

Use **Reload** on the plugin entry — plugins are compiled at load time; editing
the `.wasm` on disk does not hot-reload automatically.

## Further reading

- [Plugins API Specification](../advanced/plugins.md) — the normative contract:
  manifest schema, full host ABI, permission model, error codes, SSRF guard,
  lifecycle, and the Tauri command surface.
- [ADR-0004](../development/adr/0004-wasm-plugin-abi.md) — the accepted design
  and its stability guarantees.
- `crates/pkm-tests/tests/plugin_e2e.rs` — the verified end-to-end test suite.
