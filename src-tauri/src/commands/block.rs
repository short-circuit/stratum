//! Block management commands.

use crate::commands::vault::AppState;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BlockDto {
    pub id: String,
    pub content: String,
    pub parent_id: Option<String>,
    pub left_id: Option<String>,
    pub properties: Vec<(String, String)>,
    pub marker: Option<String>,
    pub priority: Option<String>,
    pub collapsed: bool,
    pub heading_level: Option<u8>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BlockListDto {
    pub blocks: Vec<BlockDto>,
}

#[tauri::command]
pub async fn get_blocks(
    page_path: String,
    state: tauri::State<'_, AppState>,
) -> Result<BlockListDto, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let store = pkm_block::BlockStore::open(&state.db_path).map_err(|e| e.to_string())?;
    let blocks = store.get_blocks_by_page(&page_path).map_err(|e| e.to_string())?;

    let dtos: Vec<BlockDto> = blocks
        .into_iter()
        .map(|b| BlockDto {
            id: b.id.to_string(),
            content: b.content,
            parent_id: b.parent_id.map(|id| id.to_string()),
            left_id: b.left_id.map(|id| id.to_string()),
            properties: b.properties.into_iter().collect(),
            marker: b.marker.map(|m| m.as_str().to_string()),
            priority: b.priority.map(|p| p.as_str().to_string()),
            collapsed: b.meta.collapsed,
            heading_level: b.meta.heading_level,
        })
        .collect();

    Ok(BlockListDto { blocks: dtos })
}

#[tauri::command]
pub async fn update_block(
    page_path: String,
    block: BlockDto,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let id = Uuid::parse_str(&block.id).map_err(|e| e.to_string())?;

    let mut b = pkm_block::Block::new(id, block.content);
    b.parent_id = block.parent_id.as_ref().and_then(|s| Uuid::parse_str(s).ok());
    b.left_id = block.left_id.as_ref().and_then(|s| Uuid::parse_str(s).ok());
    b.properties = block.properties.into_iter().collect();
    b.marker = block.marker.as_ref().and_then(|m| pkm_block::TaskMarker::from_str(m));
    b.priority = block
        .priority
        .as_ref()
        .and_then(|p| pkm_block::Priority::from_str(p));
    b.meta.collapsed = block.collapsed;
    b.meta.heading_level = block.heading_level;

    let store = pkm_block::BlockStore::open(&state.db_path).map_err(|e| e.to_string())?;
    store.insert_block(&b, &page_path).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn delete_block(
    block_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let id = Uuid::parse_str(&block_id).map_err(|e| e.to_string())?;
    let store = pkm_block::BlockStore::open(&state.db_path).map_err(|e| e.to_string())?;
    store.delete_block(id).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn insert_block(
    page_path: String,
    content: String,
    parent_id: Option<String>,
    after_id: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<BlockDto, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let id = Uuid::new_v4();

    let mut block = pkm_block::Block::new(id, content);
    block.parent_id = parent_id.as_ref().and_then(|s| Uuid::parse_str(s).ok());
    block.left_id = after_id.as_ref().and_then(|s| Uuid::parse_str(s).ok());

    let store = pkm_block::BlockStore::open(&state.db_path).map_err(|e| e.to_string())?;
    store.insert_block(&block, &page_path).map_err(|e| e.to_string())?;

    Ok(BlockDto {
        id: id.to_string(),
        content: block.content,
        parent_id: block.parent_id.map(|id| id.to_string()),
        left_id: block.left_id.map(|id| id.to_string()),
        properties: Vec::new(),
        marker: None,
        priority: None,
        collapsed: false,
        heading_level: None,
    })
}
