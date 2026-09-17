//! Block insertion helpers for rendered dictation memos.
//!
//! Mirrors `save_blocks`' atomic flow: parse memo markdown, append (or replace)
//! blocks inside a transaction, then reindex the page and invalidate the graph
//! cache. Consumed by the transcription and speaker-assignment commands.

use crate::commands::vault::VaultState;
// ---------------------------------------------------------------------------
// Block insertion helpers (mirror save_blocks' atomic flow)
// ---------------------------------------------------------------------------

/// Parse memo markdown into blocks and append them to the page.
pub fn insert_memo_blocks(
    state: &mut VaultState,
    page_path: &str,
    markdown: &str,
) -> Result<Vec<String>, String> {
    let store = state.get_store().map_err(|e| e.to_string())?;
    let blocks = parse_blocks(markdown)?;

    // Anchor: the current last block of the page (if any).
    let existing = store
        .get_blocks_by_page(page_path)
        .map_err(|e| e.to_string())?;
    let anchor = existing.last().map(|b| b.id);

    store.execute_batch("BEGIN").map_err(|e| e.to_string())?;
    let result = (|| -> Result<Vec<String>, String> {
        let mut ids = Vec::new();
        let mut prev = anchor;
        for mut block in blocks {
            block.left_id = prev;
            let id = block.id;
            store
                .insert_block(&block, page_path)
                .map_err(|e| e.to_string())?;
            ids.push(id.to_string());
            prev = Some(id);
        }
        Ok(ids)
    })();
    let ids = match result {
        Ok(ids) => {
            store.execute_batch("COMMIT").map_err(|e| e.to_string())?;
            ids
        }
        Err(e) => {
            store.execute_batch("ROLLBACK").ok();
            return Err(e);
        }
    };
    refresh_page_index(state, page_path)?;
    Ok(ids)
}

/// Replace a memo's blocks (after rename) with re-rendered ones.
pub fn replace_memo_blocks(
    state: &mut VaultState,
    session: &crate::commands::vault::DictationSession,
    markdown: &str,
) -> Result<Vec<String>, String> {
    let store = state.get_store().map_err(|e| e.to_string())?;

    // Anchor = the block before the first memo block (its left_id).
    let first_id = session
        .inserted_block_ids
        .first()
        .and_then(|s| uuid::Uuid::parse_str(s).ok());
    let anchor = first_id
        .and_then(|id| store.get_block(id).ok())
        .and_then(|b| b.left_id);

    store.execute_batch("BEGIN").map_err(|e| e.to_string())?;
    let result = (|| -> Result<Vec<String>, String> {
        for id_str in &session.inserted_block_ids {
            if let Ok(id) = uuid::Uuid::parse_str(id_str) {
                store.delete_block(id).map_err(|e| e.to_string())?;
            }
        }
        let mut ids = Vec::new();
        let mut prev = anchor;
        for mut block in parse_blocks(markdown)? {
            block.left_id = prev;
            let id = block.id;
            store
                .insert_block(&block, &session.page_path)
                .map_err(|e| e.to_string())?;
            ids.push(id.to_string());
            prev = Some(id);
        }
        Ok(ids)
    })();
    let ids = match result {
        Ok(ids) => {
            store.execute_batch("COMMIT").map_err(|e| e.to_string())?;
            ids
        }
        Err(e) => {
            store.execute_batch("ROLLBACK").ok();
            return Err(e);
        }
    };
    refresh_page_index(state, &session.page_path)?;
    Ok(ids)
}

/// Parse markdown into fresh blocks (block IDs regenerated on parse).
pub fn parse_blocks(markdown: &str) -> Result<Vec<pkm_block::Block>, String> {
    let (_, _, blocks) = pkm_markdown::block_parser::parse_document(markdown);
    Ok(blocks)
}

/// Reindex a page after block changes (same as save_blocks tail).
pub fn refresh_page_index(state: &mut VaultState, page_path: &str) -> Result<(), String> {
    state.record_change(page_path);
    let store = state.get_store().map_err(|e| e.to_string())?;
    let blocks = store
        .get_blocks_by_page(page_path)
        .map_err(|e| e.to_string())?;
    let block_index = state.ensure_block_index()?;
    for block in blocks {
        block_index
            .index_block(&block, page_path)
            .map_err(|e| e.to_string())?;
    }
    block_index.flush().map_err(|e| e.to_string())?;
    drop(state.block_index.take());
    let vault_path = state.vault_path.clone();
    state
        .ensure_index()?
        .refresh_page(page_path, &vault_path)
        .map_err(|e| format!("Index refresh failed: {e}"))?;
    crate::commands::graph::invalidate_graph_cache();
    Ok(())
}
