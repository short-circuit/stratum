//! One-click import command layer (thin glue).
//!
//! Wraps the [`pkm_import`] crate into Tauri commands. Business logic (format
//! detection, conversion, dedup, writes) lives entirely in the crate; these
//! handlers only adapt path/`State` plumbing. The import always targets the
//! **active vault** — any caller-provided `ImportOptions.destination` is
//! overridden with the vault path, so imported files land inside the current
//! vault's `imported/<name>/` subdirectory.

use pkm_import::{ImportOptions, ImportPreview, ImportReport};
use std::path::PathBuf;
use tauri::State;

use crate::commands::vault::AppState;

/// Preview what importing `path` would do. Never writes to the source or the
/// destination; returns counts, rel-paths, duplicates and conflicts.
#[tauri::command]
pub async fn preview_import(path: String, opts: ImportOptions) -> Result<ImportPreview, String> {
    let source = PathBuf::from(&path);
    pkm_import::preview_import(&source, &opts).map_err(|e| e.to_string())
}

/// Import `path` into the active vault per `opts` (respecting `opts.dry_run`).
///
/// After a non-dry run the newly written files are synced into SQLite + Tantivy
/// via [`crate::commands::page::sync_filesystem_to_db`], the established
/// "imported a new dataset" path, so the new pages appear in the page list.
#[tauri::command]
pub async fn import_vault(
    path: String,
    mut opts: ImportOptions,
    state: State<'_, AppState>,
) -> Result<ImportReport, String> {
    let (vault_path, db_path) = {
        let s = state.lock().map_err(|e| e.to_string())?;
        (s.vault_path.clone(), s.db_path.clone())
    };

    // Import always targets the active vault; caller destination is overridden.
    opts.destination = vault_path.clone();

    let source = PathBuf::from(&path);
    let report = pkm_import::import_vault(&source, &opts).map_err(|e| e.to_string())?;

    if !opts.dry_run {
        crate::commands::page::sync_filesystem_to_db(&vault_path, &db_path)
            .map_err(|e| e.to_string())?;
    }

    Ok(report)
}
