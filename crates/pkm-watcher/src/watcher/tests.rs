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

fn wait_for_events(events: &Arc<Mutex<Vec<FileChangeEvent>>>, min: usize) -> Vec<FileChangeEvent> {
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
