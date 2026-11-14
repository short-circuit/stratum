//! Kanban board data helpers for the service-backed provider.
//!
//! Rust allows a trait to be implemented for a type only once, so the
//! [`PanelData`] methods stay in [`super::app_panels`] and delegate here. Every
//! helper drives [`StratumService`] on the provider's Tokio runtime.
//!
//! The board is a projection of task-marked blocks, so a mutation must keep
//! SQLite *and* the owning Markdown file consistent: `update_block` and
//! `delete_block` are SQLite-only (see `stratum_app::block`), and a later
//! `save_blocks` rewrites the file from the store. Each mutating helper
//! therefore follows the same three steps as the outliner save flow —
//! mutate the store, re-read the page, serialize it back to disk — otherwise a
//! restart (or the file watcher) would resurrect the old block.

use stratum_app::block::BlockDto;

use crate::provider::panels::KanbanCardRow;
use crate::provider::AppProvider;

/// Map a service `KanbanBlockDto` onto the panel row.
fn card_row(block: stratum_app::kanban::KanbanBlockDto) -> KanbanCardRow {
    KanbanCardRow {
        id: block.id,
        content: block.content,
        parent_id: block.parent_id,
        left_id: block.left_id,
        properties: block.properties,
        marker: block.marker,
        priority: block.priority,
        collapsed: block.collapsed,
        heading_level: block.heading_level,
        page_path: block.page_path,
        page_title: block.page_title,
    }
}

/// Re-serialize `page_path` from the store so the Markdown file matches.
fn rewrite_page(provider: &AppProvider, page_path: &str) -> Result<(), String> {
    let blocks = provider
        .runtime
        .block_on(provider.service.get_blocks(page_path.to_string()))?;
    provider.runtime.block_on(provider.service.save_blocks(
        page_path.to_string(),
        blocks.blocks,
        None,
    ))
}

/// List every task-marked block across the vault.
pub(super) fn cards(provider: &AppProvider) -> Result<Vec<KanbanCardRow>, String> {
    let data = provider
        .runtime
        .block_on(provider.service.get_kanban_blocks())?;
    Ok(data.blocks.into_iter().map(card_row).collect())
}

/// Create a task block on today's journal page.
pub(super) fn create(
    provider: &AppProvider,
    content: &str,
    marker: &str,
) -> Result<KanbanCardRow, String> {
    let created = provider.runtime.block_on(
        provider
            .service
            .create_kanban_block(content.to_string(), marker.to_string()),
    )?;
    Ok(card_row(created))
}

/// Persist an edited card and rewrite its page.
pub(super) fn update(provider: &AppProvider, card: KanbanCardRow) -> Result<(), String> {
    let dto = BlockDto {
        id: card.id,
        content: card.content,
        parent_id: card.parent_id,
        left_id: card.left_id,
        properties: card.properties,
        marker: card.marker,
        priority: card.priority,
        collapsed: card.collapsed,
        heading_level: card.heading_level,
    };
    provider
        .runtime
        .block_on(provider.service.update_block(card.page_path.clone(), dto))?;
    rewrite_page(provider, &card.page_path)
}

/// Delete a card and rewrite its page.
pub(super) fn delete(provider: &AppProvider, id: &str, page_path: &str) -> Result<(), String> {
    provider
        .runtime
        .block_on(provider.service.delete_block(id.to_string()))?;
    rewrite_page(provider, page_path)
}
