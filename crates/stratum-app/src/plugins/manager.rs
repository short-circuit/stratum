//! Per-vault WASM plugin manager.
//!
//! Owns the process-wide plugin runtime and the per-vault registry. Stored in
//! [`crate::state::VaultState`] as an `Arc` so long-running host calls never
//! hold the vault mutex.

use crate::plugins::dto::{PluginInfo, PluginListResult};
use pkm_plugin::{PluginRegistry, PluginRuntime, PluginState, VaultHost};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tracing::{debug, info, warn};

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
