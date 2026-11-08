//! Task-marker toggle/clear commands.
//!
//! These are the only block mutations that rewrite the `.md` file and refresh
//! the IndexEngine; they preserve on-disk frontmatter and rebuild the page's
//! link rows so the graph stays convergent.

use crate::state::StratumService;
use uuid::Uuid;

impl StratumService {
    pub async fn toggle_block_marker(
        &self,
        page_path: String,
        block_id: String,
    ) -> Result<Option<String>, String> {
        let mut state = self.lock()?;
        let id = Uuid::parse_str(&block_id).map_err(|e| e.to_string())?;
        let store = state.get_store().map_err(|e| e.to_string())?;
        let blocks = store
            .get_blocks_by_page(&page_path)
            .map_err(|e| e.to_string())?;

        let mut tree = pkm_block::tree::BlockTree::new();
        for b in &blocks {
            tree.insert(b.clone());
        }
        let new_marker = pkm_block::ops::toggle_task(&mut tree, id).map_err(|e| e.to_string())?;

        // Wrap SQLite operations in an explicit transaction
        store.execute_batch("BEGIN").map_err(|e| e.to_string())?;
        let result = (|| -> Result<(), String> {
            store
                .delete_blocks_by_page(&page_path)
                .map_err(|e| e.to_string())?;
            let all_blocks: Vec<pkm_block::Block> = tree.all_blocks().cloned().collect();
            for b in &all_blocks {
                store
                    .insert_block(b, &page_path)
                    .map_err(|e| e.to_string())?;
            }
            // Rebuild the `links` table for this page. `delete_blocks_by_page` above
            // cascades to the page's `links` rows (FK ON), so without reconciliation
            // the graph/backlink view of this page's wiki-links would go stale.
            // The `links` table is now the authoritative source for graph edges.
            crate::page::reconcile_page_links(&store, &page_path, &all_blocks)?;
            Ok(())
        })();
        match result {
            Ok(()) => store.execute_batch("COMMIT").map_err(|e| e.to_string())?,
            Err(e) => {
                store.execute_batch("ROLLBACK").ok();
                return Err(e);
            }
        }

        let all_blocks = store
            .get_blocks_by_page(&page_path)
            .map_err(|e| e.to_string())?;
        let body = pkm_markdown::block_parser::serialize_blocks(&all_blocks);
        let full_path = state.vault_path.join(&page_path);
        let existing = std::fs::read_to_string(&full_path).unwrap_or_default();
        let title = extract_title_from_frontmatter(&existing);

        // Rebuild the .md from the existing file so custom frontmatter (tags,
        // aliases, created/modified/dates, and any extra fields) is preserved —
        // not recreated from the title alone, which dropped every non-title field.
        let final_md = pkm_markdown::block_parser::assemble_blocks_markdown(
            &existing,
            &body,
            title.as_deref(),
        );
        std::fs::write(&full_path, &final_md).map_err(|e| e.to_string())?;

        // Notify auto-commit engine
        state.record_change(&page_path);

        let block_index = state.ensure_block_index()?;
        for b in &all_blocks {
            block_index
                .index_block(b, &page_path)
                .map_err(|e| e.to_string())?;
        }
        block_index.flush().map_err(|e| e.to_string())?;
        // Drop BlockIndex writer before IndexEngine acquires its own (same Tantivy dir)
        drop(state.block_index.take());

        // Upsert page with frontmatter preservation so tags/aliases are not lost
        let mut page = pkm_block::Page::new(full_path, &state.vault_path);
        if let Some(t) = title {
            page.frontmatter.title = Some(t);
        }
        if let Ok(Some(existing_fm)) = store.get_page(&page.rel_path.to_string_lossy()) {
            if !existing_fm.tags.is_empty() {
                page.frontmatter.tags = existing_fm.tags;
            }
            if !existing_fm.aliases.is_empty() {
                page.frontmatter.aliases = existing_fm.aliases;
            }
            if existing_fm.created.is_some() {
                page.frontmatter.created = existing_fm.created;
            }
            if existing_fm.modified.is_some() {
                page.frontmatter.modified = existing_fm.modified;
            }
            if !existing_fm.extra.is_empty() {
                page.frontmatter.extra = existing_fm.extra;
            }
        }
        store.upsert_page(&page).map_err(|e| e.to_string())?;

        // Keep IndexEngine in sync with the written file
        let vault_path = state.vault_path.clone();
        state
            .ensure_index()?
            .refresh_page(&page_path, &vault_path)
            .map_err(|e| format!("Index refresh failed: {}", e))?;

        Ok(new_marker.map(|m| m.as_str().to_string()))
    }

    pub async fn clear_block_marker(
        &self,
        page_path: String,
        block_id: String,
    ) -> Result<(), String> {
        let mut state = self.lock()?;
        let id = Uuid::parse_str(&block_id).map_err(|e| e.to_string())?;
        let store = state.get_store().map_err(|e| e.to_string())?;
        let mut blocks = store
            .get_blocks_by_page(&page_path)
            .map_err(|e| e.to_string())?;

        if let Some(block) = blocks.iter_mut().find(|b| b.id == id) {
            // Clear the task marker (and priority, which is marker-scoped) before
            // persisting — the previous code re-inserted the block unchanged, so
            // `clear_block_marker` only rewrote the file without actually clearing.
            block.marker = None;
            block.priority = None;
            // Wrap SQLite operation in a transaction
            store.execute_batch("BEGIN").map_err(|e| e.to_string())?;
            let result = (|| -> Result<(), String> {
                if let Some(block) = blocks.iter().find(|b| b.id == id) {
                    store
                        .insert_block(block, &page_path)
                        .map_err(|e| e.to_string())?;
                }
                // `INSERT OR REPLACE` deletes the conflicting row first, which with
                // FK cascade ON wipes this block's `links` rows. Rebuild the page's
                // `links` from the full block set so the graph/backlink view stays
                // correct (the `links` table is the authoritative graph edge source).
                crate::page::reconcile_page_links(&store, &page_path, &blocks)?;
                Ok(())
            })();
            match result {
                Ok(()) => store.execute_batch("COMMIT").map_err(|e| e.to_string())?,
                Err(e) => {
                    store.execute_batch("ROLLBACK").ok();
                    return Err(e);
                }
            }
        }

        let body = pkm_markdown::block_parser::serialize_blocks(&blocks);
        let full_path = state.vault_path.join(&page_path);
        let existing = std::fs::read_to_string(&full_path).unwrap_or_default();
        let title = extract_title_from_frontmatter(&existing);

        // Rebuild the .md from the existing file so custom frontmatter (tags,
        // aliases, created/modified, extra fields) is preserved — not recreated
        // from the title alone (same fix as toggle_block_marker).
        let final_md = pkm_markdown::block_parser::assemble_blocks_markdown(
            &existing,
            &body,
            title.as_deref(),
        );
        std::fs::write(&full_path, &final_md).map_err(|e| e.to_string())?;

        // Notify auto-commit engine
        state.record_change(&page_path);

        let block_index = state.ensure_block_index()?;
        for b in &blocks {
            block_index
                .index_block(b, &page_path)
                .map_err(|e| e.to_string())?;
        }
        block_index.flush().map_err(|e| e.to_string())?;
        // Drop BlockIndex writer before IndexEngine acquires its own (same Tantivy dir)
        drop(state.block_index.take());

        // Upsert page with frontmatter preservation so tags/aliases are not lost
        let mut page = pkm_block::Page::new(full_path, &state.vault_path);
        if let Some(t) = title {
            page.frontmatter.title = Some(t);
        }
        if let Ok(Some(existing_fm)) = store.get_page(&page.rel_path.to_string_lossy()) {
            if !existing_fm.tags.is_empty() {
                page.frontmatter.tags = existing_fm.tags;
            }
            if !existing_fm.aliases.is_empty() {
                page.frontmatter.aliases = existing_fm.aliases;
            }
            if existing_fm.created.is_some() {
                page.frontmatter.created = existing_fm.created;
            }
            if existing_fm.modified.is_some() {
                page.frontmatter.modified = existing_fm.modified;
            }
            if !existing_fm.extra.is_empty() {
                page.frontmatter.extra = existing_fm.extra;
            }
        }
        store.upsert_page(&page).map_err(|e| e.to_string())?;

        // Keep IndexEngine in sync with the written file
        let vault_path = state.vault_path.clone();
        state
            .ensure_index()?
            .refresh_page(&page_path, &vault_path)
            .map_err(|e| format!("Index refresh failed: {}", e))?;

        Ok(())
    }
}

fn extract_title_from_frontmatter(content: &str) -> Option<String> {
    let content = content.trim();
    if let Some(rest) = content.strip_prefix("---") {
        if let Some(end) = rest.find("---") {
            let frontmatter = &rest[..end];
            for line in frontmatter.lines() {
                if let Some(val) = line.strip_prefix("title:") {
                    return Some(val.trim().to_string());
                }
            }
        }
    }
    None
}
