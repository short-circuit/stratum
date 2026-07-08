pub mod ai;
pub mod block;
pub mod export;
pub mod flashcards;
pub mod graph;
pub mod kanban;
pub mod page;
pub mod query;
pub mod search;
pub mod settings;
pub mod sync;
pub mod template;
pub mod vault;
pub mod whiteboard;

use serde::{Deserialize, Serialize};

/// Payload emitted as a "reindex-progress" Tauri event during long-running operations.
#[derive(Debug, Clone, Serialize)]
pub struct ProgressEventPayload {
    pub message: String,
    pub percent: f32,
}

/// Structured result for reindex operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReindexResult {
    pub processed: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub errors: Vec<String>,
}
