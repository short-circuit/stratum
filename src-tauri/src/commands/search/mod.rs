//! Tauri adapters for search and backlink commands.

pub mod dto;

pub use stratum_app::search::{
    build_backlink_snippet_from_store, AutocompleteItem, BacklinkContextDto, BacklinkDto,
    BacklinkSnippetDto, ConnectionSuggestion, SearchResultDto, SearchResultsDto,
};

use crate::commands::vault::AppState;
use crate::commands::TauriEventSink;

#[tauri::command]
pub async fn rebuild_search_index(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    state.rebuild_search_index(&TauriEventSink(app)).await
}

#[tauri::command]
pub async fn search_blocks(
    query: String,
    limit: Option<usize>,
    state: tauri::State<'_, AppState>,
) -> Result<SearchResultsDto, String> {
    state.search_blocks(query, limit).await
}

#[tauri::command]
pub async fn get_page_backlinks(
    page_path: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<BacklinkDto>, String> {
    state.get_page_backlinks(page_path).await
}

#[tauri::command]
pub async fn get_backlinks(
    block_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<SearchResultsDto, String> {
    state.get_backlinks(block_id).await
}

#[tauri::command]
pub async fn autocomplete(
    query: String,
    kind: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<AutocompleteItem>, String> {
    state.autocomplete(query, kind).await
}

#[tauri::command]
pub async fn get_backlink_snippet(
    note_id: String,
    backlink_ref: String,
    state: tauri::State<'_, AppState>,
) -> Result<BacklinkSnippetDto, String> {
    state.get_backlink_snippet(note_id, backlink_ref).await
}

#[tauri::command]
pub async fn get_backlink_context(
    target_page: String,
    current_page: String,
    state: tauri::State<'_, AppState>,
) -> Result<Option<BacklinkContextDto>, String> {
    state.get_backlink_context(target_page, current_page).await
}

#[tauri::command]
pub async fn suggest_connections(
    page_path: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<ConnectionSuggestion>, String> {
    state.suggest_connections(page_path).await
}

#[tauri::command]
pub async fn search_by_tag(
    tag: String,
    state: tauri::State<'_, AppState>,
) -> Result<SearchResultsDto, String> {
    state.search_by_tag(tag).await
}
