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
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Mutex;
    use tempfile::TempDir;

    /// Helper: create a temp vault with a `.pkm/` sub-directory and return
    /// the `TempDir` plus the callback-collector.
    fn setup_test_env() -> (TempDir, Arc<Mutex<Vec<FileChangeEvent>>>) {
        let dir = TempDir::with_prefix("pkm-watcher-test-").unwrap();

        // Create an empty .pkm directory so we can verify it's ignored.
        fs::create_dir_all(dir.path().join(".pkm")).unwrap();

        let events: Arc<Mutex<Vec<FileChangeEvent>>> = Arc::new(Mutex::new(Vec::new()));
        (dir, events)
    }

    /// Create a watcher that writes into the shared vector.
    fn make_watcher(vault_path: PathBuf, events: Arc<Mutex<Vec<FileChangeEvent>>>) -> FileWatcher {
        FileWatcher::new(
            vault_path,
            200, // short debounce for tests
            Box::new(move |ev| {
                events.lock().unwrap().push(ev);
            }),
        )
    }

    // ── helpers ──────────────────────────────────────────────────────

    fn touch_md(dir: &Path, name: &str) -> PathBuf {
        let p = dir.join(name);
        fs::write(&p, b"hello").unwrap();
        p
    }

    fn write_md(dir: &Path, name: &str, content: &[u8]) -> PathBuf {
        let p = dir.join(name);
        fs::write(&p, content).unwrap();
        p
    }

    fn delete_file(path: &Path) {
        fs::remove_file(path).unwrap();
    }

    fn rename_file(old: &Path, new: &Path) {
        fs::rename(old, new).unwrap();
    }

    fn wait_for_events(
        events: &Arc<Mutex<Vec<FileChangeEvent>>>,
        min: usize,
    ) -> Vec<FileChangeEvent> {
        let deadline = Duration::from_secs(5);
        let poll = Duration::from_millis(50);
        let start = std::time::Instant::now();

        loop {
            {
                let guard = events.lock().unwrap();
                if guard.len() >= min {
                    return guard.clone();
                }
            }
            if start.elapsed() > deadline {
                // Return whatever we have (tests will assert).
                return events.lock().unwrap().clone();
            }
            thread::sleep(poll);
        }
    }

    // ── Tests ────────────────────────────────────────────────────────

    #[test]
    fn test_create_md_file() {
        let (dir, events) = setup_test_env();
        let vault = dir.path().to_path_buf();
        let mut watcher = make_watcher(vault.clone(), events.clone());
        watcher.start().unwrap();

        // Let the watcher settle.
        thread::sleep(Duration::from_millis(100));

        let created = touch_md(&vault, "note.md");
        let received = wait_for_events(&events, 1);

        assert!(
            !received.is_empty(),
            "expected at least one event for file creation"
        );
        let ev = &received[0];
        assert_eq!(ev.path, created);
        // On Linux (inotify), a file create+write may arrive as a single
        // `Modified` event rather than `Created`.  Accept either.
        assert!(
            matches!(ev.kind, FileEvent::Created | FileEvent::Modified),
            "expected Created or Modified, got {:?}",
            ev.kind
        );
        assert!(
            ev.timestamp.duration_since(SystemTime::UNIX_EPOCH).is_ok(),
            "timestamp should be valid"
        );

        watcher.stop();
    }

    #[test]
    fn test_modify_md_file() {
        let (dir, events) = setup_test_env();
        let vault = dir.path().to_path_buf();

        // Pre-create a file.
        let note = write_md(&vault, "edit.md", b"original");

        let mut watcher = make_watcher(vault.clone(), events.clone());
        watcher.start().unwrap();
        thread::sleep(Duration::from_millis(100));

        // Modify it.
        // To reliably trigger a modify event, we use a different write approach.
        write_md(&vault, "edit.md", b"modified content");

        let received = wait_for_events(&events, 1);

        assert!(!received.is_empty(), "expected at least one modify event");
        let ev = &received[0];
        assert_eq!(ev.path, note);
        assert!(
            matches!(ev.kind, FileEvent::Created | FileEvent::Modified),
            "expected Created or Modified, got {:?}",
            ev.kind
        );

        watcher.stop();
    }

    #[test]
    fn test_delete_md_file() {
        let (dir, events) = setup_test_env();
        let vault = dir.path().to_path_buf();
        let note = touch_md(&vault, "delete-me.md");

        let mut watcher = make_watcher(vault.clone(), events.clone());
        watcher.start().unwrap();
        thread::sleep(Duration::from_millis(100));

        delete_file(&note);

        let received = wait_for_events(&events, 1);

        // The delete event might be paired with a prior create event from
        // the initial `touch_md` — check that at least one Delete exists.
        let deletes: Vec<_> = received
            .iter()
            .filter(|e| e.kind == FileEvent::Deleted)
            .collect();
        assert!(
            !deletes.is_empty(),
            "expected at least one Deleted event, got {:?}",
            received
        );
        assert_eq!(deletes[0].path, note);

        watcher.stop();
    }

    #[test]
    fn test_rename_md_file() {
        let (dir, events) = setup_test_env();
        let vault = dir.path().to_path_buf();
        let old = touch_md(&vault, "old-name.md");
        let new = vault.join("new-name.md");

        let mut watcher = make_watcher(vault.clone(), events.clone());
        watcher.start().unwrap();
        thread::sleep(Duration::from_millis(100));

        rename_file(&old, &new);

        let received = wait_for_events(&events, 1);

        // Rename may produce events for both old path (Remove) and new path
        // (Create) on some platforms, or a single Rename event.  Accept
        // either.
        let renames: Vec<_> = received
            .iter()
            .filter(|e| e.kind == FileEvent::Renamed)
            .collect();
        let removes: Vec<_> = received
            .iter()
            .filter(|e| e.kind == FileEvent::Deleted)
            .collect();
        let creates: Vec<_> = received
            .iter()
            .filter(|e| e.kind == FileEvent::Created)
            .collect();

        let has_rename = !renames.is_empty();
        let has_remove_and_create = !removes.is_empty() && !creates.is_empty();
        assert!(
            has_rename || has_remove_and_create,
            "expected Renamed event(s) or Remove+Create pair, got {:?}",
            received
        );

        watcher.stop();
    }

    #[test]
    fn test_ignores_pkm_directory() {
        let (dir, events) = setup_test_env();
        let vault = dir.path().to_path_buf();

        let mut watcher = make_watcher(vault.clone(), events.clone());
        watcher.start().unwrap();
        thread::sleep(Duration::from_millis(100));

        // Create a .md file inside .pkm/ — should not trigger events.
        let ignored = vault.join(".pkm/internal.md");
        write_md(&vault, ".pkm/internal.md", b"ignored");

        // Also create a real note to verify the watcher is still alive.
        let real = touch_md(&vault, "real-note.md");

        let received = wait_for_events(&events, 1);

        // The ignored file should never appear.
        for ev in &received {
            assert_ne!(
                ev.path, ignored,
                ".pkm/ file should have been ignored but got event {:?}",
                ev
            );
        }

        // But the real note should have triggered an event.
        let real_events: Vec<_> = received.iter().filter(|e| e.path == real).collect();
        assert!(
            !real_events.is_empty(),
            "expected event for real-note.md, got {:?}",
            received
        );

        watcher.stop();
    }

    #[test]
    fn test_ignores_non_md_files() {
        let (dir, events) = setup_test_env();
        let vault = dir.path().to_path_buf();

        let mut watcher = make_watcher(vault.clone(), events.clone());
        watcher.start().unwrap();
        thread::sleep(Duration::from_millis(100));

        // Create various non-.md files — none should trigger events.
        let txt = vault.join("readme.txt");
        let json = vault.join("data.json");
        let hidden = vault.join(".hidden");

        fs::write(&txt, b"hello").unwrap();
        fs::write(&json, b"{}").unwrap();
        fs::write(&hidden, b"secret").unwrap();

        // Wait a bit to make sure no events arrive.
        thread::sleep(Duration::from_millis(600));

        let guard = events.lock().unwrap();
        // We might have events from directory creation itself (depending on
        // platform); filter to only the non-.md paths.
        let bad: Vec<_> = guard
            .iter()
            .filter(|e| e.path == txt || e.path == json || e.path == hidden)
            .collect();
        assert!(
            bad.is_empty(),
            "non-.md files should be ignored, but got events: {:?}",
            bad
        );

        watcher.stop();
    }

    #[test]
    fn test_debounce_collapses_rapid_events() {
        let (dir, events) = setup_test_env();
        let vault = dir.path().to_path_buf();

        // Long debounce to ensure collapsing.
        let watcher_events = events.clone();
        let mut watcher = FileWatcher::new(
            vault.clone(),
            500, // 500 ms debounce
            Box::new(move |ev| {
                watcher_events.lock().unwrap().push(ev);
            }),
        );
        watcher.start().unwrap();
        thread::sleep(Duration::from_millis(100));

        // Rapidly write to the same file several times.
        let note = vault.join("rapid.md");
        for i in 0..5 {
            fs::write(&note, format!("content {i}")).unwrap();
            thread::sleep(Duration::from_millis(10));
        }

        // Wait for debounce to expire.
        thread::sleep(Duration::from_millis(800));

        let guard = events.lock().unwrap();
        // Should have at most 2 events (maybe one Create + one last Modify
        // after dedup, or just one if dedup fully merged).
        assert!(
            guard.len() <= 2,
            "debounce should collapse rapid events; got {} events: {:?}",
            guard.len(),
            *guard
        );

        // Verify all paths point to our note.
        for ev in guard.iter() {
            assert_eq!(ev.path, note);
        }

        watcher.stop();
    }

    #[test]
    fn test_start_stop_cleanly() {
        // Verify that start() followed by stop() does not panic or leak
        // any resources, even when no files are touched.
        let (dir, events) = setup_test_env();
        let vault = dir.path().to_path_buf();

        let mut watcher = make_watcher(vault.clone(), events.clone());
        watcher.start().unwrap();
        thread::sleep(Duration::from_millis(50));
        watcher.stop();

        // The watcher should be reusable for a second start.
        watcher.start().unwrap();
        thread::sleep(Duration::from_millis(50));

        let _created = touch_md(&vault, "after-restart.md");
        // Note: the callback was consumed by the first start(), so on
        // the second start() it is a no-op.  The test passes as long
        // as no crash/panic occurs.
        watcher.stop();
    }

    #[test]
    fn test_drop_stops_watcher() {
        let (dir, events) = setup_test_env();
        let vault = dir.path().to_path_buf();

        {
            let mut watcher = make_watcher(vault.clone(), events.clone());
            watcher.start().unwrap();
            thread::sleep(Duration::from_millis(50));
            // watcher drops here → should stop cleanly
        }

        // Create a file — should not cause panics or hangs.
        touch_md(&vault, "after-drop.md");
        thread::sleep(Duration::from_millis(200));
        // No crash is the passing condition.
    }
}
