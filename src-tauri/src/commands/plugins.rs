//! Tauri adapters for plugin lifecycle commands.
//!
//! Thin delegation to [`stratum_app::plugins`]; the manager and runtime live in
//! the headless crate.

pub use stratum_app::plugins::PluginManager;
pub use stratum_app::plugins::{
    dispatch_hook, dispatch_on_link, dispatch_on_open, dispatch_on_save, dispatch_on_search,
    PluginHttpResponseDto, PluginInfo, PluginListResult, PluginNoteDto,
};

use crate::commands::vault::AppState;

#[tauri::command]
pub async fn plugins_list(state: tauri::State<'_, AppState>) -> Result<PluginListResult, String> {
    state.plugins_list().await
}

#[tauri::command]
pub async fn plugins_enable(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<PluginInfo, String> {
    state.plugins_enable(id).await
}

#[tauri::command]
pub async fn plugins_disable(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<PluginInfo, String> {
    state.plugins_disable(id).await
}

#[tauri::command]
pub async fn plugins_reload(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<PluginInfo, String> {
    state.plugins_reload(id).await
}

#[tauri::command]
pub async fn plugins_status(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<PluginInfo, String> {
    state.plugins_status(id).await
}

#[tauri::command]
pub async fn plugins_install(
    path: String,
    state: tauri::State<'_, AppState>,
) -> Result<PluginInfo, String> {
    state.plugins_install(path).await
}

#[tauri::command]
pub async fn plugins_install_url(
    url: String,
    sha256: Option<String>,
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<PluginInfo, String> {
    state.plugins_install_url(url, sha256, id).await
}

#[tauri::command]
pub async fn plugins_uninstall(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<PluginListResult, String> {
    state.plugins_uninstall(id).await
}

#[tauri::command]
pub async fn plugin_note_read(
    path: String,
    state: tauri::State<'_, AppState>,
) -> Result<PluginNoteDto, String> {
    state.plugin_note_read(path).await
}

#[tauri::command]
pub async fn plugin_http_request(
    method: Option<String>,
    url: String,
    headers: Option<Vec<(String, String)>>,
    body: Option<String>,
    timeout_ms: Option<u64>,
    state: tauri::State<'_, AppState>,
) -> Result<PluginHttpResponseDto, String> {
    state
        .plugin_http_request(method, url, headers, body, timeout_ms)
        .await
}
