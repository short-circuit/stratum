//! Tauri adapter for per-page PDF export.

pub use stratum_app::export::pdf::{
    chromium_candidates, export_pdf_core, find_chromium, PdfExportResult,
};

use crate::commands::vault::AppState;

#[tauri::command]
pub async fn export_pdf(
    page_path: String,
    output_path: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<PdfExportResult, String> {
    state.export_pdf(page_path, output_path).await
}
