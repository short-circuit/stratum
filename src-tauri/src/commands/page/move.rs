//! Page move/rename command adapter over [`stratum_app::page`].

pub use stratum_app::page::move_page_core;

use crate::commands::TauriEventSink;
use stratum_app::StratumService;

/// The single supported rename/move operation for pages.
#[tauri::command]
pub async fn move_page(
    old_path: String,
    new_path: String,
    app: tauri::AppHandle,
    state: tauri::State<'_, StratumService>,
) -> Result<stratum_app::page::PageDto, String> {
    let sink = TauriEventSink(app);
    state.move_page(old_path, new_path, &sink).await
}
