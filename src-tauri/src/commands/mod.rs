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

use serde::Serialize;
use tauri::Emitter;

/// Payload emitted as a "reindex-progress" Tauri event during long-running operations.
#[derive(Debug, Clone, Serialize)]
pub struct ProgressEventPayload {
    pub message: String,
    pub percent: f32,
}

/// Create a `ProgressCallback` that emits Tauri events for progress reporting.
///
/// The returned closure captures the `AppHandle` and emits a `"reindex-progress"`
/// event with a `ProgressEventPayload` on each progress tick.
pub fn make_progress_callback(app: tauri::AppHandle) -> pkm_core::ProgressCallback {
    Box::new(move |message: String, percent: f32| {
        let _ = app.emit(
            "reindex-progress",
            ProgressEventPayload { message, percent },
        );
    })
}
