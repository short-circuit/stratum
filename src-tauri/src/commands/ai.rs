//! Tauri adapters for AI commands.
//!
//! Thin delegation to [`stratum_app::ai`]; no business logic lives here.

pub use stratum_app::ai::{
    AiAction, AiTransformResult, RagCitationDto, RagQueryResultDto, ResearchResultDto,
    ResearchSourceDto,
};

use crate::commands::vault::AppState;

#[tauri::command]
pub async fn ai_transform_block(
    text: String,
    action: AiAction,
    page_path: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<AiTransformResult, String> {
    state.ai_transform_block(text, action, page_path).await
}

#[tauri::command]
pub async fn ai_research(
    query: String,
    state: tauri::State<'_, AppState>,
) -> Result<ResearchResultDto, String> {
    state.ai_research(query).await
}

#[tauri::command]
pub async fn generate_mermaid(
    prompt: String,
    state: tauri::State<'_, AppState>,
) -> Result<AiTransformResult, String> {
    state.generate_mermaid(prompt).await
}

#[tauri::command]
pub async fn ai_interlink_notes(
    text: String,
    page_path: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<AiTransformResult, String> {
    state.ai_interlink_notes(text, page_path).await
}

#[tauri::command]
pub async fn ai_rag_query(
    question: String,
    state: tauri::State<'_, AppState>,
) -> Result<RagQueryResultDto, String> {
    state.ai_rag_query(question).await
}

#[tauri::command]
pub async fn discover_local_ai(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<pkm_core::DiscoveredEndpoint>, String> {
    state.discover_local_ai().await
}

#[tauri::command]
pub async fn ping_search_instance(
    url: String,
    state: tauri::State<'_, AppState>,
) -> Result<pkm_core::SearchPingResult, String> {
    state.ping_search_instance(url).await
}
