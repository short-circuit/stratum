//! Daily journal command adapter over [`stratum_app::page`].

pub use stratum_app::page::ensure_today_journal_core;

use stratum_app::StratumService;

/// Atomically ensure today's journal page exists.
#[tauri::command]
pub async fn ensure_today_journal(
    state: tauri::State<'_, StratumService>,
) -> Result<stratum_app::page::PageDto, String> {
    state.ensure_today_journal().await
}
