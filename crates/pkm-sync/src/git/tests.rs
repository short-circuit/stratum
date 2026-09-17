//! Tests for the git engine.

use super::*;
use std::fs;
use tempfile::TempDir;

fn init_repo() -> (TempDir, GitEngine) {
    let td = TempDir::new().unwrap();
    let engine = GitEngine::init(td.path()).unwrap();
    (td, engine)
}

#[test]
fn test_init_new_repo() {
    let td = TempDir::new().unwrap();
    let engine = GitEngine::init(td.path()).unwrap();
    assert!(td.path().join(".git").exists());
    let s = engine.status().unwrap();
    assert!(s.is_empty());
}

#[test]
fn test_init_opens_existing_repo() {
    let td = TempDir::new().unwrap();
    let _e1 = GitEngine::init(td.path()).unwrap();
    let e2 = GitEngine::init(td.path()).unwrap();
    assert!(e2.status().is_ok());
}

#[test]
fn test_add_and_commit() {
    let td = TempDir::new().unwrap();
    let engine = GitEngine::init(td.path()).unwrap();
    let file_path = td.path().join("hello.md");
    fs::write(&file_path, "Hello, world!").unwrap();
    engine.add(&["hello.md"]).unwrap();
    let hash = engine.commit("Initial commit", "Test User").unwrap();
    assert_eq!(hash.len(), 40);
    let log = engine.log(10).unwrap();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].message, "Initial commit");
    assert_eq!(log[0].author, "Test User");
    assert_eq!(log[0].hash, hash);
}

#[test]
fn test_status_detects_untracked() {
    let (_td, engine) = init_repo();
    let file_path = _td.path().join("new.md");
    fs::write(&file_path, "content").unwrap();
    let statuses = engine.status().unwrap();
    let untracked = statuses
        .iter()
        .any(|(p, s)| p == "new.md" && s.contains(StatusFlags::WT_NEW));
    assert!(untracked, "expected new.md to be untracked");
}

#[test]
fn test_diff_shows_changes() {
    let (_td, engine) = init_repo();
    let file_path = _td.path().join("file.md");
    fs::write(&file_path, "line1\nline2\n").unwrap();
    engine.add(&["file.md"]).unwrap();
    engine.commit("base", "Tester").unwrap();
    fs::write(&file_path, "line1\nline2 changed\n").unwrap();
    let diff = engine.diff("file.md").unwrap();
    assert!(diff.contains("line2 changed"));
    assert!(diff.contains('-') || diff.contains('+'));
}

#[test]
fn test_clone_and_log() {
    let td_orig = TempDir::new().unwrap();
    let orig = GitEngine::init(td_orig.path()).unwrap();
    fs::write(td_orig.path().join("readme.md"), "# Test").unwrap();
    orig.add(&["readme.md"]).unwrap();
    orig.commit("first", "Alice").unwrap();

    let td_clone = TempDir::new().unwrap();
    let cloned = GitEngine::clone(td_orig.path().to_str().unwrap(), td_clone.path()).unwrap();
    let log = cloned.log(10).unwrap();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].message, "first");
}

#[test]
fn test_remote_url() {
    let (_td, engine) = init_repo();
    assert!(engine.get_remote_url("origin").is_none());
    engine
        .set_remote("origin", "https://example.com/repo.git")
        .unwrap();
    let url = engine.get_remote_url("origin");
    assert_eq!(url.as_deref(), Some("https://example.com/repo.git"));
}

#[test]
fn test_set_ssh_key_path() {
    let (_td, mut engine) = init_repo();
    assert!(engine.ssh_key_path().is_none());
    engine.set_ssh_key_path(Some(PathBuf::from("/tmp/test_key")));
    assert_eq!(engine.ssh_key_path(), Some(&PathBuf::from("/tmp/test_key")));
    engine.set_ssh_key_path(None);
    assert!(engine.ssh_key_path().is_none());
}

#[test]
fn test_set_passphrase() {
    let (_td, mut engine) = init_repo();
    assert!(engine.passphrase().is_none());
    engine.set_passphrase(Some("s3cret".to_string()));
    assert_eq!(engine.passphrase(), Some("s3cret"));
    engine.set_passphrase(None);
    assert!(engine.passphrase().is_none());
}

#[test]
fn test_get_current_branch_default() {
    let (_td, engine) = init_repo();
    fs::write(_td.path().join("init.md"), "init").unwrap();
    engine.add(&["init.md"]).unwrap();
    engine.commit("init", "Tester").unwrap();
    let branch = engine.get_current_branch();
    // gix creates 'main' as default branch (instead of git2's 'master')
    assert!(branch.as_deref() == Some("master") || branch.as_deref() == Some("main"));
}
#[test]
fn test_ahead_behind_zero() {
    let td_orig = TempDir::new().unwrap();
    let orig = GitEngine::init(td_orig.path()).unwrap();

    fs::write(td_orig.path().join("readme.md"), "# Test").unwrap();
    orig.add(&["readme.md"]).unwrap();
    orig.commit("first", "Alice").unwrap();

    let td_clone = TempDir::new().unwrap();
    let cloned = GitEngine::clone(td_orig.path().to_str().unwrap(), td_clone.path()).unwrap();

    // Use the branch name from the cloned repo (gix defaults to 'main')
    let branch = cloned.get_current_branch().unwrap_or_default();
    let (ahead, behind) = cloned.ahead_behind(&branch).unwrap();
    assert_eq!(ahead, 0);
    assert_eq!(behind, 0);
}

#[test]
fn test_credentials_callback_construct() {
    let (_td, engine) = init_repo();
    let _cb = engine.credentials_callback();
}
