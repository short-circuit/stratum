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
pub async fn build_markdown(
    blocks: Vec<BlockDto>,
    title: Option<String>,
) -> Result<String, String> {
    let pkm_blocks: Vec<pkm_block::Block> = blocks
        .into_iter()
        .map(|dto| {
            let id = Uuid::parse_str(&dto.id).unwrap_or_else(|_| Uuid::new_v4());
            let mut b = pkm_block::Block::new(id, dto.content);
            b.parent_id = dto.parent_id.as_ref().and_then(|s| Uuid::parse_str(s).ok());
            b.left_id = dto.left_id.as_ref().and_then(|s| Uuid::parse_str(s).ok());
            b.properties = dto.properties.into_iter().collect();
            b.marker = dto.marker.as_ref().and_then(|m| pkm_block::TaskMarker::from_str(m));
            b.priority = dto.priority.as_ref().and_then(|p| pkm_block::Priority::from_str(p));
            b.meta.collapsed = dto.collapsed;
            b.meta.heading_level = dto.heading_level;
            b
        })
        .collect();

    let body = pkm_markdown::block_parser::serialize_blocks(&pkm_blocks);

    let full = if let Some(t) = title {
        format!("---\ntitle: {}\n---\n\n{}", t, body)
    } else {
        body
    };

    Ok(full)
}

#[tauri::command]
pub async fn save_blocks(
    page_path: String,
    blocks: Vec<BlockDto>,
    title: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let state = state.lock().map_err(|e| e.to_string())?;

    let pkm_blocks: Vec<pkm_block::Block> = blocks
        .into_iter()
        .map(|dto| {
            let id = Uuid::parse_str(&dto.id).unwrap_or_else(|_| Uuid::new_v4());
            let mut b = pkm_block::Block::new(id, dto.content);
            b.parent_id = dto.parent_id.as_ref().and_then(|s| Uuid::parse_str(s).ok());
            b.left_id = dto.left_id.as_ref().and_then(|s| Uuid::parse_str(s).ok());
            b.properties = dto.properties.into_iter().collect();
            b.marker = dto.marker.as_ref().and_then(|m| pkm_block::TaskMarker::from_str(m));
            b.priority = dto.priority.as_ref().and_then(|p| pkm_block::Priority::from_str(p));
            b.meta.collapsed = dto.collapsed;
            b.meta.heading_level = dto.heading_level;
            b
        })
        .collect();

    let body = pkm_markdown::block_parser::serialize_blocks(&pkm_blocks);
    let markdown = if let Some(t) = &title {
        format!("---\ntitle: {}\n---\n\n{}", t, body)
    } else {
        body
    };

    let full_path = state.vault_path.join(&page_path);
    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&full_path, &markdown).map_err(|e| e.to_string())?;

    let store = pkm_block::BlockStore::open(&state.db_path).map_err(|e| e.to_string())?;
    store.delete_blocks_by_page(&page_path).map_err(|e| e.to_string())?;
    for block in &pkm_blocks {
        store.insert_block(block, &page_path).map_err(|e| e.to_string())?;
    }

    // Index blocks in Tantivy for full-text search
    let index_path = state.vault_path.join(".pkm").join("search");
    if let Ok(mut block_index) = pkm_index::block_search::BlockIndex::create(&index_path) {
        for block in &pkm_blocks {
            let _ = block_index.index_block(block, &page_path);
        }
        let _ = block_index.flush();
    }

    let mut page = pkm_block::Page::new(full_path, &state.vault_path);
    if let Some(t) = title {
        page.frontmatter.title = Some(t);
    }
    store.upsert_page(&page).map_err(|e| e.to_string())?;

    Ok(())
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BlockPropertiesDto {
    pub block_id: String,
    pub properties: Vec<(String, String)>,
    pub marker: Option<String>,
    pub priority: Option<String>,
}

#[tauri::command]
pub async fn get_block_properties(
    block_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<BlockPropertiesDto, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let id = Uuid::parse_str(&block_id).map_err(|e| e.to_string())?;
    let store = pkm_block::BlockStore::open(&state.db_path).map_err(|e| e.to_string())?;
    let block = store.get_block(id).map_err(|e| e.to_string())?;

    Ok(BlockPropertiesDto {
        block_id: block.id.to_string(),
        properties: block.properties.into_iter().collect(),
        marker: block.marker.map(|m| m.as_str().to_string()),
        priority: block.priority.map(|p| p.as_str().to_string()),
    })
}

#[tauri::command]
pub async fn set_block_property(
    block_id: String,
    key: String,
    value: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let id = Uuid::parse_str(&block_id).map_err(|e| e.to_string())?;
    let store = pkm_block::BlockStore::open(&state.db_path).map_err(|e| e.to_string())?;
    let mut block = store.get_block(id).map_err(|e| e.to_string())?;
    block.properties.insert(key, value);
    store.insert_block(&block, "").map_err(|e| e.to_string())?;
    Ok(())
}
