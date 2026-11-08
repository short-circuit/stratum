//! Tauri adapters for Kanban board commands.

pub use stratum_app::kanban::{KanbanBlockDto, KanbanDataDto};

use crate::commands::vault::AppState;

#[tauri::command]
pub async fn get_kanban_blocks(state: tauri::State<'_, AppState>) -> Result<KanbanDataDto, String> {
    state.get_kanban_blocks().await
}

#[tauri::command]
pub async fn create_kanban_block(
    content: String,
    marker: String,
    state: tauri::State<'_, AppState>,
) -> Result<KanbanBlockDto, String> {
    state.create_kanban_block(content, marker).await
}
