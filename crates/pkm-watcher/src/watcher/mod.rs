use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime};

use crossbeam_channel::{Receiver, RecvTimeoutError};
use notify::event::ModifyKind;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use tracing::{debug, info};

use pkm_core::FileEvent;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// A file change event produced by the watcher after debouncing.
#[derive(Debug, Clone)]
pub struct FileChangeEvent {
    /// Absolute path of the changed file.
    pub path: PathBuf,
    /// Kind of change (Created, Modified, Deleted, Renamed).
    pub kind: FileEvent,
    /// Timestamp when the event was observed.
    pub timestamp: SystemTime,
}

/// File-system watcher with debounce.
///
/// Watches a vault directory recursively for `.md` file changes, ignores
/// files under the `.pkm/` directory, and delivers debounced
/// [`FileChangeEvent`] instances to the configured callback.
///
/// # Example
///
/// ```no_run
/// use pkm_watcher::FileWatcher;
/// use std::path::PathBuf;
///
/// let mut watcher = FileWatcher::new(
///     PathBuf::from("/my/vault"),
///     500,
///     Box::new(|event| {
///         println!("{:?} {:?}", event.kind, event.path);
///     }),
/// );
/// watcher.start().expect("failed to start watcher");
/// // ... later ...
/// watcher.stop();
/// ```
pub struct FileWatcher {
    vault_path: PathBuf,
    debounce_dur: Duration,
    on_event: Option<Box<dyn Fn(FileChangeEvent) + Send + 'static>>,
    watcher: Option<RecommendedWatcher>,
    stop_signal: Arc<AtomicBool>,
    join_handle: Option<JoinHandle<()>>,
}

impl FileWatcher {
    /// Create a new `FileWatcher`.
    ///
    /// The watcher is **not** started until [`Self::start`] is called.
    ///
    /// * `vault_path` – root directory to watch recursively.
    /// * `debounce_ms`  – debounce window in **milliseconds**.
    /// * `on_event`   – closure invoked for each debounced event.
    pub fn new(
        vault_path: PathBuf,
        debounce_ms: u64,
        on_event: Box<dyn Fn(FileChangeEvent) + Send + 'static>,
    ) -> Self {
        Self {
            vault_path,
            debounce_dur: Duration::from_millis(debounce_ms),
            on_event: Some(on_event),
            watcher: None,
            stop_signal: Arc::new(AtomicBool::new(false)),
            join_handle: None,
        }
    }

    /// Start watching.
    ///
    /// Spawns a dedicated background thread.  Returns an error if the
    /// platform-native watcher cannot be initialised or the vault path
    /// does not exist.
    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Take ownership of the callback (replaced with a no-op placeholder).
        let on_event = self.on_event.take().unwrap_or_else(|| Box::new(|_| {}));

        let vault_path = self.vault_path.clone();
        let vault_path_clone = vault_path.clone();
        let debounce_dur = self.debounce_dur;
        let stop_signal = self.stop_signal.clone();

        // Bridge from notify's closure API into crossbeam.
        let (event_tx, event_rx) = crossbeam_channel::unbounded();

        let handler_tx = event_tx.clone();
        let mut watcher: RecommendedWatcher = Watcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    let _ = handler_tx.send(event);
                }
            },
            Config::default(),
        )?;

        watcher.watch(&vault_path, RecursiveMode::Recursive)?;

        let join_handle = thread::Builder::new()
            .name("pkm-watcher".into())
            .spawn(move || {
                process_events_loop(
                    event_rx,
                    &vault_path_clone,
                    debounce_dur,
                    on_event,
                    stop_signal,
                );
            })?;

        self.watcher = Some(watcher);
        self.join_handle = Some(join_handle);

        info!("file watcher started on {:?}", vault_path);
        Ok(())
    }

    /// Stop the watcher.
    ///
    /// Signals the background thread to shut down, drops the native
    /// watcher, and joins the thread.  Any events still buffered by the
    /// debouncer are flushed to the callback before the thread exits.
    pub fn stop(&mut self) {
        self.stop_signal.store(true, Ordering::Relaxed);

        // Drop the native watcher first — this prevents new events from
        // being generated while we drain the remaining ones.
        if let Some(w) = self.watcher.take() {
            drop(w);
        }

        if let Some(handle) = self.join_handle.take() {
            let _ = handle.join();
            debug!("watcher thread joined");
        }

        info!("file watcher stopped");
    }
}

impl Drop for FileWatcher {
    fn drop(&mut self) {
        self.stop();
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Background loop that reads raw `notify` events, converts + filters them,
/// and applies the debounce window before invoking the callback.
fn process_events_loop(
    rx: Receiver<Event>,
    vault_path: &Path,
    debounce_dur: Duration,
    on_event: Box<dyn Fn(FileChangeEvent) + Send + 'static>,
    stop_signal: Arc<AtomicBool>,
) {
    let mut pending: Vec<FileChangeEvent> = Vec::new();

    // Helper: flush buffered events through the callback.
    let flush = |pending: &mut Vec<FileChangeEvent>,
                 cb: &(dyn Fn(FileChangeEvent) + Send + 'static)| {
        if pending.is_empty() {
            return;
        }
        let batch = deduplicate_and_merge(pending);
        for ev in batch {
            cb(ev);
        }
        pending.clear();
    };

    loop {
        // Check stop signal before any blocking call.
        if stop_signal.load(Ordering::Relaxed) {
            flush(&mut pending, &on_event);
            return;
        }

        // Wait for the first event (short timeout lets us poll the stop flag).
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(event) => {
                pending.extend(convert_notify_event(event, vault_path));

                // ── Inner debounce loop ──────────────────────────────
                // Keep collecting events as long as they arrive within
                // `debounce_dur` of the *previous* event.  When the
                // timeout fires we emit the accumulated batch.
                loop {
                    match rx.recv_timeout(debounce_dur) {
                        Ok(event) => {
                            pending.extend(convert_notify_event(event, vault_path));
                            // Continue — the timeout resets implicitly.
                        }
                        Err(RecvTimeoutError::Timeout) => {
                            // No new events → debounce window closed.
                            flush(&mut pending, &on_event);
                            break;
                        }
                        Err(RecvTimeoutError::Disconnected) => {
                            flush(&mut pending, &on_event);
                            return;
                        }
                    }
                }
            }
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => {
                flush(&mut pending, &on_event);
                return;
            }
        }
    }
}

/// Convert a raw `notify::Event` into zero or more `FileChangeEvent`s,
/// applying the project-specific filters:
///
/// * Only `.md` files are reported.
/// * Files under a `.pkm/` directory are silently dropped.
/// * Only `Create`, `Modify`, `Remove` and `Rename` event kinds are mapped.
fn convert_notify_event(event: Event, vault_path: &Path) -> Vec<FileChangeEvent> {
    let timestamp = SystemTime::now();
    let mut result = Vec::new();

    for path in &event.paths {
        // 1. Only process `.md` files.
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }

        // 2. Ignore anything inside a `.pkm/` directory at any level.
        if path.components().any(|c| c.as_os_str() == ".pkm") {
            continue;
        }

        // 3. Must reside within the vault.
        if !path.starts_with(vault_path) {
            continue;
        }

        // 4. Map the notify event kind, handling rename via
        //    Modify(ModifyKind::Name(...)) since notify v7 has no
        //    top-level Rename variant.
        let kind = match event.kind {
            notify::EventKind::Create(_) => FileEvent::Created,
            notify::EventKind::Modify(kind) => match kind {
                ModifyKind::Name(_) => FileEvent::Renamed,
                _ => FileEvent::Modified,
            },
            notify::EventKind::Remove(_) => FileEvent::Deleted,
            _ => continue,
        };

        result.push(FileChangeEvent {
            path: path.clone(),
            kind,
            timestamp,
        });
    }

    result
}

/// Merge events for the same path.
///
/// When multiple events for the same file arrive within a single debounce
/// window, the **last** event wins.  This produces sensible behaviour for
/// common save patterns (e.g. editor atomic-save creates a temp file →
/// renames it → modify).
fn deduplicate_and_merge(events: &[FileChangeEvent]) -> Vec<FileChangeEvent> {
    let mut map: HashMap<PathBuf, &FileChangeEvent> = HashMap::new();
    for event in events {
        map.insert(event.path.clone(), event);
    }

    let mut result: Vec<FileChangeEvent> = map.into_values().cloned().collect();
    result.sort_by(|a, b| a.path.cmp(&b.path));
    result
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests;
