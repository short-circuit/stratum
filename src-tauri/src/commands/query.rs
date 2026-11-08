//! Tauri adapters for Datalog query commands.

pub use pkm_query::saved_queries::SavedQuery;
pub use stratum_app::query::QueryResultDto;

use crate::commands::vault::AppState;

#[tauri::command]
pub async fn run_query(
    datalog: String,
    state: tauri::State<'_, AppState>,
) -> Result<QueryResultDto, String> {
    state.run_query(datalog).await
}

#[tauri::command]
pub async fn list_saved_queries(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<SavedQuery>, String> {
    state.list_saved_queries().await
}

#[tauri::command]
pub async fn save_saved_query(
    name: String,
    query: String,
    state: tauri::State<'_, AppState>,
) -> Result<SavedQuery, String> {
    state.save_saved_query(name, query).await
}

#[tauri::command]
pub async fn rename_saved_query(
    old_name: String,
    new_name: String,
    state: tauri::State<'_, AppState>,
) -> Result<SavedQuery, String> {
    state.rename_saved_query(old_name, new_name).await
}

#[tauri::command]
pub async fn delete_saved_query(
    name: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    state.delete_saved_query(name).await
}
