//! Graph visualization command adapters.
//!
//! Thin adapters over [`stratum_app::graph`]. All graph building, caching and
//! link resolution lives in the headless service; this module only preserves
//! the original IPC signatures and re-exports the DTOs at their historical
//! `commands::graph::*` paths.

pub use stratum_app::graph::{
    graph_cache, invalidate_graph_cache, ComponentDto, GraphDataDto, GraphEdgeDto, GraphNodeDto,
    GraphPanelDataDto, LinkTargetDto, OrphanDto,
};

use std::sync::Arc;
use stratum_app::StratumService;

#[tauri::command]
pub async fn get_graph_data(
    state: tauri::State<'_, StratumService>,
) -> Result<GraphDataDto, String> {
    state.get_graph_data().await
}

#[tauri::command]
pub async fn get_connected_components(
    state: tauri::State<'_, StratumService>,
) -> Result<Vec<ComponentDto>, String> {
    state.get_connected_components().await
}

#[tauri::command]
pub async fn get_orphaned_notes(
    state: tauri::State<'_, StratumService>,
) -> Result<Vec<OrphanDto>, String> {
    state.get_orphaned_notes().await
}

#[tauri::command]
pub async fn get_graph_panel_data(
    state: tauri::State<'_, StratumService>,
) -> Result<Arc<GraphPanelDataDto>, String> {
    state.get_graph_panel_data().await
}

#[tauri::command]
pub async fn resolve_link_target(
    target: String,
    state: tauri::State<'_, StratumService>,
) -> Result<LinkTargetDto, String> {
    state.resolve_link_target(target).await
}
