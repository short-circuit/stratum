//! Whiteboard panel data helpers for the service-backed provider.
//!
//! Rust allows a trait to be implemented for a type only once, so the
//! [`PanelData`] methods stay in [`super::app_panels`] and delegate here. Each
//! helper drives [`StratumService`] on the provider's Tokio runtime and maps the
//! service DTOs into the panel contract; the canvas crate supplies the
//! empty-board bytes used when creating a new board. Keeping the bodies here
//! leaves every provider source file under the repository length limit.

use stratum_canvas::{BoardDocument, Library};

use super::super::panels::{LibraryItemRow, WhiteboardRow};
use crate::provider::AppProvider;

/// List every whiteboard in the vault.
pub(super) fn whiteboards(provider: &AppProvider) -> Result<Vec<WhiteboardRow>, String> {
    let boards = provider
        .runtime
        .block_on(provider.service.list_whiteboards())?;
    Ok(boards
        .into_iter()
        .map(|board| WhiteboardRow {
            name: board.name,
            path: board.path,
            content: board.content,
        })
        .collect())
}

/// Load one whiteboard's raw document bytes.
pub(super) fn load_whiteboard(provider: &AppProvider, name: &str) -> Result<String, String> {
    provider
        .runtime
        .block_on(provider.service.load_whiteboard(name.to_string()))
}

/// Persist whiteboard bytes.
pub(super) fn save_whiteboard(
    provider: &AppProvider,
    name: &str,
    content: &str,
) -> Result<(), String> {
    provider.runtime.block_on(
        provider
            .service
            .save_whiteboard(name.to_string(), content.to_string()),
    )
}

/// Create an empty board and return its row.
pub(super) fn create_whiteboard(
    provider: &AppProvider,
    name: &str,
) -> Result<WhiteboardRow, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("whiteboard name is required".to_string());
    }
    let content = BoardDocument::empty()
        .save_string()
        .map_err(|error| error.to_string())?;
    provider.runtime.block_on(
        provider
            .service
            .save_whiteboard(name.to_string(), content.clone()),
    )?;
    Ok(WhiteboardRow::new(name, content))
}

/// Rename a whiteboard and move its file.
pub(super) fn rename_whiteboard(
    provider: &AppProvider,
    old_name: &str,
    new_name: &str,
) -> Result<(), String> {
    provider.runtime.block_on(
        provider
            .service
            .rename_whiteboard(old_name.to_string(), new_name.to_string()),
    )
}

/// Delete each named whiteboard.
pub(super) fn delete_whiteboards(provider: &AppProvider, names: Vec<String>) -> Result<(), String> {
    for name in names {
        provider
            .runtime
            .block_on(provider.service.delete_whiteboard(name))?;
    }
    Ok(())
}

/// Merge the personal and extra libraries into deduplicated panel rows.
pub(super) fn whiteboard_library(provider: &AppProvider) -> Result<Vec<LibraryItemRow>, String> {
    let personal = provider.runtime.block_on(provider.service.load_library())?;
    let extra = provider
        .runtime
        .block_on(provider.service.load_extra_libraries())?;
    let mut rows: Vec<LibraryItemRow> = Vec::new();
    for json in [personal, extra] {
        let Ok(library) = Library::load(json.as_bytes()) else {
            continue;
        };
        for item in library.items() {
            if rows.iter().any(|row| row.id == item.id()) {
                continue;
            }
            rows.push(LibraryItemRow::from_item(item));
        }
    }
    Ok(rows)
}

/// Persist the personal library JSON.
pub(super) fn save_whiteboard_library(provider: &AppProvider, json: &str) -> Result<(), String> {
    provider
        .runtime
        .block_on(provider.service.save_library(json.to_string()))
}

/// Write exported canvas bytes and return the vault-relative path.
pub(super) fn write_canvas_export(
    provider: &AppProvider,
    name: &str,
    extension: &str,
    bytes: Vec<u8>,
) -> Result<String, String> {
    provider
        .runtime
        .block_on(provider.service.write_whiteboard_export(
            name.to_string(),
            extension.to_string(),
            bytes,
        ))
}
