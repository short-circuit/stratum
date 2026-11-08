//! Block management command adapters over [`stratum_app::block`].

pub use stratum_app::block::{BlockDto, BlockListDto};

use stratum_app::StratumService;

#[tauri::command]
pub async fn build_markdown(
    blocks: Vec<BlockDto>,
    title: Option<String>,
    state: tauri::State<'_, StratumService>,
) -> Result<String, String> {
    state.build_markdown(blocks, title).await
}

#[tauri::command]
pub async fn save_blocks(
    page_path: String,
    blocks: Vec<BlockDto>,
    title: Option<String>,
    state: tauri::State<'_, StratumService>,
) -> Result<(), String> {
    state.save_blocks(page_path, blocks, title).await
}

#[tauri::command]
pub async fn get_blocks(
    page_path: String,
    state: tauri::State<'_, StratumService>,
) -> Result<BlockListDto, String> {
    state.get_blocks(page_path).await
}

#[tauri::command]
pub async fn update_block(
    page_path: String,
    block: BlockDto,
    state: tauri::State<'_, StratumService>,
) -> Result<(), String> {
    state.update_block(page_path, block).await
}

#[tauri::command]
pub async fn delete_block(
    block_id: String,
    state: tauri::State<'_, StratumService>,
) -> Result<(), String> {
    state.delete_block(block_id).await
}

#[tauri::command]
pub async fn insert_block(
    page_path: String,
    content: String,
    parent_id: Option<String>,
    after_id: Option<String>,
    state: tauri::State<'_, StratumService>,
) -> Result<BlockDto, String> {
    state
        .insert_block(page_path, content, parent_id, after_id)
        .await
}

#[tauri::command]
pub async fn toggle_block_marker(
    page_path: String,
    block_id: String,
    state: tauri::State<'_, StratumService>,
) -> Result<Option<String>, String> {
    state.toggle_block_marker(page_path, block_id).await
}

#[tauri::command]
pub async fn clear_block_marker(
    page_path: String,
    block_id: String,
    state: tauri::State<'_, StratumService>,
) -> Result<(), String> {
    state.clear_block_marker(page_path, block_id).await
}
