# ADR-0004: WASM Plugin System — API Contract and Host Function ABI

- **Status:** Accepted (normative — implementation tasks build against this)
- **Date:** 2026-09-17
- **Deciders:** architect (API owner), backend-dev, frontend-dev, QA
- **Supersedes:** none
- **Related:** `docs/advanced/plugins.md` (user-facing guide), crates/pkm-plugin, epic E3

## Context

Stratum's `pkm-plugin` crate provides a WASM (wasmtime) plugin runtime, but the
host functions are stubs (`note_read` returns a fake note, `note_write` always
succeeds, `http_request` is blocked), the crate is not linked into `src-tauri`,
and no UI or vault-level loading exists. Backend, frontend, and QA each need a
single authoritative contract to build against. Any drift between the API spec
and the running code invalidates one or more downstream tasks.

This ADR fixes the contract. It is the normative reference; the implementation
task (`t_b197920d`) must make the runtime match it exactly, and the integration
task (`t_e132158a`) must expose it through the documented Tauri command surface.

## Decision

1. **Runtime + ABI.** Plugins run as WebAssembly in wasmtime (`wasm32-unknown-unknown`
   core module, WASI-free). Host functions are imported from the `"pkm"` module in
   WASM linear-memory via the ABI in [`../advanced/plugins.md`](../advanced/plugins.md)
   ("Host Function ABI" section). Every host call reads its request from WASM memory
   at the guest-supplied offset and writes its response back at **offset 0** of the
   plugin's linear memory, returning the byte length of the response. This convention
   is what the current runtime (`crates/pkm-plugin/src/runtime.rs`) already implements
   for its imports; it is preserved.

2. **Host functions.** Exactly four host imports exist: `pkm.log`, `pkm.note_read`,
   `pkm.note_write`, `pkm.http_request`. There are **no other host functions** in the
   public contract. `log` requires no permission (always allowed).

3. **WASM module type of every host function:** `(param i32 i32) (result i32)` —
   a pointer to a UTF-8 byte span, its length, and a response byte length. The
   implementations cast host-side results into this shape; the guest never sees an
   ABI surface beyond `(i32, i32) -> i32`.

4. **Envelope.** All host request/response payloads are JSON strings, bounded at
   `HOST_PAYLOAD_MAX` (1 MiB) for both directions. Responses use the 1:1 envelope
   defined in the API spec: **every** host callback returns a `HostResponse`
   (success or error shape); the `http_request` host returns a
   `HttpResponseEnvelope` with a nested `HostError` for transport-level failures.

5. **Error codes & mapping.** The canonical machine-readable error set is
   `PluginErrorCode` (`PluginDenied`, `PluginNotFound`, `PluginLoadError`,
   `PluginRuntimeError`, `NoteNotFound`, `NoteWriteFailed`, `HttpTransport`,
   `HttpTimeout`, `HttpSsid`, `HttpStatus`, `InvalidArgument`, `ConfigError`,
   `Internal`). Rust crate errors map onto it per the "Error Mapping" section of the
   spec. HTTP status 400..=599 from a network call is an error surfaced to the plugin
   as `PluginErrorCode::HttpStatus` with the HTTPS response body embedded.

6. **Permissions.** Plugins declare capabilities in their embedded manifest. The
   permission grammar is exactly the one already implemented in
   `crates/pkm-plugin/src/permissions.rs`:
   `file:read`, `file:write`, `network`, `git`, `exec`, `all`. A host call is gated
   by the permission bound in the API spec table; a missing grant aborts the host
   callback and surfaces `PluginDenied` to the plugin. `network` is required by
   `http_request`, `file:read` by `note_read`, `file:write` by `note_write`.

7. **SSRF guard.** `http_request` is constrained to the vault's `network.allowlist`
   (from `pkm_core::Config`). Connections to any allow-listed host, or to a
   loopback/private address, are allowed; other targets return `HttpSsid`. Timeout
   is `HTTP_TIMEOUT` = 10 s (non-tunable in the contract). `note_read`/`note_write`
   are scoped by an explicit **vault-root** check that mirrors
   `resolve_safe_path`/`resolve_safe_write_path` in `src-tauri/src/commands/page.rs`:
   note paths are vault-relative, canonicalized, and rejected on traversal.

8. **Plugin packaging.** A plugin is shipped as a **single file**: a `plugin.wasm`
   core module with an **embedded** JSON manifest in the custom section named
   `stratum:manifest`. The legacy sidecar convention
   (`<entry>.wasm.manifest.json` next to the `.wasm`) remains supported **for
   reading** (both in `PluginRegistry::load_plugin` and in vault directory scans),
   but embedded manifests take precedence and are the canonical format. The
   `PluginConfig` struct in `pkm_core` remains the on-disk enablement/override
   mechanism; it must not be the source of the manifest schema.

9. **Plugin lifecycle.** States are `Discovered → Loading → Ready | Failed →
   (Enable/Disable)`, always externally visible through the documented Tauri
   command surface; a failed load records `PluginLoadError` and never returns a
   partial plugin. Hooks are dispatched per the hook table (`onSave`, `onOpen`,
   `onLink`, `onSearch`) plus arbitrary `on*` names. The runtime instantiates the
   module with **no** WASI context; the linker provides only the `pkm` module
   imports (plus the reserved `imports.wasi_snapshot_preview1` no-op for
   toolchain compatibility — see spec §7.1).

10. **Tauri command surface.** The five lifecycle commands and two host-test
    commands in the API spec (`list`, `enable`, `disable`, `reload`, `status`,
    `note_read`, `http_request`) are the only plugin commands the frontend should
    call. Their names, argument names, and DTO shapes are normative; the frontend
    task must use exactly these. The host fuzzing command `plugin_http_request`
    drives the *same* `http_request` backend used by plugins, so a plugin author can
    verify allowlist/timeout behavior without writing a plugin.

## Consequences

- Backend (`t_b197920d`, `t_e132158a`) implements against this contract; no later
  contract change is permitted inside E3 without a new ADR.
- Frontend (`t_ec520ca3`) builds the management UI against the named Tauri commands
  and DTO schemas; it may mock locally before integration lands.
- QA acceptance ("no stub behavior remains", "errors are visible/logged") maps
  directly to the error tables in the API spec.
- Contract stability is a release-blocking criterion: the `HostFunction` enum and
  the four import names are ABI-frozen for the v0.7.x series.

## Status of this ADR

Implemented and verified as of commit `e1af45f` (epic E3): the host functions
are real (see `crates/pkm-plugin/src/host.rs`), the crate is linked into and
wired through `src-tauri` (`PluginManager`, Tauri plugin commands, vault
startup scan), the management UI exists, and the contract is exercised
end-to-end by `crates/pkm-tests/tests/plugin_e2e.rs`. The `Context` section
above describes the state at decision time and is retained for the record.

## References

- API specification: `docs/advanced/plugins.md` (normative).
- Existing (stub) runtime: `crates/pkm-plugin/src/runtime.rs`.
- Existing permission module: `crates/pkm-plugin/src/permissions.rs`.
- Existing registry: `crates/pkm-plugin/src/registry.rs`.
- Path-safety reference implementations: `src-tauri/src/commands/page.rs`
  (`resolve_safe_path`, `resolve_safe_write_path`).
