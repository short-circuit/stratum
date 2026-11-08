//! Tauri adapters for export commands: static site (HTML), JSON, per-page PDF
//! (headless Chromium), and whole-vault ZIP. The PDF and ZIP adapters live in
//! the [`pdf`] and [`zip`] submodules.

pub mod pdf;
pub mod zip;

pub use stratum_app::export::{export_html_core, export_json_core, ExportResult};

use crate::commands::vault::AppState;

#[tauri::command]
pub async fn export_html(
    output_dir: String,
    state: tauri::State<'_, AppState>,
) -> Result<ExportResult, String> {
    state.export_html(output_dir).await
}

#[tauri::command]
pub async fn export_json(
    output_dir: String,
    state: tauri::State<'_, AppState>,
) -> Result<ExportResult, String> {
    state.export_json(output_dir).await
}
