//! Headless Stratum service layer.
//!
//! This crate owns the business logic behind every Tauri command: vault
//! lifecycle, pages, blocks, assets, import, graph, search, sync, backup,
//! templates, export, flashcards, kanban, reminders, whiteboards, AI,
//! dictation, TTS, settings, plugins, plugin/theme catalogs and installed
//! themes. It has **no** dependency on `tauri`
//! or any `tauri-plugin-*` crate, so the same logic can be driven from the
//! desktop shell, tests, or any other host.
//!
//! Hosts interact with [`StratumService`]: a cloneable handle around the
//! shared vault state. Commands are exposed as methods whose names, argument
//! lists, return types and error strings mirror the original IPC surface
//! exactly. Operations that previously emitted Tauri events accept an
//! [`events::EventSink`] so the host can forward them to its own event bus.
//!
//! ```no_run
//! use stratum_app::StratumService;
//! use stratum_app::events::NoopSink;
//!
//! # async fn run() -> Result<(), String> {
//! let service = StratumService::new(std::path::PathBuf::from("/tmp/vault"));
//! let info = service.get_vault_info().await?;
//! let _ = info.block_count;
//! # Ok(())
//! # }
//! ```

pub mod ai;
pub mod asset;
pub mod backup;
pub mod block;
pub mod dictation;
pub mod events;
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
pub mod state;
pub mod sync;
pub mod template;
pub mod themes;
pub mod tray;
pub mod tts;
pub mod vault;
pub mod whiteboard;

pub use events::{EventSink, NoopSink, ProgressEventPayload, ReindexResult};
pub use state::{ActiveRecording, DictationSession, IndexingGuard, StratumService, VaultState};
