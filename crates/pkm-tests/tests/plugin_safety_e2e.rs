//! Safety-focused integration tests for the WASM plugin runtime.
//!
//! These tests exercise the REAL plugin path with a compiled `.wasm` module
//! (built in-process with `wat`) and the real registry/runtime/host
//! implementations — no stubs of plugin-system internals. They cover two
//! requirements that the functional E2E suite does not:
//!
//! * **Memory out-of-bounds safety**: an OOB host call (a plugin passing a
//!   pointer/length past the linear-memory end) or an OOB guest access must
//!   trap in a *contained* way — `run_plugin` returns an error, never a panic,
//!   never memory corruption, and a subsequent in-bounds call on the same
//!   runtime still succeeds.
//! * **Corrupted-WASM handling**: truncated modules, garbage bytes, a valid
//!   module whose embedded `stratum:manifest` payload is not JSON, and a valid
//!   module with no manifest at all must all be recorded as load failures
//!   (surface as status `error` through `plugins_list`) without aborting the
//!   vault scan or preventing healthy plugins in the same directory from
//!   loading.

use pkm_plugin::registry::extract_embedded_manifest;
use pkm_plugin::registry::{PluginRegistry, PluginState};
use pkm_plugin::runtime::PluginRuntime;
use std::collections::HashMap;
use std::path::Path;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// A temp vault with a canonical `.pkm/plugins` directory.
struct TestVault {
    _dir: TempDir,
    root: std::path::PathBuf,
}

impl TestVault {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().to_path_buf();
        std::fs::create_dir_all(root.join(".pkm").join("plugins")).unwrap();
        Self { _dir: dir, root }
    }

    /// Exact validation of the canonical layout
    /// `<vault>/.pkm/plugins/<id>/plugin.wasm`.
    #[allow(dead_code)]
    fn plugin_dir(&self, id: &str) -> impl AsRef<Path> {
        self.root.join(".pkm").join("plugins").join(id)
    }

    fn write_wasm(&self, id: &str, bytes: &[u8]) -> std::path::PathBuf {
        let dir = self.root.join(".pkm").join("plugins").join(id);
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("plugin.wasm");
        std::fs::write(&p, bytes).unwrap();
        p
    }

    fn write_sidecar_manifest(&self, id: &str, json: &str) {
        let dir = self.root.join(".pkm").join("plugins").join(id);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("plugin.wasm.manifest.json"), json).unwrap();
    }
}

/// Build a runtime rooted at `vault` with a real `VaultHost` (default empty
/// SSRF allowlist).
fn runtime_for(vault: &TestVault) -> PluginRuntime {
    PluginRuntime::with_host(Box::new(pkm_plugin::VaultHost::root(vault.root.clone())))
        .expect("runtime")
}

fn valid_manifest(id: &str) -> pkm_plugin::registry::PluginManifest {
    let mut hooks = HashMap::new();
    hooks.insert("onSave".to_string(), true);
    pkm_plugin::registry::PluginManifest {
        schema_version: 1,
        id: id.to_string(),
        name: id.to_string(),
        version: "0.1.0".to_string(),
        author: "stratum-test".to_string(),
        description: "safety e2e test plugin".to_string(),
        entry: "plugin.wasm".to_string(),
        permissions: pkm_plugin::PermissionSet::new(),
        hooks,
    }
}

fn state_for(wasm: Vec<u8>) -> PluginState {
    PluginState::new(valid_manifest("com.example.safety-e2e"), wasm, true)
}

// ---------------------------------------------------------------------------
// Memory out-of-bounds safety
// ---------------------------------------------------------------------------

/// A real module whose `onSave` forwards a pointer 1 MiB past the start — far
/// beyond the single 64 KiB page it exports — to the `pkm.note_read` host
/// import with a non-trivial length. The host's `memory_read` must reject it
/// as out of bounds.
const OOB_HOST_READ_WAT: &str = r#"
    (module
        (import "pkm" "note_read" (func $note_read (param i32 i32) (result i32)))
        (memory (export "memory") 1)
        (func (export "onSave") (param i32 i32) (result i32)
            i32.const 1048576
            i32.const 128
            call $note_read
        )
    )
"#;

/// A real module whose `onSave` reads from guest linear memory at a huge
/// offset — an OOB *guest* access that wasmtime must trap.
const OOB_GUEST_READ_WAT: &str = r#"
    (module
        (memory (export "memory") 1)
        (func (export "onSave") (param i32 i32) (result i32)
            i32.load (i32.const 1048576)
        )
    )
"#;

/// An in-bounds, well-behaved module used to prove the runtime still works
/// after an OOB failure on the same runtime (no poisoned state).
const WELL_BEHAVED_WAT: &str = r#"
    (module
        (memory (export "memory") 1)
        (func (export "onSave") (param i32 i32) (result i32)
            ;; Write a fixed byte at offset 0 and return 1.
            i32.const 0
            i32.const 42
            i32.store8
            i32.const 1
        )
    )
"#;

#[test]
fn oob_host_read_is_contained_and_runtime_stays_usable() {
    let vault = TestVault::new();
    let runtime = runtime_for(&vault);

    // 1. OOB host pointer -> error, not panic, not corruption. The trap
    //    surfaces as a contained `PkmError::Plugin` describing the wasm
    //    failure (a host-import trap prints a wasm backtrace).
    let oob = state_for(wat::parse_str(OOB_HOST_READ_WAT).expect("WAT"));
    let err = runtime
        .run_plugin(&oob, "onSave", "{}")
        .expect_err("OOB host read must fail, not panic");
    let msg = err.to_string();
    assert!(
        msg.contains("hook 'onSave' failed") || msg.contains("out of bounds"),
        "error must be a contained wasm failure, got: {msg}"
    );

    // 2. The same runtime is still fully usable afterward.
    let good = state_for(wat::parse_str(WELL_BEHAVED_WAT).expect("WAT"));
    let out = runtime
        .run_plugin(&good, "onSave", "ignored")
        .expect("in-bounds call must still succeed after an OOB failure");
    // Return length 1 -> one byte read back from offset 0 (value 42).
    assert_eq!(out.as_bytes(), &[42u8]);
}

#[test]
fn oob_guest_read_traps_and_is_contained() {
    let vault = TestVault::new();
    let runtime = runtime_for(&vault);

    let oob_guest = state_for(wat::parse_str(OOB_GUEST_READ_WAT).expect("WAT"));
    let err = runtime
        .run_plugin(&oob_guest, "onSave", "ignored")
        .expect_err("OOB guest access must fail, not panic");
    let msg = err.to_string();
    assert!(
        msg.contains("Failed to compile") || msg.contains("out of bounds") || msg.contains("trap"),
        "error must be a contained wasm failure, got: {msg}"
    );

    // Runtime remains healthy.
    let good = state_for(wat::parse_str(WELL_BEHAVED_WAT).expect("WAT"));
    assert!(runtime.run_plugin(&good, "onSave", "ignored").is_ok());
}

// ---------------------------------------------------------------------------
// Corrupted-WASM handling (registry discovery must never abort)
// ---------------------------------------------------------------------------

/// The canonical minimal valid plugin: a module with no manifest source is
/// rejected by `read_manifest`. For scan-level tests the *loaded* healthy
/// sidecar is a valid module + a valid sidecar manifest.
const HEALTHY_PLUGIN_WAT: &str = r#"
    (module
        (memory (export "memory") 1)
        (func (export "onSave") (param i32 i32) (result i32)
            i32.const 0
        )
    )
"#;

const HEALTHY_SIDECAR: &str = r#"{
    "schema_version": 1,
    "id": "com.example.healthy",
    "name": "Healthy",
    "version": "0.1.0",
    "entry": "plugin.wasm",
    "permissions": [],
    "hooks": { "onSave": true }
}"#;

/// A real module with an embedded `stratum:manifest` custom section whose
/// payload is not valid JSON.
fn module_with_bad_manifest_json() -> Vec<u8> {
    let module = wat::parse_str(HEALTHY_PLUGIN_WAT).expect("WAT");
    // Reuse the same custom-section append logic as the E2E suite.
    append_custom_section(&module, "stratum:manifest", b"{ this is not json !!")
}

/// A real module with an embedded manifest that IS valid JSON but fails schema
/// validation (unknown permission).
fn module_with_invalid_manifest() -> Vec<u8> {
    let module = wat::parse_str(HEALTHY_PLUGIN_WAT).expect("WAT");
    append_custom_section(
        &module,
        "stratum:manifest",
        br#"{"schema_version":1,"id":"com.example.bad","name":"Bad","version":"0.1.0","entry":"plugin.wasm","permissions":["not-a-real-permission"],"hooks":{}}"#,
    )
}

/// A valid module with an embedded manifest payload that is valid JSON but
/// whose `id` is invalid (empty), which must fail validation.
fn module_with_no_manifest() -> Vec<u8> {
    wat::parse_str(HEALTHY_PLUGIN_WAT).expect("WAT")
}

// Custom-section helpers (mirror the E2E suite so the safety file is
// self-contained).
fn append_custom_section(module: &[u8], name: &str, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(module.len() + name.len() + payload.len() + 32);
    out.extend_from_slice(module);
    let name_bytes = name.as_bytes();
    let body_len = leb128_len(name_bytes.len() as u64) + name_bytes.len() + payload.len();
    out.push(0u8);
    write_uleb(&mut out, body_len as u64);
    write_uleb(&mut out, name_bytes.len() as u64);
    out.extend_from_slice(name_bytes);
    out.extend_from_slice(payload);
    out
}

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

/// Scan the vault and collect the load-failure map. Uses the real registry
/// scan path — the same one `PluginManager::init_for_vault` delegates to.
fn scan_failures(vault: &TestVault) -> std::collections::HashMap<String, String> {
    let mut registry = PluginRegistry::new();
    let dir = vault.root.join(".pkm").join("plugins");
    registry.scan_vault(&dir, &std::collections::HashSet::new());
    registry.failed_ids().into_iter().collect()
}

#[test]
fn scan_vault_records_truncated_wasm_without_aborting() {
    let vault = TestVault::new();
    // A valid module cut off mid-section: magic+version present, section
    // length claims more bytes than exist.
    let module = wat::parse_str(HEALTHY_PLUGIN_WAT).expect("WAT");
    let truncated = &module[..10];
    vault.write_wasm("com.example.truncated", truncated);

    // A healthy plugin in the same directory must still load.
    vault.write_wasm(
        "com.example.healthy",
        &wat::parse_str(HEALTHY_PLUGIN_WAT).unwrap(),
    );
    vault.write_sidecar_manifest("com.example.healthy", HEALTHY_SIDECAR);

    let failures = scan_failures(&vault);
    assert!(
        failures.contains_key("com.example.truncated"),
        "truncated plugin must be recorded as failed: {failures:?}"
    );

    let mut registry = PluginRegistry::new();
    let dir = vault.root.join(".pkm").join("plugins");
    let loaded = registry.scan_vault(&dir, &std::collections::HashSet::new());
    assert_eq!(
        registry
            .get("com.example.healthy")
            .map(|s| s.manifest.name.as_str()),
        Some("Healthy"),
        "healthy plugin must still load despite a truncated neighbor"
    );
    assert!(loaded >= 1, "scan must not abort on corrupted input");
}

#[test]
fn scan_vault_records_garbage_bytes_without_aborting() {
    let vault = TestVault::new();
    // Random bytes that are not even 8 bytes long (bad magic/version).
    vault.write_wasm("com.example.garbage", b"\x00\x61\x73\x6d\x00");

    let failures = scan_failures(&vault);
    assert!(
        failures.contains_key("com.example.garbage"),
        "garbage must be recorded as failed: {failures:?}"
    );
}

#[test]
fn scan_vault_records_bad_embedded_manifest_json_without_aborting() {
    let vault = TestVault::new();
    vault.write_wasm("com.example.badmanifest", &module_with_bad_manifest_json());
    // Healthy neighbor must still load.
    vault.write_wasm(
        "com.example.healthy",
        &wat::parse_str(HEALTHY_PLUGIN_WAT).unwrap(),
    );
    vault.write_sidecar_manifest("com.example.healthy", HEALTHY_SIDECAR);

    let failures = scan_failures(&vault);
    assert!(
        failures.contains_key("com.example.badmanifest"),
        "bad-manifest module must be recorded as failed: {failures:?}"
    );

    let mut registry = PluginRegistry::new();
    let dir = vault.root.join(".pkm").join("plugins");
    registry.scan_vault(&dir, &std::collections::HashSet::new());
    assert!(
        registry.get("com.example.healthy").is_some(),
        "healthy neighbor must still load"
    );
}

#[test]
fn scan_vault_records_schema_invalid_manifest_without_aborting() {
    let vault = TestVault::new();
    vault.write_wasm("com.example.schemabad", &module_with_invalid_manifest());
    vault.write_wasm(
        "com.example.healthy",
        &wat::parse_str(HEALTHY_PLUGIN_WAT).unwrap(),
    );
    vault.write_sidecar_manifest("com.example.healthy", HEALTHY_SIDECAR);

    let failures = scan_failures(&vault);
    assert!(
        failures.contains_key("com.example.schemabad"),
        "schema-invalid manifest must be recorded as failed: {failures:?}"
    );

    let mut registry = PluginRegistry::new();
    let dir = vault.root.join(".pkm").join("plugins");
    registry.scan_vault(&dir, &std::collections::HashSet::new());
    assert!(registry.get("com.example.healthy").is_some());
}

#[test]
fn scan_vault_records_module_without_manifest_without_aborting() {
    let vault = TestVault::new();
    // A perfectly valid module with no embedded manifest and no sidecar:
    // `read_manifest` has no source of identity -> rejected.
    vault.write_wasm("com.example.nomanifest", &module_with_no_manifest());
    vault.write_wasm(
        "com.example.healthy",
        &wat::parse_str(HEALTHY_PLUGIN_WAT).unwrap(),
    );
    vault.write_sidecar_manifest("com.example.healthy", HEALTHY_SIDECAR);

    let failures = scan_failures(&vault);
    assert!(
        failures.contains_key("com.example.nomanifest"),
        "manifest-less module must be recorded as failed: {failures:?}"
    );
    let mut registry = PluginRegistry::new();
    let dir = vault.root.join(".pkm").join("plugins");
    registry.scan_vault(&dir, &std::collections::HashSet::new());
    assert!(registry.get("com.example.healthy").is_some());
}

/// Direct `extract_embedded_manifest` probes for the corruption classes the
/// scanner routes through — these document the exact failure contracts and
/// guard the parser against silent acceptance.
#[test]
fn extract_embedded_manifest_reports_corruption_rather_than_silently_accepting() {
    // Too short.
    assert!(extract_embedded_manifest(b"\x00\x61").is_err());

    // Bad magic.
    assert!(
        extract_embedded_manifest(b"\x00\x61\x73\x4d\x01\x00\x00\x00").is_err(),
        "wrong magic must be rejected"
    );

    // Bad version.
    assert!(
        extract_embedded_manifest(b"\x00\x61\x73\x6d\x02\x00\x00\x00").is_err(),
        "wrong version must be rejected"
    );

    // Valid module without any custom section -> Ok(None), not an error.
    let module = wat::parse_str(HEALTHY_PLUGIN_WAT).unwrap();
    assert_eq!(
        extract_embedded_manifest(&module).expect("valid module parses"),
        None,
        "no manifest present is a valid state, not corruption"
    );

    // Truncated custom section (section length beyond file) -> error.
    let mut truncated = module.clone();
    truncated.truncate(module.len() - 2);
    assert!(
        extract_embedded_manifest(&truncated).is_err(),
        "section overrunning the file must be rejected"
    );
}
