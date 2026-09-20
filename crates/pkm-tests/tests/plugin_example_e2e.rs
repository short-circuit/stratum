//! End-to-end test that installs the *real* Rust example plugin
//! (`examples/plugins/titlecase/plugin.wasm`, built with
//! `cargo build --target wasm32-wasip1`) into a temp vault through the real
//! [`PluginManager`], then dispatches `onSave` and asserts the note on disk
//! has been title-cased.
//!
//! This is the acceptance proof for E3.F5 ("example plugin installs and runs
//! end-to-end"): the artifact under test is the actual wasm32-wasip1 Rust
//! binary shipped in the repo, which imports the real WASIp1 CRT
//! (fd_write, environ_get, proc_exit) *and* the `pkm.*` host imports — a
//! combination that requires the runtime's real WASI preview1 support.

use std::path::{Path, PathBuf};

use tempfile::TempDir;

/// Path to the committed example plugin artifact, resolved relative to this
/// file (crates/pkm-tests/tests/ -> repo root).
fn example_plugin_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent() // crates
        .unwrap()
        .parent() // repo root
        .unwrap()
        .join("examples")
        .join("plugins")
        .join("titlecase")
        .join("plugin.wasm")
}

/// Write a `.pkm/config.toml` enabling the plugin.
fn write_config(vault: &Path, plugin_id: &str) {
    let pkm_dir = vault.join(".pkm");
    std::fs::create_dir_all(&pkm_dir).unwrap();
    let config = format!(
        "\n[[plugins]]\nname = \"{plugin_id}\"\nenabled = true\nwasm_path = \".pkm/plugins/{plugin_id}/plugin.wasm\"\npermissions = [\"file:read\", \"file:write\"]\n"
    );
    std::fs::write(pkm_dir.join("config.toml"), config).unwrap();
}

#[test]
fn example_plugin_wasm32_wasi_runs_end_to_end() {
    use app_lib::commands::plugins::{dispatch_on_save, PluginManager};

    let wasm = example_plugin_path();
    assert!(
        wasm.is_file(),
        "example plugin artifact must exist — run `cargo build --release --target wasm32-wasip1` in examples/plugins/titlecase"
    );

    let tmp = TempDir::new().unwrap();
    let vault = tmp.path();
    let notes = vault.join("notes");
    std::fs::create_dir_all(&notes).unwrap();
    let note = notes.join("welcome.md");
    std::fs::write(&note, "a title for the ages\n\nand a body line.").unwrap();

    // Install the real artifact into the vault layout.
    let plugins_dir = vault.join(".pkm").join("plugins");
    std::fs::create_dir_all(&plugins_dir).unwrap();
    let dest = plugins_dir.join("com.example.titlecase");
    std::fs::create_dir_all(&dest).unwrap();
    std::fs::copy(&wasm, dest.join("plugin.wasm")).unwrap();
    write_config(vault, "com.example.titlecase");

    let manager = PluginManager::init_for_vault(vault).expect("init_for_vault");

    // It must be discovered, loaded, and enabled with the manifest's hooks.
    let list = manager.list();
    let plugin = list
        .plugins
        .iter()
        .find(|p| p.id == "com.example.titlecase")
        .unwrap_or_else(|| panic!("plugin not listed: {:?}", list.plugins));
    assert_eq!(plugin.status, "ready", "plugin must load: {plugin:?}");
    assert!(plugin.enabled);
    assert!(manager.has_hook("onSave"));

    // Dispatch onSave with the real payload shape the app sends (§8).
    let content = std::fs::read_to_string(&note).unwrap();
    dispatch_on_save(&manager, "notes/welcome.md", &content);

    // The plugin rewrote the note through the host API.
    let after = std::fs::read_to_string(&note).unwrap();
    assert_ne!(after, content, "note content must have been transformed");
    assert!(
        after.starts_with("A Title For The Ages"),
        "expected title-cased output, got: {after:?}"
    );
    assert!(after.contains("And A Body Line."), "got: {after:?}");
}

#[test]
fn example_plugin_compiles_and_installs_through_registry() {
    use app_lib::commands::plugins::PluginManager;

    let wasm = example_plugin_path();
    let tmp = TempDir::new().unwrap();
    let vault = tmp.path();
    let src = vault.join("staging.wasm");
    std::fs::copy(&wasm, &src).unwrap();

    let manager = PluginManager::init_for_vault(vault).expect("init_for_vault");
    // install() should fail fast if the module does not compile; this is the
    // real wasm32-wasip1 binary, so it must succeed and register.
    let info = manager
        .install(&src)
        .expect("install real wasm32-wasip1 plugin");
    assert_eq!(info.id, "com.example.titlecase");
    assert_eq!(info.status, "disabled", "fresh install starts disabled");

    // The registry can reload from the copied artifact and dispatch.
    let state = manager.status("com.example.titlecase").unwrap();
    assert_eq!(state.name, "Title Case");
}
