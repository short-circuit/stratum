//! Tauri adapters for cloud/folder backup commands (E10.F3).

pub use stratum_app::backup::BackupStatusDto;

use crate::commands::vault::AppState;

#[tauri::command]
pub async fn run_backup(state: tauri::State<'_, AppState>) -> Result<BackupStatusDto, String> {
    state.run_backup().await
}

#[tauri::command]
pub async fn get_backup_status(
    state: tauri::State<'_, AppState>,
) -> Result<BackupStatusDto, String> {
    state.get_backup_status().await
}

/// Test connectivity + auth against a WebDAV endpoint. No vault lock is needed.
#[tauri::command]
pub async fn test_webdav(
    url: String,
    username: Option<String>,
    password: Option<String>,
) -> Result<String, String> {
    stratum_app::backup::test_webdav(url, username, password).await
}
