//! Shared vault state and the [`StratumService`] handle.
//!
//! The state struct is the same one the Tauri shell used to manage; the
//! service wraps it in an `Arc<Mutex<_>>` and exposes cloneable access plus a
//! poison-aware [`StratumService::lock`] helper. Domain modules add
//! `impl StratumService` blocks with one method per command.

use crate::plugins::PluginManager;
use pkm_block::BlockStore;
use pkm_block::StoreResult;
use pkm_index::block_search::BlockIndex;
use pkm_index::indexer::IndexEngine;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::SystemTime;
use tracing::info;

/// Application state holding the active vault.
pub struct VaultState {
    pub vault_path: PathBuf,
    pub db_path: PathBuf,
    pub block_store: Option<BlockStore>,
    pub index_engine: Option<IndexEngine>,
    pub block_index: Option<BlockIndex>,
    pub sync_scheduler: Option<pkm_sync::SyncScheduler>,
    pub auto_commit_engine: Option<pkm_sync::AutoCommitEngine>,
    /// Periodic cloud/folder backup scheduler (E10.F3), independent of git.
    pub backup_scheduler: Option<pkm_sync::BackupScheduler>,
    pub passphrase: Option<String>,
    #[cfg(not(target_os = "android"))]
    pub watcher: Option<pkm_watcher::FileWatcher>,
    pub watcher_last_save: SystemTime,
    indexing_in_progress: Arc<AtomicBool>,
    /// Active voice recording (started via `dictation_start`).
    pub recorder: Option<ActiveRecording>,
    /// Finished dictation sessions keyed by recording path, so speakers can
    /// be renamed/enrolled after insertion.
    pub dictation_sessions: std::collections::HashMap<String, DictationSession>,
    /// Per-vault WASM plugin manager (registry + runtime + host), or `None`
    /// until a vault is opened/initialized.
    pub plugin_manager: Option<Arc<PluginManager>>,
}

/// An in-progress voice recording.
pub struct ActiveRecording {
    pub handle: pkm_audio::RecordingHandle,
    pub recording_path: PathBuf,
}

/// A completed dictation: everything needed to re-render the memo.
#[derive(Clone)]
pub struct DictationSession {
    pub page_path: String,
    pub clip_path: PathBuf,
    pub clip_rel_path: String,
    pub recorded_at: chrono::DateTime<chrono::Local>,
    pub duration_secs: f64,
    pub turns: Vec<pkm_stt::SpeakerTurn>,
    pub speaker_names: std::collections::HashMap<String, String>,
    pub summary: Option<String>,
    pub related: Vec<String>,
    pub tags: Vec<String>,
    pub inserted_block_ids: Vec<String>,
}

impl VaultState {
    pub fn new(vault_path: PathBuf) -> Self {
        let db_path = vault_path.join(".pkm").join("blocks.db");
        let block_store = BlockStore::open(&db_path).ok();
        let index_engine = IndexEngine::new(&vault_path).ok();
        if index_engine.is_some() {
            info!("IndexEngine initialized at {:?}", vault_path);
        }
        Self {
            vault_path,
            db_path,
            block_store,
            index_engine,
            block_index: None,
            sync_scheduler: None,
            auto_commit_engine: None,
            backup_scheduler: None,
            passphrase: None,
            #[cfg(not(target_os = "android"))]
            watcher: None,
            watcher_last_save: SystemTime::UNIX_EPOCH,
            indexing_in_progress: Arc::new(AtomicBool::new(false)),
            recorder: None,
            dictation_sessions: std::collections::HashMap::new(),
            plugin_manager: None,
        }
    }

    /// Get a handle to the block store.
    /// Opens a new connection to the same SQLite database.
    /// The stored block_store keeps the database "warm" (open).
    /// Future optimization: switch to connection pooling via Arc<Mutex<Connection>>.
    pub fn get_store(&self) -> StoreResult<BlockStore> {
        BlockStore::open(&self.db_path)
    }

    pub fn ensure_index(&mut self) -> Result<&mut IndexEngine, String> {
        if self.index_engine.is_none() {
            self.index_engine = Some(
                IndexEngine::new(&self.vault_path)
                    .map_err(|e| format!("Failed to create IndexEngine: {}", e))?,
            );
        }
        self.index_engine
            .as_mut()
            .ok_or_else(|| "IndexEngine not available".to_string())
    }

    pub fn ensure_block_index(&mut self) -> Result<&mut BlockIndex, String> {
        if self.block_index.is_none() {
            let index_path = self.vault_path.join(".pkm").join("search");
            self.block_index = Some(
                BlockIndex::create(&index_path)
                    .map_err(|e| format!("Failed to create BlockIndex: {}", e))?,
            );
        }
        self.block_index
            .as_mut()
            .ok_or_else(|| "BlockIndex not available".to_string())
    }

    #[allow(dead_code)]
    pub fn flush_block_index(&mut self) -> Result<(), String> {
        if let Some(ref mut block_index) = self.block_index {
            block_index
                .flush()
                .map_err(|e| format!("Failed to flush block index: {}", e))?;
        }
        Ok(())
    }

    pub fn set_passphrase(&mut self, passphrase: String) {
        self.passphrase = Some(passphrase);
    }

    /// Attempt to mark indexing as in-progress.
    /// Returns an error if another index operation is already running.
    pub fn try_start_indexing(&self) -> Result<(), String> {
        self.indexing_in_progress
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .map(|_| ())
            .map_err(|_| "A reindex operation is already in progress".to_string())
    }

    /// Clear the indexing-in-progress flag.
    pub fn finish_indexing(&self) {
        self.indexing_in_progress.store(false, Ordering::SeqCst);
    }

    /// Get the indexing-in-progress flag.
    #[allow(dead_code)] // used on desktop but not android (file watcher disabled)
    pub fn is_indexing(&self) -> bool {
        self.indexing_in_progress.load(Ordering::SeqCst)
    }

    /// Record a file change to trigger the auto-commit engine.
    pub fn record_change(&mut self, path: &str) {
        if let Some(ref mut engine) = self.auto_commit_engine {
            let _ = engine.record_change(path);
            let _ = engine.tick(); // Immediately evaluate whether to commit
        }
    }
}

/// RAII guard that marks indexing as in-progress for its lifetime.
/// When the guard is dropped, the flag is automatically cleared.
///
/// The guard owns an `Arc` clone of the shared flag rather than borrowing the
/// `VaultState`, so it can be held across bodies that take `&mut self` (e.g.
/// `ensure_index`, `get_store`) — a borrow-based guard would conflict with those
/// mutable borrows. This lets callers span their entire fallible body with the
/// guard, guaranteeing the flag is released even when a `?` returns early.
pub struct IndexingGuard {
    flag: Arc<AtomicBool>,
}

impl IndexingGuard {
    /// Create a new guard, returning an error if indexing is already in progress.
    pub fn new(state: &VaultState) -> Result<Self, String> {
        state.try_start_indexing()?;
        Ok(Self {
            flag: Arc::clone(&state.indexing_in_progress),
        })
    }
}

impl Drop for IndexingGuard {
    fn drop(&mut self) {
        self.flag.store(false, Ordering::SeqCst);
    }
}

/// Cloneable handle around the shared vault state.
///
/// Every domain module implements its commands as methods on this type. The
/// lock is poison-aware and returns the same error string the Tauri commands
/// used (`PoisonError::to_string`).
#[derive(Clone)]
pub struct StratumService {
    state: Arc<Mutex<VaultState>>,
}

impl StratumService {
    /// Create a service for `vault_path`. Mirrors `VaultState::new`.
    pub fn new(vault_path: PathBuf) -> Self {
        Self {
            state: Arc::new(Mutex::new(VaultState::new(vault_path))),
        }
    }

    /// Lock the vault state, mapping a poisoned mutex to its error string.
    pub fn lock(&self) -> Result<MutexGuard<'_, VaultState>, String> {
        self.state.lock().map_err(|e| e.to_string())
    }

    /// Current vault root path.
    pub fn vault_path(&self) -> PathBuf {
        match self.lock() {
            Ok(state) => state.vault_path.clone(),
            Err(_) => PathBuf::new(),
        }
    }

    /// Shared state handle (for hosts that need to install engines/watchers).
    pub fn state(&self) -> &Arc<Mutex<VaultState>> {
        &self.state
    }
}

/// Legacy alias retained for hosts that used `Mutex<VaultState>` as managed
/// state. It now resolves to the headless service handle.
pub type AppState = StratumService;

/// Re-export so callers can name the vault path type without importing `std`.
pub type VaultPath = Path;
