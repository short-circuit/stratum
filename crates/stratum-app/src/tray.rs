//! Tray status + Quick Capture persistence.
//!
//! These are the platform-independent halves of the tray feature: reading the
//! configured status and persisting a captured note. Hosts keep the actual
//! tray icon / menu / global-shortcut glue.

use crate::state::StratumService;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrayStatusDto {
    pub enabled: bool,
    pub close_to_tray: bool,
    pub shortcut: String,
    pub capture_target: String,
    pub inbox_path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QuickCaptureResultDto {
    pub page_path: String,
    pub block_id: String,
}

/// Desktop tray + close-to-tray effective value. Always false on mobile.
pub fn get_effective_close_to_tray(config: &pkm_core::TrayConfig, desktop: bool) -> bool {
    desktop && config.enabled && config.close_to_tray
}

impl StratumService {
    /// Read the current tray/quick-capture status for the frontend.
    ///
    /// Returns desktop defaults when the vault has no config. On mobile the tray
    /// is reported disabled but the configured shortcut/capture values are
    /// returned.
    pub fn get_tray_status(&self) -> Result<TrayStatusDto, String> {
        let state = self.lock()?;
        let config = load_config(&state)?;
        let desktop = !(cfg!(target_os = "android") || cfg!(target_os = "ios"));
        Ok(TrayStatusDto {
            enabled: if desktop { config.tray.enabled } else { false },
            close_to_tray: get_effective_close_to_tray(&config.tray, desktop),
            shortcut: config.tray.quick_capture_shortcut.clone(),
            capture_target: match config.tray.capture_target {
                pkm_core::CaptureTarget::Inbox => "inbox".into(),
                pkm_core::CaptureTarget::Journal => "journal".into(),
            },
            inbox_path: config.tray.inbox_path.clone(),
        })
    }

    /// Capture a note as a block into today's journal (or the configured inbox)
    /// and persist it to disk + search index.
    ///
    /// The method does its own persistence (store insert, `.md` write and Tantivy
    /// index update) so the capture is durable even if the file watcher is off.
    pub async fn quick_capture_save(&self, text: String) -> Result<QuickCaptureResultDto, String> {
        let mut state = self.lock()?;
        let config = load_config(&state)?;
        let vault = state.vault_path.clone();
        let store = state.get_store().map_err(|e| e.to_string())?;

        let page_path = match config.tray.capture_target {
            pkm_core::CaptureTarget::Inbox => {
                let mut p = config.tray.inbox_path.clone();
                if !p.ends_with(".md") {
                    p.push_str(".md");
                }
                p
            }
            pkm_core::CaptureTarget::Journal => {
                let today = chrono::Local::now().format("%Y-%m-%d").to_string();
                format!("journals/{}.md", today)
            }
        };

        ensure_target_page(&store, &vault, &page_path, &config)?;

        let id = uuid::Uuid::new_v4();
        let block = pkm_block::Block::new(id, text.clone());
        store
            .insert_block(&block, &page_path)
            .map_err(|e| e.to_string())?;

        persist_blocks_to_disk(&store, &vault, &page_path)?;
        state.watcher_last_save = std::time::SystemTime::now();
        state.record_change(&page_path);

        let block_index = state.ensure_block_index()?;
        block_index
            .index_block(&block, &page_path)
            .map_err(|e| e.to_string())?;
        block_index.flush().map_err(|e| e.to_string())?;
        // Drop the Tantivy writer so the file watcher can acquire the directory lock.
        drop(state.block_index.take());

        Ok(QuickCaptureResultDto {
            page_path,
            block_id: id.to_string(),
        })
    }
}

/// Load the vault config, falling back to defaults (with the vault path set)
/// when no config file exists.
fn load_config(state: &crate::state::VaultState) -> Result<pkm_core::Config, String> {
    let config_path = state.vault_path.join(".pkm").join("config.toml");
    if config_path.exists() {
        pkm_core::Config::load(&config_path).map_err(|e| e.to_string())
    } else {
        Ok(pkm_core::Config {
            vault_path: state.vault_path.clone(),
            ..Default::default()
        })
    }
}

/// Ensure the capture target page exists on disk and is registered in SQLite.
///
/// Journal pages use the existing `ensure_today_journal_core` registration; an
/// inbox (or any custom) page is created with a title frontmatter if missing,
/// then synced from disk so it becomes queryable. Idempotent.
fn ensure_target_page(
    store: &pkm_block::BlockStore,
    vault: &Path,
    page_path: &str,
    config: &pkm_core::Config,
) -> Result<(), String> {
    let full = crate::page::resolve_safe_write_path(vault, page_path)?;

    if !full.exists() {
        let title = match config.tray.capture_target {
            pkm_core::CaptureTarget::Journal => chrono::Local::now().format("%Y-%m-%d").to_string(),
            pkm_core::CaptureTarget::Inbox => "Inbox".to_string(),
        };
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let content = format!("---\ntitle: {}\n---\n", title);
        std::fs::write(&full, &content).map_err(|e| e.to_string())?;
    }

    if config.tray.capture_target == pkm_core::CaptureTarget::Journal {
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        crate::page::ensure_today_journal_core(store, vault, &today).map(|_| ())
    } else {
        crate::page::sync_page_from_disk(store, page_path, vault, None).map(|_| ())
    }
}

/// Re-serialize all blocks for a page to disk, preserving every frontmatter field
/// from the existing file (same approach as `save_blocks`).
fn persist_blocks_to_disk(
    store: &pkm_block::BlockStore,
    vault: &Path,
    page_path: &str,
) -> Result<(), String> {
    let full = crate::page::resolve_safe_write_path(vault, page_path)?;
    let blocks = store
        .get_blocks_by_page(page_path)
        .map_err(|e| e.to_string())?;
    let body = pkm_markdown::block_parser::serialize_blocks(&blocks);
    let existing = std::fs::read_to_string(&full).unwrap_or_default();
    let markdown = pkm_markdown::block_parser::assemble_blocks_markdown(&existing, &body, None);

    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let temp = full.with_extension("md.tmp");
    std::fs::write(&temp, &markdown).map_err(|e| e.to_string())?;
    std::fs::rename(&temp, &full).map_err(|e| e.to_string())?;
    Ok(())
}
