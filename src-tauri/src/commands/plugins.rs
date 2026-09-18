//! Plugin lifecycle commands and the per-vault [`PluginManager`].
//!
//! Implements the normative Tauri command surface from
//! `docs/advanced/plugins.md` §9 (ADR-0004): `plugins_list`, `plugins_enable`,
//! `plugins_disable`, `plugins_reload`, `plugins_status`, `plugin_note_read`,
//! `plugin_http_request`, plus the F-series lifecycle commands `plugins_install`
//! and `plugins_uninstall`. Command names, argument names, and DTO shapes are
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

    /// Install a plugin from a WASM file path into the vault plugin directory
    /// (`<vault>/.pkm/plugins/<id>/`) and register it in the registry.
    ///
    /// The source may be a canonical `.wasm` (embedded `stratum:manifest`) or a
    /// bare `.wasm` with a sibling `.wasm.manifest.json` sidecar. The installation
    /// fails fast if the module does not compile or the manifest is invalid.
    ///
    /// A freshly installed plugin is **disabled** by default (consistent with
    /// spec §7.3: plugins absent from the config enable list are loaded
    /// disabled). Re-installing an id that is already enabled in the config
    /// keeps it enabled (upgrade path).
    pub fn install(&self, src_wasm: &Path) -> Result<PluginInfo, String> {
        if !src_wasm.is_file() {
            return Err(format!(
                "plugin_not_found: source WASM not found at {}",
                src_wasm.display()
            ));
        }
        let plugins_dir = self.vault_root.join(".pkm").join("plugins");
        std::fs::create_dir_all(&plugins_dir).map_err(|e| e.to_string())?;

        let mut reg = self.registry.lock().map_err(|e| e.to_string())?;
        let manifest = reg
            .install_from_path(src_wasm, &plugins_dir)
            .map_err(|e| format!("plugin_load_error: {e}"))?;
        let id = manifest.id.clone();

        // Fail fast on a module that does not compile.
        let state = reg
            .get(&id)
            .ok_or_else(|| format!("plugin_load_error: install did not register `{id}`"))?
            .clone();
        self.runtime
            .compile(&state.wasm_bytes)
            .map_err(|e| format!("plugin_load_error: {e}"))?;

        // Persist install state to config. A fresh install is disabled unless
        // the id is already present in the config enable list (upgrade path).
        let enabled = Self::enabled_ids_from_config(&self.vault_root).contains(&id);
        if let Some(s) = reg.get_mut(&id) {
            s.enabled = enabled;
        }
        drop(reg);
        self.persist_installed(&id, enabled)?;

        info!("[stratum] Installed plugin `{id}` (enabled={enabled})");
        self.status(&id)
            .ok_or_else(|| format!("plugin_load_error: `{id}` not visible after install"))
    }

    /// Uninstall a plugin by id: unload it from the registry, clear any
    /// recorded failure, remove its persisted directory, and remove the
    /// plugin from the vault config.
    pub fn uninstall(&self, id: &str) -> Result<PluginListResult, String> {
        let plugins_dir = self.vault_root.join(".pkm").join("plugins");
        let mut reg = self.registry.lock().map_err(|e| e.to_string())?;
        reg.uninstall(id, &plugins_dir)
            .map_err(|e| format!("plugin_uninstall_error: {e}"))?;
        drop(reg);
        self.remove_from_config(id)?;
        info!("[stratum] Uninstalled plugin `{id}`");
        Ok(self.list())
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

    /// Record an installation in the vault config so a subsequent scan sees the
    /// plugin (as enabled or disabled). Creates the config file if absent.
    fn persist_installed(&self, id: &str, enabled: bool) -> Result<(), String> {
        let config_path = self.vault_root.join(".pkm").join("config.toml");
        let mut config = match pkm_core::Config::load(&config_path) {
            Ok(c) => c,
            Err(_) if !config_path.exists() => pkm_core::Config::default(),
            Err(e) => return Err(e.to_string()),
        };
        // Replace any existing entry (keeps a single row per id).
        config.plugins.retain(|p| p.name != id);
        config.plugins.push(pkm_core::config::PluginConfig {
            name: id.to_string(),
            enabled,
            wasm_path: self
                .vault_root
                .join(".pkm")
                .join("plugins")
                .join(id)
                .join("plugin.wasm"),
            permissions: Vec::new(),
        });
        config.save(&config_path).map_err(|e| e.to_string())?;
        info!("[stratum] Persisted installed plugin `{id}` enabled={enabled}");
        Ok(())
    }

    /// Remove a plugin from the vault config entirely (uninstall path).
    fn remove_from_config(&self, id: &str) -> Result<(), String> {
        let config_path = self.vault_root.join(".pkm").join("config.toml");
        if !config_path.exists() {
            return Ok(());
        }
        let mut config = pkm_core::Config::load(&config_path).map_err(|e| e.to_string())?;
        let before = config.plugins.len();
        config.plugins.retain(|p| p.name != id);
        if config.plugins.len() != before {
            config.save(&config_path).map_err(|e| e.to_string())?;
        }
        info!("[stratum] Removed plugin `{id}` from config");
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

/// §9.8 (F-series) — Install a plugin from a WASM file path.
///
/// Copies the source (with an optional sibling `.wasm.manifest.json`) into
/// `<vault>/.pkm/plugins/<id>/` and registers it. The path is validated and
/// the module must compile; otherwise `plugin_load_error` is returned. A new
/// install is disabled by default; re-installing an id already enabled in the
/// config keeps it enabled.
#[tauri::command]
pub async fn plugins_install(
    path: String,
    state: tauri::State<'_, AppState>,
) -> Result<PluginInfo, String> {
    let locked = state.lock().map_err(|e| e.to_string())?;
    let manager = get_manager(&locked)?;
    drop(locked);
    manager.install(std::path::Path::new(&path))
}

/// §9.9 (F-series) — Uninstall a plugin by id.
///
/// Unloads the plugin from the registry, removes its directory and config
/// entry, and returns the updated plugin list.
#[tauri::command]
pub async fn plugins_uninstall(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<PluginListResult, String> {
    let locked = state.lock().map_err(|e| e.to_string())?;
    let manager = get_manager(&locked)?;
    drop(locked);
    manager.uninstall(&id)
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

/// Dispatch `onOpen` to all enabled plugins that declare it. Payload is
/// `{"path": "…"}` (spec §8). Returns the per-plugin dispatch results.
pub fn dispatch_on_open(manager: &PluginManager, path: &str) -> Vec<(String, Option<String>)> {
    if !manager.has_hook("onOpen") {
        return Vec::new();
    }
    let payload = serde_json::json!({ "path": path });
    manager.dispatch_all("onOpen", &payload.to_string())
}

/// Dispatch `onLink` to all enabled plugins that declare it. `links` is the
/// list of wiki-link targets extracted from the saved note content. Payload is
/// `{"path": "…", "links": ["…"]}` (spec §8).
pub fn dispatch_on_link(
    manager: &PluginManager,
    path: &str,
    links: &[String],
) -> Vec<(String, Option<String>)> {
    if !manager.has_hook("onLink") {
        return Vec::new();
    }
    let payload = serde_json::json!({ "path": path, "links": links });
    manager.dispatch_all("onLink", &payload.to_string())
}

/// Dispatch `onSearch` to all enabled plugins that declare it. Payload is
/// `{"query": "…", "limit": N}` (spec §8).
pub fn dispatch_on_search(
    manager: &PluginManager,
    query: &str,
    limit: usize,
) -> Vec<(String, Option<String>)> {
    if !manager.has_hook("onSearch") {
        return Vec::new();
    }
    let payload = serde_json::json!({ "query": query, "limit": limit });
    manager.dispatch_all("onSearch", &payload.to_string())
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

    /// Write a source `.wasm` + sidecar manifest into a temp dir so it can be
    /// installed via `PluginManager::install`. Returns the wasm path.
    fn write_source_plugin(src_dir: &std::path::Path) -> std::path::PathBuf {
        std::fs::create_dir_all(src_dir).expect("mkdir src dir");
        let wasm_path = src_dir.join("to-install.wasm");
        std::fs::write(&wasm_path, EMPTY_MODULE).expect("write source wasm");
        std::fs::write(
            src_dir.join("to-install.wasm.manifest.json"),
            SIDECAR_MANIFEST,
        )
        .expect("write source manifest");
        wasm_path
    }

    #[test]
    fn install_registers_disabled_and_persists_to_config() {
        let vault = tempfile::tempdir().expect("tempdir");
        let manager = PluginManager::init_for_vault(vault.path()).expect("init_for_vault");
        assert!(manager.is_empty());

        let src = write_source_plugin(&vault.path().join("_src"));
        let info = manager.install(&src).expect("install must succeed");

        assert_eq!(info.id, "com.example.scan-test");
        assert_eq!(info.status, "disabled", "fresh install is disabled");
        assert!(!info.enabled);
        assert_eq!(manager.len(), 1);

        // Files landed in the canonical layout.
        let dest = vault
            .path()
            .join(".pkm")
            .join("plugins")
            .join("com.example.scan-test")
            .join("plugin.wasm");
        assert!(
            dest.is_file(),
            "installed wasm must exist at the canonical path"
        );

        // Config now lists the plugin as disabled.
        let config = pkm_core::Config::load(vault.path().join(".pkm").join("config.toml")).unwrap();
        let entry = config
            .plugins
            .iter()
            .find(|p| p.name == "com.example.scan-test")
            .expect("config must contain the installed plugin");
        assert!(!entry.enabled);
    }

    #[test]
    fn install_missing_source_errors() {
        let vault = tempfile::tempdir().expect("tempdir");
        let manager = PluginManager::init_for_vault(vault.path()).expect("init_for_vault");
        let missing = vault.path().join("does-not-exist.wasm");
        let err = manager.install(&missing).expect_err("must fail");
        assert!(err.contains("plugin_not_found"), "err: {err}");
    }

    #[test]
    fn install_invalid_wasm_errors() {
        let vault = tempfile::tempdir().expect("tempdir");
        let manager = PluginManager::init_for_vault(vault.path()).expect("init_for_vault");
        let src_dir = vault.path().join("_src");
        std::fs::create_dir_all(&src_dir).unwrap();
        let bad = src_dir.join("bad.wasm");
        std::fs::write(&bad, b"not wasm").unwrap();
        std::fs::write(
            src_dir.join("bad.wasm.manifest.json"),
            r#"{
                "schema_version": 1,
                "id": "com.example.bad",
                "name": "Bad",
                "version": "0.1.0",
                "entry": "bad.wasm",
                "permissions": [],
                "hooks": {}
            }"#,
        )
        .unwrap();
        let err = manager.install(&bad).expect_err("invald module must fail");
        assert!(err.contains("plugin_load_error"), "err: {err}");
    }

    #[test]
    fn uninstall_removes_plugin_dir_and_config_entry() {
        let vault = tempfile::tempdir().expect("tempdir");
        let manager = PluginManager::init_for_vault(vault.path()).expect("init_for_vault");
        let src = write_source_plugin(&vault.path().join("_src"));
        manager.install(&src).expect("install");
        assert_eq!(manager.len(), 1);

        let result = manager
            .uninstall("com.example.scan-test")
            .expect("uninstall");
        assert!(manager.is_empty(), "registry must be empty after uninstall");
        assert!(result.plugins.is_empty(), "updated list must be empty");

        let dest_dir = vault
            .path()
            .join(".pkm")
            .join("plugins")
            .join("com.example.scan-test");
        assert!(!dest_dir.exists(), "plugin dir must be removed");

        let config = pkm_core::Config::load(vault.path().join(".pkm").join("config.toml")).unwrap();
        assert!(
            config
                .plugins
                .iter()
                .all(|p| p.name != "com.example.scan-test"),
            "config entry must be removed"
        );
    }
}
