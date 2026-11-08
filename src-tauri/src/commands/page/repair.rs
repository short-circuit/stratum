//! Database repair command adapter over [`stratum_app::page`].

pub use stratum_app::page::{get_blocks_heal_from_disk, repair_db_core};

use crate::commands::{ReindexResult, TauriEventSink};
use stratum_app::StratumService;

/// Reconcile every .md file on disk into SQLite and prune orphan rows.
#[tauri::command]
pub async fn repair_db_from_disk(
    app: tauri::AppHandle,
    state: tauri::State<'_, StratumService>,
) -> Result<ReindexResult, String> {
    let sink = TauriEventSink(app);
    state.repair_db_from_disk(&sink).await
}
