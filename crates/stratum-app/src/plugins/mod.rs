//! Plugin lifecycle support (manager, DTOs, install/URL install, hook dispatch).
//!
//! The manager owns the WASM runtime and per-vault registry; it is stored in
//! [`crate::state::VaultState`] as an `Arc` so plugin work never blocks the
//! vault mutex. Commands are exposed as [`crate::state::StratumService`]
//! methods; hosts keep only their IPC adapters.

pub mod dto;
pub mod hooks;
pub mod install;
pub mod manager;

#[cfg(test)]
mod tests;

pub use dto::{PluginHttpResponseDto, PluginInfo, PluginListResult, PluginNoteDto};
pub use hooks::{
    dispatch_hook, dispatch_on_link, dispatch_on_open, dispatch_on_save, dispatch_on_search,
};
pub use install::install_url;
pub use manager::PluginManager;

use crate::state::StratumService;
use crate::state::VaultState;
use pkm_plugin::{HostApi, VaultHost};
use std::sync::Arc;

fn get_manager(state: &VaultState) -> Result<Arc<PluginManager>, String> {
    state
        .plugin_manager
        .clone()
        .ok_or_else(|| "no vault is open — plugin runtime not initialized".to_string())
}

impl StratumService {
    /// §9.1 — List installed plugins with status. No vault open ⇒ empty list.
    pub async fn plugins_list(&self) -> Result<PluginListResult, String> {
        let locked = self.lock()?;
        let Some(manager) = locked.plugin_manager.clone() else {
            return Ok(PluginListResult {
                plugins: Vec::new(),
            });
        };
        drop(locked);
        Ok(manager.list())
    }

    /// §9.2 — Enable a disabled plugin and re-arm hooks.
    pub async fn plugins_enable(&self, id: String) -> Result<PluginInfo, String> {
        let locked = self.lock()?;
        let manager = get_manager(&locked)?;
        drop(locked);
        manager.enable(&id)
    }

    /// §9.3 — Disable a running plugin.
    pub async fn plugins_disable(&self, id: String) -> Result<PluginInfo, String> {
        let locked = self.lock()?;
        let manager = get_manager(&locked)?;
        drop(locked);
        manager.disable(&id)
    }

    /// §9.4 — Re-instantiate a plugin from disk.
    pub async fn plugins_reload(&self, id: String) -> Result<PluginInfo, String> {
        let locked = self.lock()?;
        let manager = get_manager(&locked)?;
        drop(locked);
        manager.reload(&id)
    }

    /// §9.5 — Full status for one plugin.
    pub async fn plugins_status(&self, id: String) -> Result<PluginInfo, String> {
        let locked = self.lock()?;
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
    pub async fn plugins_install(&self, path: String) -> Result<PluginInfo, String> {
        let locked = self.lock()?;
        let manager = get_manager(&locked)?;
        drop(locked);
        manager.install(std::path::Path::new(&path))
    }

    /// §9.10 (F-series) — Install a plugin by downloading it from a catalog URL.
    ///
    /// Downloads the WASM bytes from `download_url`, verifies the `sha256` hex
    /// digest against the catalog entry when one is provided, stages the bytes in
    /// a temp file, and delegates to [`PluginManager::install`] (which reads the
    /// embedded `stratum:manifest`, registers + persists, and compiles the
    /// module). The staged temp file is cleaned up on all paths. A digest
    /// mismatch fails fast with `plugin_checksum_error` before anything is
    /// written into the vault.
    pub async fn plugins_install_url(
        &self,
        url: String,
        sha256: Option<String>,
        id: String,
    ) -> Result<PluginInfo, String> {
        let manager = {
            let locked = self.lock()?;
            get_manager(&locked)?
        };
        install_url(&manager, &url, sha256.as_deref(), &id).await
    }

    /// §9.9 (F-series) — Uninstall a plugin by id.
    ///
    /// Unloads the plugin from the registry, removes its directory and config
    /// entry, and returns the updated plugin list.
    pub async fn plugins_uninstall(&self, id: String) -> Result<PluginListResult, String> {
        let locked = self.lock()?;
        let manager = get_manager(&locked)?;
        drop(locked);
        manager.uninstall(&id)
    }

    /// §9.6 — Run the same `note_read` backend used by plugins, without a plugin.
    pub async fn plugin_note_read(&self, path: String) -> Result<PluginNoteDto, String> {
        let root = {
            let locked = self.lock()?;
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
    pub async fn plugin_http_request(
        &self,
        method: Option<String>,
        url: String,
        headers: Option<Vec<(String, String)>>,
        body: Option<String>,
        timeout_ms: Option<u64>,
    ) -> Result<PluginHttpResponseDto, String> {
        let root = {
            let locked = self.lock()?;
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
            .map_err(|code| format!("invalid_argument: {}", code.as_str()))?;
        // The host-test path is not gated by plugin permissions; use an empty
        // allowlist (private/loopback default) for the same SSRF behavior.
        let host = VaultHost::root(root);
        let resp = host
            .http_request(req)
            .await
            .map_err(|code| code.as_str().to_string())?;
        Ok(PluginHttpResponseDto {
            status: resp.status,
            headers: resp.headers,
            body: resp.body,
        })
    }
}
