//! Tauri command adapters.
//!
//! Every command module in this tree is a thin adapter over
//! [`stratum_app::StratumService`]: it keeps the original IPC signature, then
//! delegates to the headless service. Business logic lives in `stratum-app`.

pub mod ai;
pub mod asset;
pub mod backup;
pub mod block;
pub mod dictation;
pub mod export;
pub mod flashcards;
pub mod graph;
pub mod import;
pub mod kanban;
pub mod page;
pub mod plugins;
pub mod query;
pub mod reminders;
pub mod search;
pub mod settings;
pub mod sync;
pub mod template;
#[cfg(desktop)]
pub mod tray;
pub mod tts;
pub mod vault;
pub mod whiteboard;

pub use stratum_app::events::{ProgressEventPayload, ReindexResult};

use std::sync::Arc;

/// Tauri-backed [`stratum_app::events::EventSink`].
///
/// Forwards service events to the frontend with the exact event names and
/// payload shapes the original commands emitted. Delivery failures are ignored,
/// matching the previous `let _ = app.emit(...)` behavior.
pub struct TauriEventSink(pub tauri::AppHandle);

impl stratum_app::events::EventSink for TauriEventSink {
    fn emit(&self, event: &str, payload: serde_json::Value) {
        use tauri::Emitter;
        let _ = self.0.emit(event, payload);
    }

    fn shared(&self) -> Arc<dyn stratum_app::events::EventSink> {
        Arc::new(Self(self.0.clone()))
    }
}
