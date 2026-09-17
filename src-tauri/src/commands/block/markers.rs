//! Block marker toggling (TODO/DOING/DONE) and marker clearing.

use crate::commands::vault::AppState;
use uuid::Uuid;

// update_block, delete_block, and insert_block operate solely on SQLite and
// do NOT write to disk or trigger IndexEngine refresh.  The next save_blocks
// call (which always follows these operations in the editing flow) will
// re-serialize the full page to disk and call refresh_page, keeping
// everything consistent.

#[tauri::command]
pub async fn toggle_block_marker(
    page_path: String,
    block_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<Option<String>, String> {
    let mut state = state.lock().map_err(|e| e.to_string())?;
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
        for b in tree.all_blocks() {
            store
                .insert_block(b, &page_path)
                .map_err(|e| e.to_string())?;
        }
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

    let final_md = if let Some(t) = &title {
        format!("---\ntitle: {t}\n---\n\n{body}")
    } else {
        body
    };
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

#[tauri::command]
pub async fn clear_block_marker(
    page_path: String,
    block_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let mut state = state.lock().map_err(|e| e.to_string())?;
    let id = Uuid::parse_str(&block_id).map_err(|e| e.to_string())?;
    let store = state.get_store().map_err(|e| e.to_string())?;
    let mut blocks = store
        .get_blocks_by_page(&page_path)
        .map_err(|e| e.to_string())?;

    if let Some(block) = blocks.iter_mut().find(|b| b.id == id) {
        // Wrap SQLite operation in a transaction
        store.execute_batch("BEGIN").map_err(|e| e.to_string())?;
        let result = (|| -> Result<(), String> {
            store
                .insert_block(block, &page_path)
                .map_err(|e| e.to_string())?;
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

    let final_md = if let Some(t) = &title {
        format!("---\ntitle: {t}\n---\n\n{body}")
    } else {
        body
    };
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
