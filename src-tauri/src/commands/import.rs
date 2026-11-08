//! One-click import command adapters over [`stratum_app::import`].

pub use pkm_import::{ImportOptions, ImportPreview, ImportReport};

use stratum_app::StratumService;

#[tauri::command]
pub async fn preview_import(
    path: String,
    opts: ImportOptions,
    state: tauri::State<'_, StratumService>,
) -> Result<ImportPreview, String> {
    state.preview_import(path, opts).await
}

#[tauri::command]
pub async fn import_vault(
    path: String,
    opts: ImportOptions,
    state: tauri::State<'_, StratumService>,
) -> Result<ImportReport, String> {
    state.import_vault(path, opts).await
}
