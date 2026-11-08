//! Tauri adapter for whole-vault ZIP export.

pub use stratum_app::export::zip::{export_zip_core, ZipExportResult};

use crate::commands::vault::AppState;

#[tauri::command]
pub async fn export_zip(
    output_path: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<ZipExportResult, String> {
    state.export_zip(output_path).await
}
