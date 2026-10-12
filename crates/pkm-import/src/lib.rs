//! # pkm-import — One-click vault importers
//!
//! Imports notes from Obsidian, Logseq, or a generic Markdown folder into a
//! Stratum vault. The operation is **idempotent** (re-importing produces only
//! duplicates) and **reversible** (content is only ever added under a single
//! `imported/<name>/` subdirectory; nothing in the vault is modified or
//! deleted, and the source is never touched).
//!
//! Typical flow:
//! 1. [`preview_import`] — compute a [`ImportPreview`] showing pages/assets,
//!    duplicates, and conflicts, *without writing anything*.
//! 2. [`import_vault`] — run the same plan and write only the `new` items
//!    (honoring [`ImportOptions::dry_run`]).
//!
//! Format is auto-detected via [`detect_format`] unless overridden. Per-file
//! failures are collected into [`ImportReport::errors`] rather than aborting the
//! whole import.

pub mod convert;
pub mod detect;
pub mod generic;
pub mod import;
pub mod logseq;
pub mod obsidian;
pub mod report;

pub use convert::{sanitize_subdir, Converted};
pub use detect::detect_format;
pub use import::{import_vault, preview_import};
pub use report::{
    ImportAsset, ImportFormat, ImportOptions, ImportPreview, ImportPreviewItem, ImportReport,
};
