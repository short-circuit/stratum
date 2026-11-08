//! Vault management commands.
//!
//! Provides vault info, path switching, and initialization (including Android
//! SAF content-URI resolution). Hosts resolve a user-picked directory and call
//! [`StratumService::init_vault_at`] to run the shared setup body.

use crate::plugins::PluginManager;
use crate::state::{StratumService, VaultState};
use pkm_index::indexer::IndexEngine;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VaultInfo {
    pub path: String,
    pub block_count: usize,
    pub page_count: usize,
}

impl StratumService {
    pub async fn get_vault_info(&self) -> Result<VaultInfo, String> {
        let state = self.lock()?;
        let store = state.get_store().map_err(|e| e.to_string())?;
        let block_count = store.block_count().map_err(|e| e.to_string())?;
        let page_count = store.page_count().map_err(|e| e.to_string())?;

        Ok(VaultInfo {
            path: state.vault_path.to_string_lossy().to_string(),
            block_count,
            page_count,
        })
    }

    pub async fn set_vault_path(&self, path: String) -> Result<(), String> {
        let vault_path = PathBuf::from(&path);
        if !vault_path.exists() {
            return Err(format!("Vault path does not exist: {}", path));
        }
        let mut state = self.lock()?;
        state.vault_path = vault_path.clone();
        state.db_path = vault_path.join(".pkm").join("blocks.db");
        Ok(())
    }

    /// Initialize vault at a user-picked path.
    /// Frontend calls `open({ directory: true })` and passes the result here.
    pub async fn init_vault(&self, path: String) -> Result<VaultInfo, String> {
        let vault_path = resolve_picked_path(&path)?;

        if !vault_path.exists() {
            std::fs::create_dir_all(&vault_path)
                .map_err(|e| format!("Failed to create vault directory: {}", e))?;
        }

        self.init_vault_at(vault_path).await
    }

    /// Shared initialization body used by every host-specific picker.
    pub async fn init_vault_at(&self, vault_path: PathBuf) -> Result<VaultInfo, String> {
        let mut vstate = self.lock()?;
        let (_, block_count, page_count) = setup_vault(&vault_path, &mut vstate)?;
        Ok(VaultInfo {
            path: vault_path.to_string_lossy().to_string(),
            block_count,
            page_count,
        })
    }
}

/// Resolve a user-picked path to a real filesystem path.
/// On Android, converts SAF content URI to a real path.
#[cfg(target_os = "android")]
pub fn resolve_picked_path(picked: &str) -> Result<PathBuf, String> {
    resolve_saf_content_uri(picked)
}

#[cfg(not(target_os = "android"))]
pub fn resolve_picked_path(picked: &str) -> Result<PathBuf, String> {
    Ok(PathBuf::from(picked))
}

/// Parse an Android SAF content URI of the form:
/// `content://.../tree/primary:Documents%2FMyFolder` and resolve it to a
/// real filesystem path like `/storage/emulated/0/Documents/MyFolder`.
/// Also handles URIs with a `/document/` suffix segment which Android's
/// `ACTION_OPEN_DOCUMENT_TREE` may append after the tree root.
#[allow(dead_code)]
pub fn resolve_saf_content_uri(picked: &str) -> Result<PathBuf, String> {
    // Extract the encoded path after "/tree/" — drop any "/document/" suffix
    let path_encoded = picked
        .split("/tree/")
        .nth(1)
        .ok_or_else(|| format!("Could not parse Android content URI: {}", picked))?
        .split("/document/")
        .next()
        .unwrap_or("");
    let path_decoded = percent_decode(path_encoded);

    if let Some(subpath) = path_decoded.strip_prefix("primary:") {
        Ok(PathBuf::from("/storage/emulated/0").join(subpath))
    } else if let Some((volume, subpath)) = path_decoded.split_once(':') {
        Ok(PathBuf::from("/storage").join(volume).join(subpath))
    } else {
        Err(format!("Unrecognized content URI format: {}", picked))
    }
}

#[allow(dead_code)]
pub fn percent_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if let Ok(b) = u8::from_str_radix(&hex, 16) {
                out.push(b as char);
            } else {
                out.push('%');
                out.push_str(&hex);
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn setup_vault(
    vault_path: &std::path::Path,
    vstate: &mut VaultState,
) -> Result<(pkm_block::BlockStore, usize, usize), String> {
    let pkm_dir = vault_path.join(".pkm");
    std::fs::create_dir_all(&pkm_dir).map_err(|e| format!("Failed to initialize vault: {e}"))?;

    // Write a .gitignore inside .pkm/ to prevent derived/indexed data from
    // being committed — these are rebuilt from .md files and cause merge
    // conflicts during git sync when tracked.
    let gitignore_path = pkm_dir.join(".gitignore");
    if !gitignore_path.exists() {
        let contents = "\
# Derived data — rebuilt from .md files, never commit these
blocks.db
blocks.db-wal
blocks.db-shm
cache.db
cache.db-wal
cache.db-shm
search.idx/
history/
";
        let _ = std::fs::write(&gitignore_path, contents);
    }

    let db_path = pkm_dir.join("blocks.db");
    let index_engine = IndexEngine::new(vault_path).ok();
    vstate.vault_path = vault_path.to_path_buf();
    vstate.db_path = db_path.clone();
    vstate.index_engine = index_engine;

    // ── WASM plugins (spec §7.3) ────────────────────────────────────────────
    // Scan `<vault>/.pkm/plugins/` and build the plugin manager. Enablement is
    // taken from the vault config `plugins` list. Failures are logged and
    // surfaced through `plugins_list`; a broken plugin never aborts load.
    // Shares the same single entry point as the app startup scan.
    let plugin_manager = PluginManager::init_for_vault(vault_path)
        .map_err(|e| format!("Failed to initialize plugin manager: {e}"))?;

    // Replacing the manager drops the *previous* one. That drop shuts down the
    // old `PluginRuntime`, which owns an embedded tokio runtime (see
    // pkm-plugin/src/runtime.rs). On tokio >= 1.52, dropping a `Runtime` from
    // inside an entered async context panics with "Cannot drop a runtime in a
    // context where blocking is not allowed" — and because we hold the vault
    // `Mutex` here, that panic poisons it for the rest of the process (E7.F9
    // on-device finding). A freshly spawned OS thread has no tokio thread-local
    // context, so runtime shutdown there is always allowed. Vault setup is a
    // rare operation, so one throwaway thread per re-init is acceptable.
    let old_manager = vstate.plugin_manager.replace(plugin_manager);
    if let Some(old) = old_manager {
        std::thread::spawn(move || drop(old));
    }

    let store = pkm_block::BlockStore::open(&db_path).map_err(|e| e.to_string())?;
    vstate.block_store = pkm_block::BlockStore::open(&db_path).ok();
    let block_count = store.block_count().map_err(|e| e.to_string())?;
    let page_count = store.page_count().map_err(|e| e.to_string())?;
    Ok((store, block_count, page_count))
}

#[cfg(test)]
mod tests;
