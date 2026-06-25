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

#[tauri::command]
pub async fn load_library(
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
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
