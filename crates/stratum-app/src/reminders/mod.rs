//! Block reminder commands.
//!
//! Provides `list_reminders` to list all blocks carrying a `due` property,
//! plus `snooze_reminder` / `clear_reminder` to mutate that `due` property and
//! persist the change through the full block-save pipeline (SQLite + disk).

pub mod scheduler;

pub use scheduler::{DueReminder, NotifyFn, NowFn, ReminderScheduler};

use crate::state::{StratumService, VaultState};
use pkm_block::BlockStore;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A serializable reminder delivered to the frontend.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReminderDto {
    pub block_id: String,
    pub page_path: String,
    pub content: String,
    /// RFC3339 UTC due timestamp.
    pub due: String,
    pub marker: Option<String>,
    pub priority: Option<String>,
    pub page_title: Option<String>,
}

impl StratumService {
    /// List all blocks that carry a `due` property, ordered by due ascending.
    ///
    /// The DTO stays pure (no overdue computation) — the frontend derives
    /// overdue/today/upcoming groupings from the due timestamp.
    pub async fn list_reminders(&self) -> Result<Vec<ReminderDto>, String> {
        let state = self.lock()?;
        let store = state.get_store().map_err(|e| e.to_string())?;

        let mut reminders = Vec::new();
        for (block, page_path) in store.find_due_blocks().map_err(|e| e.to_string())? {
            let Some(due) = block.due() else {
                continue;
            };
            let page_title = store
                .get_page(&page_path)
                .ok()
                .flatten()
                .and_then(|fm| fm.title);
            reminders.push(ReminderDto {
                block_id: block.id.to_string(),
                page_path,
                content: block.content,
                due: due.to_rfc3339(),
                marker: block.marker.map(|m| m.as_str().to_string()),
                priority: block.priority.map(|p| p.as_str().to_string()),
                page_title,
            });
        }
        reminders.sort_by(|a, b| a.due.cmp(&b.due));
        Ok(reminders)
    }

    /// Push a reminder's `due` time forward by `minutes` (10 or 60).
    pub async fn snooze_reminder(
        &self,
        page_path: String,
        block_id: String,
        minutes: u32,
    ) -> Result<(), String> {
        let mut state = self.lock()?;
        let id = Uuid::parse_str(&block_id).map_err(|e| format!("Invalid block id: {e}"))?;
        let store = state.get_store().map_err(|e| e.to_string())?;

        let block = store.get_block(id).map_err(|e| e.to_string())?;
        let now = chrono::Utc::now();
        let updated = block.with_due(now + chrono::Duration::minutes(minutes as i64));

        store.update_block(&updated).map_err(|e| e.to_string())?;
        persist_page(&mut state, &store, &page_path)?;
        Ok(())
    }

    /// Remove the `due` property from a block, clearing its reminder.
    pub async fn clear_reminder(&self, page_path: String, block_id: String) -> Result<(), String> {
        let mut state = self.lock()?;
        let id = Uuid::parse_str(&block_id).map_err(|e| format!("Invalid block id: {e}"))?;
        let store = state.get_store().map_err(|e| e.to_string())?;

        let mut block = store.get_block(id).map_err(|e| e.to_string())?;
        block.properties.remove("due");
        store.update_block(&block).map_err(|e| e.to_string())?;
        persist_page(&mut state, &store, &page_path)?;
        Ok(())
    }
}

/// Re-serialize the page's blocks to disk, preserving frontmatter, and record
/// the change for the auto-commit engine.
fn persist_page(state: &mut VaultState, store: &BlockStore, page_path: &str) -> Result<(), String> {
    let full_path = state.vault_path.join(page_path);
    let all_blocks = store
        .get_blocks_by_page(page_path)
        .map_err(|e| e.to_string())?;
    let body = pkm_markdown::block_parser::serialize_blocks(&all_blocks);
    let existing = std::fs::read_to_string(&full_path).unwrap_or_default();
    let markdown = pkm_markdown::block_parser::assemble_blocks_markdown(&existing, &body, None);

    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let temp_path = full_path.with_extension("md.tmp");
    std::fs::write(&temp_path, &markdown).map_err(|e| e.to_string())?;
    std::fs::rename(&temp_path, &full_path).map_err(|e| e.to_string())?;
    state.watcher_last_save = std::time::SystemTime::now();
    state.record_change(page_path);
    Ok(())
}
