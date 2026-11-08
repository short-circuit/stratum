//! Tauri adapters for block reminder commands.

pub use stratum_app::reminders::ReminderDto;

use crate::commands::vault::AppState;

#[tauri::command]
pub async fn list_reminders(state: tauri::State<'_, AppState>) -> Result<Vec<ReminderDto>, String> {
    state.list_reminders().await
}

#[tauri::command]
pub async fn snooze_reminder(
    page_path: String,
    block_id: String,
    minutes: u32,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    state.snooze_reminder(page_path, block_id, minutes).await
}

#[tauri::command]
pub async fn clear_reminder(
    page_path: String,
    block_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    state.clear_reminder(page_path, block_id).await
}
