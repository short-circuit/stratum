//! Event plumbing for headless hosts.
//!
//! Commands that used to call `AppHandle::emit` now receive a
//! [`EventSink`] and forward the same event name/payload through it. Tauri
//! adapters implement the trait over the real `AppHandle`; tests can record
//! emitted events. [`NoopSink`] discards everything.

use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Sink for progress/lifecycle events emitted by long-running operations.
///
/// Implementations must be `Send + Sync` because operations may hand the sink
/// to a `'static` progress callback (see [`EventSink::shared`]).
pub trait EventSink: Send + Sync {
    /// Emit `payload` under `event`. Hosts must not fail the operation when
    /// delivery fails (the original Tauri adapters ignored emit errors).
    fn emit(&self, event: &str, payload: serde_json::Value);

    /// Return a `'static` clone of this sink for use inside progress callbacks.
    ///
    /// Implementations should cheaply clone whatever handle they wrap.
    fn shared(&self) -> Arc<dyn EventSink>;
}

/// Event sink that discards every event.
pub struct NoopSink;

impl EventSink for NoopSink {
    fn emit(&self, _event: &str, _payload: serde_json::Value) {}

    fn shared(&self) -> Arc<dyn EventSink> {
        Arc::new(NoopSink)
    }
}

/// Payload emitted as a `"reindex-progress"` event during long-running
/// operations.
#[derive(Debug, Clone, Serialize)]
pub struct ProgressEventPayload {
    pub message: String,
    pub percent: f32,
}

/// Structured result for reindex/repair operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReindexResult {
    pub processed: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub errors: Vec<String>,
}
