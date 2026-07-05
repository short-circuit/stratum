//! Vault management commands.

use pkm_index::indexer::IndexEngine;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

/// Application state holding the active vault.
pub struct VaultState {
    pub vault_path: PathBuf,
    pub db_path: PathBuf,
    pub index_engine: Option<IndexEngine>,
    pub sync_scheduler: Option<pkm_sync::SyncScheduler>,
    pub auto_commit_engine: Option<pkm_sync::AutoCommitEngine>,
    pub passphrase: Option<String>,
}

impl VaultState {
    pub fn new(vault_path: PathBuf) -> Self {
        let db_path = vault_path.join(".pkm").join("blocks.db");
        let index_engine = IndexEngine::new(&vault_path).ok();
        if index_engine.is_some() {
            eprintln!("[stratum] IndexEngine initialized at {:?}", vault_path);
        }
        Self {
            vault_path,
            db_path,
            index_engine,
            sync_scheduler: None,
            auto_commit_engine: None,
            passphrase: None,
        }
    }

    pub fn ensure_index(&mut self) -> Result<&mut IndexEngine, String> {
        if self.index_engine.is_none() {
            self.index_engine = Some(
                IndexEngine::new(&self.vault_path)
                    .map_err(|e| format!("Failed to create IndexEngine: {}", e))?,
            );
        }
        self.index_engine
            .as_mut()
            .ok_or_else(|| "IndexEngine not available".to_string())
    }

    pub fn set_passphrase(&mut self, passphrase: String) {
        self.passphrase = Some(passphrase);
    }
}

pub type AppState = Mutex<VaultState>;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VaultInfo {
    pub path: String,
    pub block_count: usize,
    pub page_count: usize,
}

#[tauri::command]
pub async fn get_vault_info(state: tauri::State<'_, AppState>) -> Result<VaultInfo, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let store = pkm_block::BlockStore::open(&state.db_path).map_err(|e| e.to_string())?;
    let block_count = store.block_count().map_err(|e| e.to_string())?;
    let page_count = store.page_count().map_err(|e| e.to_string())?;

    Ok(VaultInfo {
        path: state.vault_path.to_string_lossy().to_string(),
        block_count,
        page_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percent_decode_simple() {
        assert_eq!(percent_decode("hello%20world"), "hello world");
    }

    #[test]
    fn test_percent_decode_no_encoding() {
        assert_eq!(percent_decode("plaintext"), "plaintext");
    }

    #[test]
    fn test_percent_decode_multiple() {
        assert_eq!(percent_decode("%46%6F%6F"), "Foo");
    }

    #[test]
    fn test_percent_decode_empty() {
        assert_eq!(percent_decode(""), "");
    }

    #[test]
    fn test_resolve_saf_primary_storage() {
        let uri = "content://com.android.externalstorage.documents/tree/primary%3ADocuments%2FMyVault";
        let result = resolve_saf_content_uri(uri).unwrap();
        assert_eq!(result, PathBuf::from("/storage/emulated/0/Documents/MyVault"));
    }

    #[test]
    fn test_resolve_saf_secondary_volume() {
        let uri = "content://com.android.externalstorage.documents/tree/1234-5678%3AMyVault";
        let result = resolve_saf_content_uri(uri).unwrap();
        assert_eq!(result, PathBuf::from("/storage/1234-5678/MyVault"));
    }

    #[test]
    fn test_resolve_saf_deeply_nested() {
        let uri = "content://com.android.externalstorage.documents/tree/primary%3ADocuments%2FProjects%2FStratum%2Fvault";
        let result = resolve_saf_content_uri(uri).unwrap();
        assert_eq!(result, PathBuf::from("/storage/emulated/0/Documents/Projects/Stratum/vault"));
    }

    #[test]
    fn test_resolve_saf_invalid_uri() {
        let uri = "content://com.android.something/not-a-tree-uri";
        let result = resolve_saf_content_uri(uri);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Could not parse Android content URI"));
    }

    #[test]
    fn test_resolve_picked_path_non_android() {
        #[cfg(not(target_os = "android"))]
        {
            let result = resolve_picked_path("/home/user/vault").unwrap();
            assert_eq!(result, PathBuf::from("/home/user/vault"));
        }
    }

    #[test]
    fn test_resolve_saf_with_document_suffix() {
        // Some SAF implementations append /document/primary:... after the tree root
        let uri = "content://com.android.externalstorage.documents/tree/primary%3AStratumVault/document/primary%3AStratumVault";
        let result = resolve_saf_content_uri(uri).unwrap();
        assert_eq!(result, PathBuf::from("/storage/emulated/0/StratumVault"));
    }
}

#[tauri::command]
pub async fn set_vault_path(path: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let vault_path = PathBuf::from(&path);
    if !vault_path.exists() {
        return Err(format!("Vault path does not exist: {}", path));
    }
    let mut state = state.lock().map_err(|e| e.to_string())?;
    state.vault_path = vault_path.clone();
    state.db_path = vault_path.join(".pkm").join("blocks.db");
    Ok(())
}

/// Resolve a user-picked path to a real filesystem path.
/// On Android, converts SAF content URI to a real path.
#[cfg(target_os = "android")]
fn resolve_picked_path(picked: &str) -> Result<PathBuf, String> {
    resolve_saf_content_uri(picked)
}

#[cfg(not(target_os = "android"))]
fn resolve_picked_path(picked: &str) -> Result<PathBuf, String> {
    Ok(PathBuf::from(picked))
}

/// Parse an Android SAF content URI of the form:
/// `content://.../tree/primary:Documents%2FMyFolder` and resolve it to a
/// real filesystem path like `/storage/emulated/0/Documents/MyFolder`.
/// Also handles URIs with a `/document/` suffix segment which Android's
/// `ACTION_OPEN_DOCUMENT_TREE` may append after the tree root.
fn resolve_saf_content_uri(picked: &str) -> Result<PathBuf, String> {
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

fn percent_decode(s: &str) -> String {
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

/// Initialize vault at a user-picked path.
/// Frontend calls `open({ directory: true })` and passes the result here.
#[tauri::command]
pub async fn init_vault(
    path: String,
    state: tauri::State<'_, AppState>,
) -> Result<VaultInfo, String> {
    let vault_path = resolve_picked_path(&path)?;

    if !vault_path.exists() {
        std::fs::create_dir_all(&vault_path)
            .map_err(|e| format!("Failed to create vault directory: {}", e))?;
    }

    let mut vstate = state.lock().map_err(|e| e.to_string())?;
    let (_, block_count, page_count) = setup_vault(&vault_path, &mut vstate)?;

    Ok(VaultInfo {
        path: vault_path.to_string_lossy().to_string(),
        block_count,
        page_count,
    })
}

/// Initialize vault at the application's default data directory
/// (app-private storage on Android, ~/StratumVault on desktop).
/// This is the command called on mobile instead of the folder picker.
#[tauri::command]
pub async fn init_default_vault(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<VaultInfo, String> {
    let vault_path = crate::resolve_default_vault_path(&app);
    std::fs::create_dir_all(&vault_path)
        .map_err(|e| format!("Failed to create vault directory: {}", e))?;
    let mut vstate = state.lock().map_err(|e| e.to_string())?;
    let (_, block_count, page_count) = setup_vault(&vault_path, &mut vstate)?;
    Ok(VaultInfo {
        path: vault_path.to_string_lossy().to_string(),
        block_count,
        page_count,
    })
}

fn setup_vault(
    vault_path: &std::path::Path,
    vstate: &mut VaultState,
) -> Result<(pkm_block::BlockStore, usize, usize), String> {
    std::fs::create_dir_all(vault_path.join(".pkm"))
        .map_err(|e| format!("Failed to initialize vault: {}", e))?;

    let db_path = vault_path.join(".pkm").join("blocks.db");
    let index_engine = IndexEngine::new(vault_path).ok();
    vstate.vault_path = vault_path.to_path_buf();
    vstate.db_path = db_path.clone();
    vstate.index_engine = index_engine;

    let store = pkm_block::BlockStore::open(&db_path).map_err(|e| e.to_string())?;
    let block_count = store.block_count().map_err(|e| e.to_string())?;
    let page_count = store.page_count().map_err(|e| e.to_string())?;
    Ok((store, block_count, page_count))
}

#[cfg(desktop)]
#[tauri::command]
pub async fn pick_vault_directory(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<VaultInfo, String> {
    use tauri_plugin_dialog::DialogExt;

    let (tx, rx) = std::sync::mpsc::channel();

    app.dialog().file().pick_folder(move |path| {
        let _ = tx.send(path);
    });

    let picked = rx
        .recv()
        .map_err(|_| "Folder picker cancelled".to_string())?;
    let picked_path = picked.ok_or("No folder selected".to_string())?;

    let vault_path = PathBuf::from(picked_path.to_string());
    if !vault_path.exists() {
        return Err(format!(
            "Selected path does not exist: {}",
            vault_path.display()
        ));
    }

    let mut vstate = state.lock().map_err(|e| e.to_string())?;
    let (_, block_count, page_count) = setup_vault(&vault_path, &mut vstate)?;

    Ok(VaultInfo {
        path: vault_path.to_string_lossy().to_string(),
        block_count,
        page_count,
    })
}
