//! Plugin lifecycle commands and the per-vault [`PluginManager`].
//!
//! Implements the normative Tauri command surface from
//! `docs/advanced/plugins.md` §9 (ADR-0004): `plugins_list`, `plugins_enable`,
//! `plugins_disable`, `plugins_reload`, `plugins_status`, `plugin_note_read`,
//! `plugin_http_request`. Command names, argument names, and DTO shapes are
//! ABI-frozen for the v0.7.x series.
//!
//! The [`PluginManager`] owns the process-wide plugin runtime and the per-vault
//! registry. It lives in `VaultState.plugin_manager` as an `Arc` so long-running
//! host calls never hold the vault mutex.

use crate::commands::vault::{AppState, VaultState};
use pkm_plugin::{HostApi, PluginRegistry, PluginRuntime, PluginState, VaultHost};
use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tracing::{debug, info, warn};

// ---------------------------------------------------------------------------
// DTOs (normative — ADR-0004 §9)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    /// `"ready"` | `"disabled"` | `"error"`
    pub status: String,
    pub enabled: bool,
    pub permissions: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PluginListResult {
    pub plugins: Vec<PluginInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PluginNoteDto {
    pub path: String,
    pub content: String,
    pub mtime: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PluginHttpResponseDto {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl PluginInfo {
    fn from_state(state: &PluginState) -> Self {
        let status = if state.enabled { "ready" } else { "disabled" };
        Self {
            id: state.manifest.id.clone(),
            name: state.manifest.name.clone(),
            version: state.manifest.version.clone(),
            status: status.to_string(),
            enabled: state.enabled,
            permissions: state.manifest.permission_names(),
            error: None,
        }
    }

    fn from_failure(id: String, error: String) -> Self {
        let name = id.clone();
        Self {
            id,
            name,
            version: String::new(),
            status: "error".to_string(),
            enabled: false,
            permissions: Vec::new(),
            error: Some(error),
        }
    }
}

// ---------------------------------------------------------------------------
// Plugin manager
// ---------------------------------------------------------------------------

/// Owns the WASM runtime and the per-vault plugin registry.
///
/// This is `Send + Sync`: the runtime is `Send + Sync` and the registry is
/// guarded by a `Mutex`. The `Arc<PluginManager>` is stored in `VaultState` so
/// plugin work never blocks the vault mutex.
pub struct PluginManager {
    runtime: PluginRuntime,
    registry: Mutex<PluginRegistry>,
    vault_root: PathBuf,
}

impl PluginManager {
    /// Create a manager rooted at `vault_root`, loading plugins from
    /// `<vault_root>/.pkm/plugins/` (spec §7.3). `enabled_ids` is the set of
    /// plugin ids enabled in the vault config; absent ids are loaded disabled.
    pub fn new(
        vault_root: PathBuf,
        allowlist: Vec<String>,
        enabled_ids: HashSet<String>,
    ) -> Result<Arc<Self>, String> {
        let host = VaultHost::new(vault_root.clone(), allowlist);
        let runtime = PluginRuntime::with_host(Box::new(host)).map_err(|e| e.to_string())?;

        let mut registry = PluginRegistry::new();
        let plugins_dir = vault_root.join(".pkm").join("plugins");
        let scanned = registry.scan_vault(&plugins_dir, &enabled_ids);
        debug!(
            "[stratum] Scanned plugins dir {:?}: {scanned} loaded",
            plugins_dir
        );

        Ok(Arc::new(Self {
            runtime,
            registry: Mutex::new(registry),
            vault_root,
        }))
    }

    /// A manager with an empty registry (no plugins dir present).
    pub fn empty(vault_root: PathBuf) -> Result<Arc<Self>, String> {
        let host = VaultHost::root(vault_root.clone());
        let runtime = PluginRuntime::with_host(Box::new(host)).map_err(|e| e.to_string())?;
        Ok(Arc::new(Self {
            runtime,
            registry: Mutex::new(PluginRegistry::new()),
            vault_root,
        }))
    }

    /// Initialize a manager for `vault_root`, resolving the SSRF allowlist
    /// and the enabled plugin set from the vault config.
    ///
    /// This is the single entry point for both the interactive vault-open
    /// path (`setup_vault`) and the app startup path, so the spec §7.3
    /// startup scan is honored on boot — not only after an explicit
    /// re-init. A broken plugin is never fatal: `scan_vault` records the
    /// failure and surfaces it via `plugins_list` (spec §6.4).
    pub fn init_for_vault(vault_root: &std::path::Path) -> Result<Arc<Self>, String> {
        let allowlist = pkm_core::Config::load(vault_root.join(".pkm").join("config.toml"))
            .map(|c| c.network.allowlist)
            .unwrap_or_default();
        let enabled_ids = Self::enabled_ids_from_config(vault_root);
        Self::new(vault_root.to_path_buf(), allowlist, enabled_ids)
    }

    /// Number of plugins currently in the registry (used by startup logging).
    pub fn len(&self) -> usize {
        self.registry.lock().map(|r| r.list().len()).unwrap_or(0)
    }

    /// Whether the registry holds no plugins.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn vault_root(&self) -> &PathBuf {
        &self.vault_root
    }

    /// Snapshot of all plugin info for `plugins_list`.
    pub fn list(&self) -> PluginListResult {
        let reg = match self.registry.lock() {
            Ok(r) => r,
            Err(_) => {
                return PluginListResult {
                    plugins: Vec::new(),
                }
            }
        };
        let mut plugins: Vec<PluginInfo> = reg
            .list()
            .iter()
            .map(|s| PluginInfo::from_state(s))
            .collect();
        plugins.extend(
            reg.failed_ids()
                .into_iter()
                .map(|(id, err)| PluginInfo::from_failure(id, err)),
        );
        plugins.sort_by(|a, b| a.id.cmp(&b.id));
        PluginListResult { plugins }
    }

    /// Status for one plugin id, or `None` if unknown.
    pub fn status(&self, id: &str) -> Option<PluginInfo> {
        let reg = self.registry.lock().ok()?;
        if let Some(state) = reg.get(id) {
            Some(PluginInfo::from_state(state))
        } else {
            reg.failed_error(id)
                .map(|err| PluginInfo::from_failure(id.to_string(), err.to_string()))
        }
    }

    /// Enable a plugin by id. Re-loads from disk so hooks are re-armed
    /// (spec §9.2). Persists to the vault config.
    pub fn enable(&self, id: &str) -> Result<PluginInfo, String> {
        self.persist_enablement(id, true)?;
        let mut reg = self.registry.lock().map_err(|e| e.to_string())?;
        reg.enable(id)
            .map_err(|_| format!("plugin_not_found: no plugin with id `{id}` is loaded"))?;
        Ok(PluginInfo::from_state(
            reg.get(id)
                .ok_or_else(|| format!("plugin_not_found: `{id}`"))?,
        ))
    }

    /// Disable a plugin by id (hooks + dispatch skipped, state retained).
    pub fn disable(&self, id: &str) -> Result<PluginInfo, String> {
        self.persist_enablement(id, false)?;
        let mut reg = self.registry.lock().map_err(|e| e.to_string())?;
        if reg.get(id).is_none() {
            return Ok(PluginInfo::from_failure(
                id.to_string(),
                format!("plugin_not_found: no plugin with id `{id}` is loaded"),
            ));
        }
        reg.disable(id);
        Ok(PluginInfo::from_state(
            reg.get(id)
                .ok_or_else(|| format!("plugin_not_found: `{id}`"))?,
        ))
    }

    /// Re-instantiate a plugin from disk (re-read manifest, recompile).
    pub fn reload(&self, id: &str) -> Result<PluginInfo, String> {
        let plugins_dir = self.vault_root.join(".pkm").join("plugins");
        let plugin_dir = plugins_dir.join(id);
        let wasm_path = plugin_dir.join("plugin.wasm");
        if !wasm_path.exists() {
            return Err(format!("plugin_not_found: `{id}`"));
        }
        let mut reg = self.registry.lock().map_err(|e| e.to_string())?;
        match reg.load_plugin(&wasm_path) {
            Ok(_) => {
                // Re-load enables by default; re-apply current enablement.
                let enabled = reg.failed_error(id).is_none()
                    && reg.get(id).map(|s| s.enabled).unwrap_or(false);
                if let Some(state) = reg.get_mut(id) {
                    state.enabled = enabled || state.enabled;
                }
                reg.clear_failure(id);
                Ok(PluginInfo::from_state(
                    reg.get(id)
                        .ok_or_else(|| format!("plugin_not_found: `{id}`"))?,
                ))
            }
            Err(e) => {
                reg.record_failure(id.to_string(), e.to_string());
                Err(format!("plugin_load_error: {e}"))
            }
        }
    }

    /// Run an enabled plugin's hook with the given payload, logging failures
    /// without aborting the surrounding operation (spec §8). Returns the
    /// plugin's JSON response if any.
    pub fn dispatch_hook(&self, id: &str, hook: &str, payload: &str) -> Option<String> {
        let reg = self.registry.lock().ok()?;
        let state = reg.get(id)?.clone();
        if !state.enabled {
            return None;
        }
        let name = state.manifest.name.clone();
        drop(reg);
        match self.runtime.run_plugin(&state, hook, payload) {
            Ok(out) => {
                debug!(
                    "[stratum] plugin {name} hook {hook} returned {} chars",
                    out.len()
                );
                Some(out)
            }
            Err(e) => {
                warn!("[stratum] plugin {name} hook {hook} failed: {e}");
                None
            }
        }
    }

    /// Dispatch a hook to every enabled plugin that declares it.
    pub fn dispatch_all(&self, hook: &str, payload: &str) -> Vec<(String, Option<String>)> {
        let reg = match self.registry.lock() {
            Ok(r) => r,
            Err(_) => return Vec::new(),
        };
        let targets: Vec<(String, PluginState)> = reg
            .list_enabled()
            .iter()
            .filter(|s| s.manifest.hook_enabled(hook))
            .map(|s| (s.manifest.id.clone(), (*s).clone()))
            .collect();
        drop(reg);
        targets
            .into_iter()
            .map(|(id, _)| {
                let out = self.dispatch_hook(&id, hook, payload);
                (id, out)
            })
            .collect()
    }

    /// True if any enabled plugin declares this hook.
    pub fn has_hook(&self, hook: &str) -> bool {
        let Ok(reg) = self.registry.lock() else {
            return false;
        };
        reg.list_enabled()
            .iter()
            .any(|s| s.manifest.hook_enabled(hook))
    }

    /// Persist enablement state for a plugin id to the vault config.toml
    /// (`plugins` list). The enabled set is written as the full list of
    /// currently enabled plugin ids.
    fn persist_enablement(&self, id: &str, enabled: bool) -> Result<(), String> {
        let config_path = self.vault_root.join(".pkm").join("config.toml");
        if !config_path.exists() {
            // No config yet — plugin enablement is in-memory only.
            return Ok(());
        }
        let mut config = pkm_core::Config::load(&config_path).map_err(|e| e.to_string())?;
        if enabled {
            if !config.plugins.iter().any(|p| p.name == id) {
                config.plugins.push(pkm_core::config::PluginConfig {
                    name: id.to_string(),
                    enabled: true,
                    wasm_path: self
                        .vault_root
                        .join(".pkm")
                        .join("plugins")
                        .join(id)
                        .join("plugin.wasm"),
                    permissions: Vec::new(),
                });
            }
        } else {
            config.plugins.retain(|p| p.name != id);
            config.plugins.push(pkm_core::config::PluginConfig {
                name: id.to_string(),
                enabled: false,
                wasm_path: self
                    .vault_root
                    .join(".pkm")
                    .join("plugins")
                    .join(id)
                    .join("plugin.wasm"),
                permissions: Vec::new(),
            });
        }
        config.save(&config_path).map_err(|e| e.to_string())?;
        info!("[stratum] Persisted plugin `{id}` enabled={enabled}");
        Ok(())
    }

    /// The set of plugin ids currently enabled in the vault config.
    /// Returns empty if no config exists.
    pub fn enabled_ids_from_config(vault_root: &Path) -> HashSet<String> {
        let config_path = vault_root.join(".pkm").join("config.toml");
        let Ok(config) = pkm_core::Config::load(&config_path) else {
            return HashSet::new();
        };
        config
            .plugins
            .iter()
            .filter(|p| p.enabled)
            .map(|p| p.name.clone())
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

fn get_manager(state: &VaultState) -> Result<Arc<PluginManager>, String> {
    state
        .plugin_manager
        .clone()
        .ok_or_else(|| "no vault is open — plugin runtime not initialized".to_string())
}

/// §9.1 — List installed plugins with status. No vault open ⇒ empty list.
#[tauri::command]
pub async fn plugins_list(state: tauri::State<'_, AppState>) -> Result<PluginListResult, String> {
    let locked = state.lock().map_err(|e| e.to_string())?;
    let Some(manager) = locked.plugin_manager.clone() else {
        return Ok(PluginListResult {
            plugins: Vec::new(),
        });
    };
    drop(locked);
    Ok(manager.list())
}

/// §9.2 — Enable a disabled plugin and re-arm hooks.
#[tauri::command]
pub async fn plugins_enable(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<PluginInfo, String> {
    let locked = state.lock().map_err(|e| e.to_string())?;
    let manager = get_manager(&locked)?;
    drop(locked);
    manager.enable(&id)
}

/// §9.3 — Disable a running plugin.
#[tauri::command]
pub async fn plugins_disable(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<PluginInfo, String> {
    let locked = state.lock().map_err(|e| e.to_string())?;
    let manager = get_manager(&locked)?;
    drop(locked);
    manager.disable(&id)
}

/// §9.4 — Re-instantiate a plugin from disk.
#[tauri::command]
pub async fn plugins_reload(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<PluginInfo, String> {
    let locked = state.lock().map_err(|e| e.to_string())?;
    let manager = get_manager(&locked)?;
    drop(locked);
    manager.reload(&id)
}

/// §9.5 — Full status for one plugin.
#[tauri::command]
pub async fn plugins_status(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<PluginInfo, String> {
    let locked = state.lock().map_err(|e| e.to_string())?;
    let manager = get_manager(&locked)?;
    drop(locked);
    manager
        .status(&id)
        .ok_or_else(|| format!("plugin_not_found: `{id}`"))
}

/// §9.6 — Run the same `note_read` backend used by plugins, without a plugin.
#[tauri::command]
pub async fn plugin_note_read(
    path: String,
    state: tauri::State<'_, AppState>,
) -> Result<PluginNoteDto, String> {
    let root = {
        let locked = state.lock().map_err(|e| e.to_string())?;
        let manager = get_manager(&locked)?;
        manager.vault_root().clone()
    };

    let host = VaultHost::root(root);
    let note = host.read_note(&path).await.map_err(|code| match code {
        pkm_plugin::PluginErrorCode::NoteNotFound => {
            format!("note_not_found: `{path}`")
        }
        _ => format!("plugin_runtime_error: {code:?}"),
    })?;
    Ok(PluginNoteDto {
        path: note.path,
        content: note.content,
        mtime: note.mtime,
    })
}

/// §9.7 — Run the same `http_request` backend used by plugins, without a plugin.
#[tauri::command]
pub async fn plugin_http_request(
    method: Option<String>,
    url: String,
    headers: Option<Vec<(String, String)>>,
    body: Option<String>,
    timeout_ms: Option<u64>,
    state: tauri::State<'_, AppState>,
) -> Result<PluginHttpResponseDto, String> {
    let root = {
        let locked = state.lock().map_err(|e| e.to_string())?;
        let manager = get_manager(&locked)?;
        manager.vault_root().clone()
    };

    let headers_map = headers
        .map(|pairs| {
            pairs
                .into_iter()
                .map(|(k, v)| (k, serde_json::Value::String(v)))
                .collect::<serde_json::Map<String, serde_json::Value>>()
        })
        .unwrap_or_default();
    let req = pkm_plugin::to_host_request(method, &url, Some(headers_map), body, timeout_ms)
        .map_err(|code| format!("invalid_argument: {code:?}"))?;
    // The host-test path is not gated by plugin permissions; use an empty
    // allowlist (private/loopback default) for the same SSRF behavior.
    let host = VaultHost::root(root);
    let resp = host
        .http_request(req)
        .await
        .map_err(|code| format!("{code:?}"))?;
    Ok(PluginHttpResponseDto {
        status: resp.status,
        headers: resp.headers,
        body: resp.body,
    })
}

// ---------------------------------------------------------------------------
// Hook dispatch helpers (spec §8) — called from page/backlink/search flows
// ---------------------------------------------------------------------------

/// Dispatch `onSave` to all enabled plugins that declare it. Payload is
/// `{"path": ..., "content": ...}`.
pub fn dispatch_on_save(manager: &PluginManager, path: &str, content: &str) {
    if !manager.has_hook("onSave") {
        return;
    }
    let payload = serde_json::json!({ "path": path, "content": content });
    manager.dispatch_all("onSave", &payload.to_string());
}

/// Dispatch `onOpen` to all enabled plugins that declare it.
pub fn dispatch_on_open(manager: &PluginManager, path: &str) {
    if !manager.has_hook("onOpen") {
        return;
    }
    let payload = serde_json::json!({ "path": path });
    manager.dispatch_all("onOpen", &payload.to_string());
}

/// Addressable handle for hook dispatch from other command modules.
pub fn dispatch_hook(manager: &PluginManager, hook: &str, payload: &str) {
    manager.dispatch_all(hook, payload);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal valid WASM module (empty, no exports) — sufficient for
    /// `scan_vault` to load/register a plugin. Manifest comes from the sidecar.
    const EMPTY_MODULE: &[u8] = &[0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];

    /// Sidecar manifest for a plugin with `file:read` + `network` and an
    /// `onSave` hook. Placed as `<id>.wasm.manifest.json` beside the wasm.
    const SIDECAR_MANIFEST: &str = r#"{
        "schema_version": 1,
        "id": "com.example.scan-test",
        "name": "Scan Test",
        "version": "0.1.0",
        "description": "integration test",
        "author": "test",
        "entry": "scan-test.wasm",
        "permissions": ["file:read", "network"],
        "hooks": { "onSave": true }
    }"#;

    /// Build `<vault>/.pkm/plugins/<id>/` containing a wasm module and its
    /// sidecar manifest, mirroring the legacy layout `scan_vault` accepts.
    /// Returns the plugin directory, or panics.
    fn write_plugin_dir(
        vault: &std::path::Path,
        wasm_bytes: &[u8],
        manifest: &str,
    ) -> std::path::PathBuf {
        let plugin_dir = vault.join(".pkm").join("plugins").join("scan-test");
        std::fs::create_dir_all(&plugin_dir).expect("mkdir plugin dir");
        std::fs::write(plugin_dir.join("scan-test.wasm"), wasm_bytes).expect("write wasm");
        std::fs::write(plugin_dir.join("scan-test.wasm.manifest.json"), manifest)
            .expect("write manifest");
        plugin_dir
    }

    /// Write `<vault>/.pkm/config.toml` with the plugin enabled. Enablement
    /// keys off the manifest `id` via the `[[plugins]]` list (§7.3).
    fn write_enabling_config(vault: &std::path::Path) {
        let pkm_dir = vault.join(".pkm");
        std::fs::create_dir_all(&pkm_dir).expect("mkdir .pkm");
        let toml = r#"
[[plugins]]
name = "com.example.scan-test"
enabled = true
wasm_path = ".pkm/plugins/scan-test/scan-test.wasm"
permissions = ["file:read", "network"]
"#;
        std::fs::write(pkm_dir.join("config.toml"), toml).expect("write config");
    }

    #[test]
    fn init_for_vault_scans_plugins_dir_and_loads_plugins() {
        let vault = tempfile::tempdir().expect("tempdir");
        write_plugin_dir(vault.path(), EMPTY_MODULE, SIDECAR_MANIFEST);
        write_enabling_config(vault.path());

        let manager = PluginManager::init_for_vault(vault.path()).expect("init_for_vault");
        assert_eq!(manager.len(), 1, "scan must register one plugin");
        assert!(!manager.is_empty());

        let info = manager
            .status("com.example.scan-test")
            .expect("plugin must be visible after scan");
        assert_eq!(info.status, "ready", "enabled plugin must load ready");
        assert!(info.enabled);
        assert_eq!(info.name, "Scan Test");
        let mut perms = info.permissions.clone();
        let mut want = vec!["file:read".to_string(), "network".to_string()];
        perms.sort();
        want.sort();
        assert_eq!(perms, want, "permission set must be surfaced");
    }

    #[test]
    fn init_for_vault_scanned_plugin_defaults_to_disabled() {
        // A plugin present on disk but absent from the config `[[plugins]]`
        // enable list is loaded but disabled (spec §7.3).
        let vault = tempfile::tempdir().expect("tempdir");
        write_plugin_dir(vault.path(), EMPTY_MODULE, SIDECAR_MANIFEST);

        let manager = PluginManager::init_for_vault(vault.path()).expect("init_for_vault");
        let info = manager
            .status("com.example.scan-test")
            .expect("plugin must be visible after scan");
        assert_eq!(info.status, "disabled");
        assert!(!info.enabled);
    }

    #[test]
    fn init_for_vault_with_no_plugins_dir_is_empty() {
        // A vault with no `.pkm/plugins/` directory must not fail or panic.
        let vault = tempfile::tempdir().expect("tempdir");
        let manager = PluginManager::init_for_vault(vault.path()).expect("init_for_vault");
        assert!(manager.is_empty());
        assert_eq!(manager.len(), 0);
        assert!(manager.list().plugins.is_empty());
    }

    #[test]
    fn init_for_vault_records_broken_plugin_as_error() {
        let vault = tempfile::tempdir().expect("tempdir");
        // A wasm that is not a valid module — the plugin must not abort the
        // scan; it surfaces as status "error". The failure is keyed by the
        // on-disk directory name (scan_vault cannot read an id from a module
        // that fails to load).
        let _plugin_dir =
            write_plugin_dir(vault.path(), b"not a wasm module at all", SIDECAR_MANIFEST);

        let manager = PluginManager::init_for_vault(vault.path()).expect("init_for_vault");
        let info = manager
            .status("scan-test")
            .expect("failed plugin still visible");
        assert_eq!(info.status, "error");
        assert!(info.error.is_some(), "error detail must be surfaced");
    }
}
