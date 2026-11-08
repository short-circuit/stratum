//! Filesystem ⇄ SQLite sync helper re-exports.
//!
//! These are pure helpers (no IPC) used from lib.rs, the block adapter, the
//! import adapter and the file watcher. They are re-exported verbatim from
//! [`stratum_app::page`] so existing `commands::page::*` call sites keep working.

pub use stratum_app::page::{
    heal_missing_page_links, reconcile_page_links, sync_filesystem_to_db, sync_page_from_disk,
};

#[cfg(not(target_os = "android"))]
pub use stratum_app::page::handle_renamed_or_created;
