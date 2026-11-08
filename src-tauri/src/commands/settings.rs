//! Tauri adapters for settings and AI-configuration commands.
//!
//! Thin delegation to [`stratum_app::settings`]. `save_settings` additionally
//! applies the tray state on desktop, which is host-only glue.

pub use stratum_app::settings::{
    AiModelDto, AiSettingsDto, BackupSettingsDto, GraphSettingsDto, LayoutSettingsDto,
    ReminderSettingsDto, ResearchSettingsDto, SettingsDto, SttSettingsDto, SyncSettingsDto,
    ThemeSettingsDto, TraySettingsDto, TtsSettingsDto, VoiceSettingsDto,
};

use crate::commands::vault::AppState;

#[tauri::command]
pub async fn get_settings(state: tauri::State<'_, AppState>) -> Result<SettingsDto, String> {
    state.get_settings().await
}

#[tauri::command]
pub async fn save_settings<R: tauri::Runtime>(
    settings: SettingsDto,
    app: tauri::AppHandle<R>,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let config = state.save_settings(settings).await?;
    #[cfg(desktop)]
    crate::commands::tray::apply_tray_state(&app, &config.tray);
    #[cfg(not(desktop))]
    let _ = (&app, &config);
    Ok(())
}

#[tauri::command]
pub async fn save_graph_settings(
    graph: GraphSettingsDto,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    state.save_graph_settings(graph).await
}

#[tauri::command]
pub async fn fetch_models(state: tauri::State<'_, AppState>) -> Result<Vec<String>, String> {
    state.fetch_models().await
}

#[tauri::command]
pub async fn get_onboarding_state(state: tauri::State<'_, AppState>) -> Result<bool, String> {
    state.get_onboarding_state().await
}

#[tauri::command]
pub async fn mark_onboarding_done(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.mark_onboarding_done().await
}

#[tauri::command]
pub async fn set_onboarding_pending(
    pending: bool,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    state.set_onboarding_pending(pending).await
}
