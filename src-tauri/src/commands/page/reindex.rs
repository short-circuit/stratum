//! Vault / page reindexing command adapters over [`stratum_app::page`].

use crate::commands::{ReindexResult, TauriEventSink};
use stratum_app::StratumService;

/// Re-sync every .md file from disk into SQLite.
#[tauri::command]
pub async fn reindex_vault(
    app: tauri::AppHandle,
    state: tauri::State<'_, StratumService>,
) -> Result<ReindexResult, String> {
    let sink = TauriEventSink(app);
    state.reindex_vault(&sink).await
}

/// Re-sync a single page from disk into SQLite.
#[tauri::command]
pub async fn reindex_page(
    path: String,
    state: tauri::State<'_, StratumService>,
) -> Result<ReindexResult, String> {
    state
        .reindex_page(path, &stratum_app::events::NoopSink)
        .await
}
