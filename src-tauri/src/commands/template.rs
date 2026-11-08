//! Tauri adapters for template management commands.

pub use stratum_app::template::TemplateDto;

use crate::commands::vault::AppState;

#[tauri::command]
pub async fn list_templates(state: tauri::State<'_, AppState>) -> Result<Vec<TemplateDto>, String> {
    state.list_templates().await
}

#[tauri::command]
pub async fn save_template(
    name: String,
    content: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    state.save_template(name, content).await
}

#[tauri::command]
pub async fn apply_template(
    template_name: String,
    target_page: String,
    variables: Vec<(String, String)>,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    state
        .apply_template(template_name, target_page, variables)
        .await
}

#[tauri::command]
pub async fn install_starter_templates(
    names: Option<Vec<String>>,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<String>, String> {
    state.install_starter_templates(names).await
}

#[tauri::command]
pub async fn install_starter_sample(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<String>, String> {
    state.install_starter_sample().await
}
