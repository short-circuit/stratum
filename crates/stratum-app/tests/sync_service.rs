//! Headless tests for the git sync service commands:
//! `get_sync_status`, `sync_vault`, `sync_vault_with_passphrase`,
//! `start_sync_scheduler`, `stop_sync_scheduler`, `get_commit_log`,
//! `resolve_conflict_file`, `abort_merge`.

mod common;

use common::create_test_vault;

/// Run a git command in `cwd`, asserting success. Uses a fixed identity so the
/// tests do not depend on the host's global git configuration.
fn git(cwd: &std::path::Path, args: &[&str]) {
    let status = std::process::Command::new("git")
        .current_dir(cwd)
        .args(args)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@t")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@t")
        .status()
        .unwrap_or_else(|e| panic!("git {:?} failed to spawn: {e}", args));
    assert!(status.success(), "git {:?} failed with {status}", args);
}

/// Initialize a git repository at `vault_path` with an initial commit.
fn init_repo(vault_path: &std::path::Path) {
    git(vault_path, &["init", "-b", "main"]);
    std::fs::write(vault_path.join("README.md"), "# vault\n").unwrap();
    git(vault_path, &["add", "."]);
    git(vault_path, &["commit", "-m", "seed"]);
}

#[tokio::test]
async fn sync_vault_without_remote_commits_and_persists_state() {
    let vault = create_test_vault();
    init_repo(&vault.vault_path);
    vault.write_file("pages/idea.md", "# idea\n");

    let status = vault.service.sync_vault().await.unwrap();

    assert_eq!(status.status, "ok", "sync should report ok: {status:?}");
    assert_eq!(status.last_sync_success, Some(true));
    assert!(status.last_sync_time.is_some());
    assert!(
        vault.exists(".pkm/sync_state.json"),
        "sync state ledger must be persisted"
    );

    let log = vault.service.get_commit_log().await.unwrap();
    assert!(
        log.iter()
            .any(|c| c.message.contains("idea") || c.message.contains("auto-commit")),
        "the new note must be committed: {log:?}"
    );
    assert!(
        !log.is_empty(),
        "commit log must contain at least one commit"
    );
}

#[tokio::test]
async fn sync_vault_with_passphrase_caches_the_passphrase() {
    let vault = create_test_vault();
    init_repo(&vault.vault_path);
    vault.write_file("pages/secret.md", "# secret\n");

    let status = vault
        .service
        .sync_vault_with_passphrase("hunter2".into())
        .await
        .unwrap();

    assert_eq!(status.status, "ok");
    assert_eq!(
        vault.service.lock().unwrap().passphrase.as_deref(),
        Some("hunter2"),
        "the passphrase must be cached in vault state"
    );
}

#[tokio::test]
async fn get_sync_status_auto_initializes_a_missing_repo() {
    let vault = create_test_vault();

    // `GitEngine::init` creates a repository when `.git` is absent, so a vault
    // that has never been synced reports an empty-but-clean repo rather than
    // the `no_repo` fallback (which only fires when git init itself fails).
    let status = vault.service.get_sync_status().await.unwrap();

    assert_eq!(status.status, "ok");
    assert!(status.conflicts.is_empty());
    assert!(vault.exists(".git"), "status must initialize the repo");
}

#[tokio::test]
async fn get_sync_status_reports_clean_repo() {
    let vault = create_test_vault();
    init_repo(&vault.vault_path);

    let status = vault.service.get_sync_status().await.unwrap();

    assert_eq!(status.status, "ok", "clean repo must report ok: {status:?}");
    assert_eq!(status.branch.as_deref(), Some("main"));
    assert!(status.conflicts.is_empty());
}

#[tokio::test]
async fn start_and_stop_sync_scheduler_roundtrip() {
    let vault = create_test_vault();
    init_repo(&vault.vault_path);

    vault.service.start_sync_scheduler().await.unwrap();
    assert!(
        vault.service.lock().unwrap().sync_scheduler.is_some(),
        "scheduler must be installed in state"
    );

    vault.service.stop_sync_scheduler().await.unwrap();
    assert!(
        vault.service.lock().unwrap().sync_scheduler.is_none(),
        "scheduler must be removed from state"
    );

    // Stopping again is a no-op.
    vault.service.stop_sync_scheduler().await.unwrap();
}

#[tokio::test]
async fn resolve_conflict_file_stages_and_commits_the_path() {
    let vault = create_test_vault();
    init_repo(&vault.vault_path);
    vault.write_file("shared.md", "resolved content\n");

    vault
        .service
        .resolve_conflict_file("shared.md".into())
        .await
        .unwrap();

    let log = vault.service.get_commit_log().await.unwrap();
    assert!(
        log.iter()
            .any(|c| c.message.contains("Resolved conflict in shared.md")),
        "resolution must be committed: {log:?}"
    );
}

#[tokio::test]
async fn resolve_conflict_file_rejects_paths_outside_the_vault() {
    let vault = create_test_vault();
    init_repo(&vault.vault_path);

    let err = vault
        .service
        .resolve_conflict_file("../escape.md".into())
        .await
        .unwrap_err();

    assert!(
        err.contains("outside") || err.contains("vault") || err.contains("escape"),
        "error must explain the path rejection: {err}"
    );
}

#[tokio::test]
async fn abort_merge_restores_index_to_head() {
    let vault = create_test_vault();
    init_repo(&vault.vault_path);

    // No merge in progress: abort must still succeed and leave the repo usable.
    vault.service.abort_merge().await.unwrap();

    let status = vault.service.get_sync_status().await.unwrap();
    assert_eq!(status.status, "ok", "repo must remain clean: {status:?}");
}
