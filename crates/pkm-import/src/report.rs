//! Wire DTOs for the `pkm-import` crate: formats, options, previews, and reports.
//!
//! These types form the single shared contract between the importer logic in
//! this crate, the thin Tauri command layer, and the TypeScript frontend — so
//! the frontend can render a preview before anything is written, and only add
//! files, never mutate or delete.
//!
//! All struct fields are serialized verbatim (snake_case, matching the Rust
//! field names) with no `rename_all`, mirroring the convention used by every
//! other DTO in the repository so the TypeScript mirror in `src/lib/types.ts`
//! stays in lockstep.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Supported source vault formats for one-click import.
///
/// `Auto` asks the importer to detect the format from the source directory's
/// structure (see [`crate::detect::detect_format`]); the other variants force
/// a specific importer regardless of what detection would say.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ImportFormat {
    #[default]
    Auto,
    Obsidian,
    Logseq,
    Generic,
}

impl ImportFormat {
    /// Human-facing label used in error messages and reports.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Obsidian => "obsidian",
            Self::Logseq => "logseq",
            Self::Generic => "generic",
        }
    }
}

/// Options controlling an import operation.
///
/// This is what the caller provides; the active vault `destination` is always
/// overridden by the Tauri layer to the currently open vault.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportOptions {
    /// Vault root to import INTO.
    #[serde(default)]
    pub destination: PathBuf,
    /// Format to assume (default [`ImportFormat::Auto`] → detect).
    #[serde(default)]
    pub format: ImportFormat,
    /// When true, compute a preview only and write nothing (default true).
    #[serde(default = "default_true")]
    pub dry_run: bool,
    /// Destination subdirectory for imported content (default
    /// `imported/<sanitized-source-name>`).
    #[serde(default)]
    pub subdir: Option<String>,
}

fn default_true() -> bool {
    true
}

impl Default for ImportOptions {
    fn default() -> Self {
        Self {
            destination: PathBuf::new(),
            format: ImportFormat::Auto,
            dry_run: true,
            subdir: None,
        }
    }
}

/// One planned page in an import preview.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportPreviewItem {
    /// Path inside the source vault.
    pub source_rel: String,
    /// Destination-relative target (e.g. `imported/obsidian-sample/Welcome.md`).
    pub rel_path: String,
    /// Best-effort page title (frontmatter title, falling back to the slug).
    pub title: String,
    /// `new` | `duplicate` | `conflict`.
    pub status: String,
    /// Size of the content that would be written, in bytes.
    pub size_bytes: u64,
}

/// One planned asset (binary attachment) in an import preview.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportAsset {
    /// Path inside the source vault (under `assets/`).
    pub source_rel: String,
    /// Destination-relative target (e.g. `imported/obsidian-sample/assets/logo.png`).
    pub rel_path: String,
    /// `new` | `duplicate` | `conflict`.
    pub status: String,
    /// Size of the file in bytes.
    pub size_bytes: u64,
}

/// Full import preview, rendered by the frontend before any write happens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportPreview {
    /// The source path the user selected.
    pub source_path: String,
    /// The source directory's basename.
    pub source_name: String,
    /// The (possibly detected) format.
    pub format: ImportFormat,
    /// Planned pages.
    pub pages: Vec<ImportPreviewItem>,
    /// Planned assets.
    pub assets: Vec<ImportAsset>,
    /// Total number of planned pages.
    pub total_pages: usize,
    /// Total number of planned assets.
    pub total_assets: usize,
    /// Destination-relative paths skipped because the destination already has
    /// byte-identical content.
    pub duplicates: Vec<String>,
    /// Destination-relative paths skipped because the destination exists with
    /// different content (never overwritten).
    pub conflicts: Vec<String>,
}

/// Result of an import run (or a dry-run, which reports what *would* happen).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportReport {
    /// The source path the user selected.
    pub source_path: String,
    /// The source directory's basename.
    pub source_name: String,
    /// The (possibly detected) format.
    pub format: ImportFormat,
    /// Number of pages written (0 when dry_run).
    pub pages_imported: usize,
    /// Number of assets written (0 when dry_run).
    pub assets_imported: usize,
    /// Number of pages/assets skipped because they already existed identically.
    pub duplicates_skipped: usize,
    /// Number of pages/assets skipped because a conflicting file already exists.
    pub conflicts_skipped: usize,
    /// Whether this was a dry run (no writes).
    pub dry_run: bool,
    /// Per-file failures collected during the run (never fatal).
    pub errors: Vec<String>,
    /// Destination-relative paths written (empty when dry_run).
    pub imported_paths: Vec<String>,
}
