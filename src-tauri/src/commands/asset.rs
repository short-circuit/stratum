//! Asset / attachment management commands.
//!
//! Thin glue layer exposing the `pkm_block::assets` free functions to the frontend over
//! Tauri IPC. Each command locks `AppState` to clone the vault/db paths, then runs the
//! store work on a blocking pool so the async event loop is never stalled. Asset bytes
//! are transported as base64; the file on disk remains the single source of truth.

use crate::commands::vault::AppState;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};

/// Payload for reading an asset's bytes back to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetBytesDto {
    pub data_base64: String,
    pub mime: String,
    pub size: u64,
}

/// Import an asset from an existing file path.
///
/// The path is resolved inside the vault via `vault_path.join(path)` and the file's
/// bytes are registered as an asset. `dest_name` optionally overrides the default
/// deterministic name.
#[tauri::command]
pub async fn import_asset_path(
    path: String,
    dest_name: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<pkm_block::assets::AssetRef, String> {
    let vault_path = {
        let s = state.lock().map_err(|e| e.to_string())?;
        s.vault_path.clone()
    };
    let db_path = {
        let s = state.lock().map_err(|e| e.to_string())?;
        s.db_path.clone()
    };

    tokio::task::spawn_blocking(move || {
        let store = pkm_block::BlockStore::open(&db_path)?;
        let abs = vault_path.join(&path);
        pkm_block::assets::register_asset_path(
            &store,
            &vault_path,
            &abs,
            None,
            dest_name.as_deref(),
        )
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

/// Import an asset from base64-encoded bytes (e.g. a clipboard paste or drag-drop).
///
/// `original_name` provides a fallback name; if empty it defaults to `paste.png`.
#[tauri::command]
pub async fn import_asset_bytes(
    data_base64: String,
    original_name: String,
    mime: Option<String>,
    dest_name: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<pkm_block::assets::AssetRef, String> {
    let vault_path = {
        let s = state.lock().map_err(|e| e.to_string())?;
        s.vault_path.clone()
    };
    let db_path = {
        let s = state.lock().map_err(|e| e.to_string())?;
        s.db_path.clone()
    };

    let bytes = STANDARD
        .decode(data_base64)
        .map_err(|e| format!("Invalid base64 data: {e}"))?;
    let original_name = if original_name.trim().is_empty() {
        "paste.png".to_string()
    } else {
        original_name
    };

    tokio::task::spawn_blocking(move || {
        let store = pkm_block::BlockStore::open(&db_path)?;
        pkm_block::assets::register_asset_bytes(
            &store,
            &vault_path,
            &bytes,
            &original_name,
            mime.as_deref(),
            dest_name.as_deref(),
        )
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

/// Read an asset's bytes (as base64), MIME, and size.
#[tauri::command]
pub async fn read_asset(
    ref_path: String,
    state: tauri::State<'_, AppState>,
) -> Result<AssetBytesDto, String> {
    let vault_path = {
        let s = state.lock().map_err(|e| e.to_string())?;
        s.vault_path.clone()
    };
    let db_path = {
        let s = state.lock().map_err(|e| e.to_string())?;
        s.db_path.clone()
    };

    let (data, mime) = tokio::task::spawn_blocking(move || {
        let store = pkm_block::BlockStore::open(&db_path)?;
        let (bytes, mime) = pkm_block::assets::read_asset(&store, &vault_path, &ref_path)?;
        Ok::<_, pkm_core::PkmError>((bytes, mime))
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    let size = data.len() as u64;
    Ok(AssetBytesDto {
        data_base64: STANDARD.encode(data),
        mime,
        size,
    })
}

/// Delete an asset (removes the file and its index record).
#[tauri::command]
pub async fn delete_asset(
    ref_path: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let vault_path = {
        let s = state.lock().map_err(|e| e.to_string())?;
        s.vault_path.clone()
    };
    let db_path = {
        let s = state.lock().map_err(|e| e.to_string())?;
        s.db_path.clone()
    };

    tokio::task::spawn_blocking(move || {
        let store = pkm_block::BlockStore::open(&db_path)?;
        pkm_block::assets::delete_asset(&store, &vault_path, &ref_path)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

/// List every asset registered in the vault, ordered by ref path.
#[tauri::command]
pub async fn list_asset_refs(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<pkm_block::assets::AssetRef>, String> {
    let vault_path = {
        let s = state.lock().map_err(|e| e.to_string())?;
        s.vault_path.clone()
    };
    let db_path = {
        let s = state.lock().map_err(|e| e.to_string())?;
        s.db_path.clone()
    };

    tokio::task::spawn_blocking(move || {
        let store = pkm_block::BlockStore::open(&db_path)?;
        pkm_block::assets::list_assets(&store, &vault_path)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

/// List the assets referenced by a specific page, resolving each to its record.
#[tauri::command]
pub async fn get_page_asset_refs(
    page_path: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<pkm_block::assets::AssetRef>, String> {
    let db_path = {
        let s = state.lock().map_err(|e| e.to_string())?;
        s.db_path.clone()
    };

    tokio::task::spawn_blocking(move || {
        let store = pkm_block::BlockStore::open(&db_path)?;
        let refs = store.get_asset_refs_for_page(&page_path)?;
        let mut out = Vec::new();
        for rp in refs {
            if let Some(rec) = store.get_asset_record(&rp)? {
                out.push(pkm_block::assets::AssetRef {
                    vault_relative_path: rec.ref_path,
                    mime: rec.mime,
                    size: rec.size as u64,
                    sha256: rec.sha256,
                });
            }
        }
        Ok::<_, pkm_core::PkmError>(out)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}
