//! Kanban board commands.
//!
//! Provides `get_kanban_blocks` to query all blocks with task markers
//! (TODO/DOING/DONE/NOW/LATER/WAITING/CANCELLED) across all pages,
//! and `create_kanban_block` to create a new block with a marker
//! on today's journal page.

use crate::commands::vault::AppState;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KanbanBlockDto {
    pub id: String,
    pub content: String,
    pub parent_id: Option<String>,
    pub left_id: Option<String>,
    pub properties: Vec<(String, String)>,
    pub marker: Option<String>,
    pub priority: Option<String>,
    pub collapsed: bool,
    pub heading_level: Option<u8>,
    pub page_path: String,
    pub page_title: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KanbanDataDto {
    pub blocks: Vec<KanbanBlockDto>,
}

/// Query all blocks with task markers across all pages.
#[tauri::command]
pub async fn get_kanban_blocks(state: tauri::State<'_, AppState>) -> Result<KanbanDataDto, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let store = pkm_block::BlockStore::open(&state.db_path).map_err(|e| e.to_string())?;

    // Use the find_blocks_by_markers method
    let markers = &[
        "TODO",
        "DOING",
        "DONE",
        "NOW",
        "LATER",
        "WAITING",
        "CANCELLED",
    ];
    let results = store
        .find_blocks_by_markers(markers)
        .map_err(|e| e.to_string())?;

    let mut blocks = Vec::new();
    for (block, page_path) in results {
        // Resolve page title from the store
        let page_title = store
            .get_page(&page_path)
            .ok()
            .flatten()
            .and_then(|fm| fm.title);

        blocks.push(KanbanBlockDto {
            id: block.id.to_string(),
            content: block.content,
            parent_id: block.parent_id.map(|id| id.to_string()),
            left_id: block.left_id.map(|id| id.to_string()),
            properties: block.properties.into_iter().collect(),
            marker: block.marker.map(|m| m.as_str().to_string()),
            priority: block.priority.map(|p| p.as_str().to_string()),
            collapsed: block.meta.collapsed,
            heading_level: block.meta.heading_level,
            page_path: page_path.clone(),
            page_title,
        });
    }

    Ok(KanbanDataDto { blocks })
}

/// Create a new block with a task marker on today's journal page.
#[tauri::command]
pub async fn create_kanban_block(
    content: String,
    marker: String,
    state: tauri::State<'_, AppState>,
) -> Result<KanbanBlockDto, String> {
    let state = state.lock().map_err(|e| e.to_string())?;

    // Create the block on today's journal page
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let page_path = format!("journals/{}.md", today);

    let full_path = state.vault_path.join(&page_path);
    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    // Create the block
    let id = Uuid::new_v4();
    let mut block = pkm_block::Block::new(id, content.clone());
    block.marker = pkm_block::TaskMarker::parse(&marker);

    // Open store, insert block, upsert page
    let store = pkm_block::BlockStore::open(&state.db_path).map_err(|e| e.to_string())?;

    // Wrap SQLite operations in an explicit transaction
    store.execute_batch("BEGIN").map_err(|e| e.to_string())?;
    let result = (|| -> Result<(), String> {
        store
            .insert_block(&block, &page_path)
            .map_err(|e| e.to_string())?;
        let page = pkm_block::Page::new(full_path.clone(), &state.vault_path);
        store.upsert_page(&page).map_err(|e| e.to_string())?;
        Ok(())
    })();
    match result {
        Ok(()) => store.execute_batch("COMMIT").map_err(|e| e.to_string())?,
        Err(e) => {
            store.execute_batch("ROLLBACK").ok();
            return Err(e);
        }
    }

    // Serialize all blocks for this page to .md file
    let all_blocks = store
        .get_blocks_by_page(&page_path)
        .map_err(|e| e.to_string())?;
    let body = pkm_markdown::block_parser::serialize_blocks(&all_blocks);
    let markdown = format!("---\ntitle: {}\n---\n\n{}", today, body);
    std::fs::write(&full_path, &markdown).map_err(|e| e.to_string())?;

    Ok(KanbanBlockDto {
        id: id.to_string(),
        content,
        parent_id: None,
        left_id: None,
        properties: Vec::new(),
        marker: Some(marker),
        priority: None,
        collapsed: false,
        heading_level: None,
        page_path: page_path.clone(),
        page_title: Some(today),
    })
}
