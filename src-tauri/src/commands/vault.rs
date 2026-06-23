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
