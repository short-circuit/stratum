//! Whiteboard commands.
//!
//! Whiteboards are stored as `.excalidraw` files (Excalidraw JSON format) in the vault.

use crate::commands::vault::AppState;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WhiteboardDto {
    pub name: String,
    pub path: String,
    pub content: String,
}

#[tauri::command]
pub async fn list_whiteboards(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<WhiteboardDto>, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let wb_dir = state.vault_path.join("whiteboards");
    if !wb_dir.exists() {
        return Ok(Vec::new());
    }

    let mut boards = Vec::new();
    let entries = std::fs::read_dir(&wb_dir).map_err(|e| e.to_string())?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("excalidraw") {
            let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("untitled")
                .to_string();
            let rel_path = path
                .strip_prefix(&state.vault_path)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();
            boards.push(WhiteboardDto {
                name,
                path: rel_path,
                content,
            });
        }
    }

    Ok(boards)
}

#[tauri::command]
pub async fn save_whiteboard(
    name: String,
    content: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let wb_dir = state.vault_path.join("whiteboards");
    std::fs::create_dir_all(&wb_dir).map_err(|e| e.to_string())?;
    let path = wb_dir.join(format!("{}.excalidraw", name));
    std::fs::write(&path, &content).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn rename_whiteboard(
    old_name: String,
    new_name: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let wb_dir = state.vault_path.join("whiteboards");
    let old_path = wb_dir.join(format!("{}.excalidraw", old_name));
    let new_path = wb_dir.join(format!("{}.excalidraw", new_name));
    if !old_path.exists() {
        return Err("Whiteboard not found".to_string());
    }
    if new_path.exists() {
        return Err("A whiteboard with that name already exists".to_string());
    }
    std::fs::rename(&old_path, &new_path).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn delete_whiteboard(
    name: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let path = state
        .vault_path
        .join("whiteboards")
        .join(format!("{}.excalidraw", name));
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn load_whiteboard(
    name: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let path = state
        .vault_path
        .join("whiteboards")
        .join(format!("{}.excalidraw", name));
    if !path.exists() {
        return Ok(serde_json::json!({
            "elements": [],
            "appState": { "viewBackgroundColor": "#ffffff" }
        })
        .to_string());
    }
    std::fs::read_to_string(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_library(
    content: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let wb_dir = state.vault_path.join("whiteboards");
    std::fs::create_dir_all(&wb_dir).map_err(|e| e.to_string())?;
    let path = wb_dir.join("library.excalidrawlib");
    std::fs::write(&path, &content).map_err(|e| e.to_string())?;
    Ok(())
}

/// Load the user's personal library from `library.excalidrawlib`.
#[tauri::command]
pub async fn load_library(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let path = state
        .vault_path
        .join("whiteboards")
        .join("library.excalidrawlib");
    if !path.exists() {
        return Ok("[]".to_string());
    }
    std::fs::read_to_string(&path).map_err(|e| e.to_string())
}

/// Load additional `.excalidrawlib` files from the whiteboards folder (excluding
/// `library.excalidrawlib`) and merge them into a single JSON array. Any library
/// files placed in the folder are auto-loaded into the library panel.
#[tauri::command]
pub async fn load_extra_libraries(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let wb_dir = state.vault_path.join("whiteboards");
    if !wb_dir.exists() {
        return Ok("[]".to_string());
    }

    let mut merged: Vec<serde_json::Value> = Vec::new();
    let entries = std::fs::read_dir(&wb_dir).map_err(|e| e.to_string())?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        // Skip the personal library file — it's loaded separately
        if path.file_stem().and_then(|s| s.to_str()) == Some("library") {
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) == Some("excalidrawlib") {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(items) = serde_json::from_str::<Vec<serde_json::Value>>(&content) {
                    merged.extend(items);
                }
            }
        }
    }

    serde_json::to_string(&merged).map_err(|e| e.to_string())
}
