//! One-click import service layer.
//!
//! Wraps the [`pkm_import`] crate. Business logic (format detection,
//! conversion, dedup, writes) lives entirely in the crate; these methods only
//! adapt path/state plumbing. The import always targets the **active vault** —
//! any caller-provided `ImportOptions.destination` is overridden with the vault
//! path, so imported files land inside the current vault's `imported/<name>/`
//! subdirectory.

use crate::state::StratumService;
use std::path::PathBuf;

pub use pkm_import::{
    ImportAsset, ImportFormat, ImportOptions, ImportPreview, ImportPreviewItem, ImportReport,
};

impl StratumService {
    /// Preview what importing `path` would do. Never writes to the source or the
    /// destination; returns counts, rel-paths, duplicates and conflicts.
    pub async fn preview_import(
        &self,
        path: String,
        opts: ImportOptions,
    ) -> Result<ImportPreview, String> {
        let source = PathBuf::from(&path);
        pkm_import::preview_import(&source, &opts).map_err(|e| e.to_string())
    }

    /// Import `path` into the active vault per `opts` (respecting `opts.dry_run`).
    ///
    /// After a non-dry run the newly written files are synced into SQLite +
    /// Tantivy via [`crate::page::sync_filesystem_to_db`], the established
    /// "imported a new dataset" path, so the new pages appear in the page list.
    pub async fn import_vault(
        &self,
        path: String,
        mut opts: ImportOptions,
    ) -> Result<ImportReport, String> {
        let (vault_path, db_path) = {
            let s = self.lock()?;
            (s.vault_path.clone(), s.db_path.clone())
        };

        // Import always targets the active vault; caller destination is overridden.
        opts.destination = vault_path.clone();

        let source = PathBuf::from(&path);
        let report = pkm_import::import_vault(&source, &opts).map_err(|e| e.to_string())?;

        if !opts.dry_run {
            crate::page::sync_filesystem_to_db(&vault_path, &db_path).map_err(|e| e.to_string())?;
        }

        Ok(report)
    }
}
