//! File normalization command adapters over [`stratum_app::page`].

use crate::commands::TauriEventSink;
use stratum_app::StratumService;

/// Normalize a single .md file by parsing it back through the block parser and
/// re-serializing.
#[tauri::command]
pub async fn normalize_file(
    path: String,
    state: tauri::State<'_, StratumService>,
) -> Result<(), String> {
    state
        .normalize_file(path, &stratum_app::events::NoopSink)
        .await
}

/// Normalize all .md files in the vault. Progress is reported via
/// "reindex-progress" events.
#[tauri::command]
pub async fn normalize_all_files(
    app: tauri::AppHandle,
    state: tauri::State<'_, StratumService>,
) -> Result<usize, String> {
    let sink = TauriEventSink(app);
    state.normalize_all_files(&sink).await
}
