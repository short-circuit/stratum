//! Page management command adapters.
//!
//! Thin adapters over [`stratum_app::page`]. Free helpers and DTOs keep their
//! original `commands::page::*` paths for cross-module callers (lib.rs, tray,
//! import, block).

mod journal;
mod r#move;
mod normalize;
mod reindex;
mod repair;
mod sync;

pub use journal::*;
pub use normalize::*;
pub use r#move::*;
pub use reindex::*;
pub use repair::*;
pub use sync::*;

pub use stratum_app::page::{
    get_file_mtime, resolve_safe_path, resolve_safe_write_path, PageDto, PageListDto,
};

use stratum_app::StratumService;

#[tauri::command]
pub async fn list_pages(state: tauri::State<'_, StratumService>) -> Result<PageListDto, String> {
    state.list_pages().await
}

#[tauri::command]
pub async fn open_page(
    path: String,
    state: tauri::State<'_, StratumService>,
) -> Result<PageDto, String> {
    state.open_page(path).await
}

#[tauri::command]
pub async fn save_page(
    path: String,
    content: String,
    state: tauri::State<'_, StratumService>,
) -> Result<(), String> {
    state.save_page(path, content).await
}

#[tauri::command]
pub async fn create_page(
    path: String,
    title: Option<String>,
    state: tauri::State<'_, StratumService>,
) -> Result<PageDto, String> {
    state.create_page(path, title).await
}

#[tauri::command]
pub async fn delete_page(
    path: String,
    state: tauri::State<'_, StratumService>,
) -> Result<(), String> {
    state.delete_page(path).await
}
