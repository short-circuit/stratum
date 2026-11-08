//! Tauri adapters for whiteboard commands.

pub use stratum_app::whiteboard::WhiteboardDto;

use crate::commands::vault::AppState;

#[tauri::command]
pub async fn list_whiteboards(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<WhiteboardDto>, String> {
    state.list_whiteboards().await
}

#[tauri::command]
pub async fn save_whiteboard(
    name: String,
    content: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    state.save_whiteboard(name, content).await
}

#[tauri::command]
pub async fn rename_whiteboard(
    old_name: String,
    new_name: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    state.rename_whiteboard(old_name, new_name).await
}

#[tauri::command]
pub async fn delete_whiteboard(
    name: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    state.delete_whiteboard(name).await
}

#[tauri::command]
pub async fn load_whiteboard(
    name: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    state.load_whiteboard(name).await
}

#[tauri::command]
pub async fn save_library(
    content: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    state.save_library(content).await
}

#[tauri::command]
pub async fn load_library(state: tauri::State<'_, AppState>) -> Result<String, String> {
    state.load_library().await
}

#[tauri::command]
pub async fn load_extra_libraries(state: tauri::State<'_, AppState>) -> Result<String, String> {
    state.load_extra_libraries().await
}
