//! Tauri adapters for git sync commands.

pub use stratum_app::sync::{sync_vault_core, CommitLogEntry, SyncStatusDto};

use crate::commands::vault::AppState;

#[tauri::command]
pub async fn get_sync_status(state: tauri::State<'_, AppState>) -> Result<SyncStatusDto, String> {
    state.get_sync_status().await
}

#[tauri::command]
pub async fn sync_vault(state: tauri::State<'_, AppState>) -> Result<SyncStatusDto, String> {
    state.sync_vault().await
}

#[tauri::command]
pub async fn sync_vault_with_passphrase(
    passphrase: String,
    state: tauri::State<'_, AppState>,
) -> Result<SyncStatusDto, String> {
    state.sync_vault_with_passphrase(passphrase).await
}

#[tauri::command]
pub async fn start_sync_scheduler(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.start_sync_scheduler().await
}

#[tauri::command]
pub async fn stop_sync_scheduler(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.stop_sync_scheduler().await
}

#[tauri::command]
pub async fn get_commit_log(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<CommitLogEntry>, String> {
    state.get_commit_log().await
}

#[tauri::command]
pub async fn resolve_conflict_file(
    path: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    state.resolve_conflict_file(path).await
}

#[tauri::command]
pub async fn abort_merge(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.abort_merge().await
}
