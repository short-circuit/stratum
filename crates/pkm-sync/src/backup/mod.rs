//! Periodic cloud backup (E10.F3) as an independent mirror layer.
//!
//! Backup is deliberately decoupled from git sync: it copies vault files to a
//! local folder or a WebDAV endpoint as a differential per-file mirror, never
//! touching git state or the search index. It does not produce a zip archive —
//! instead it mirrors the file tree and tracks what it has written in a
//! `manifest.json` at the target root. Delete propagation only removes files
//! that were recorded in the previous manifest and no longer exist locally, so
//! a destination containing unrelated data is never disturbed.
//!
//! Modules:
//! - [`folder`] — `backup_to_folder`, a local-directory differential mirror.
//! - [`webdav`] — `backup_to_webdav` / `backup_to_webdav_blocking`, a WebDAV
//!   differential mirror.
//!
//! [`BackupScheduler`] runs either target on a timer, mirroring
//! `crate::scheduler::SyncScheduler`, but is entirely independent of git.

pub mod folder;
pub mod webdav;

use chrono::{DateTime, Utc};
use pkm_core::{BackupConfig, BackupMode, PkmError};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, UNIX_EPOCH};
use tracing::{error, info, warn};

/// Metadata about a single backed-up file, recorded in `manifest.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMeta {
    /// File size in bytes.
    pub size: u64,
    /// Last-modified time as Unix seconds.
    pub mtime: u64,
}

/// The backup manifest written at the target root.
///
/// `files` is a `BTreeMap` to keep the serialized output deterministic across
/// runs, which keeps the manifest stable and makes diffs meaningful.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    /// Schema version (currently 1).
    pub version: u32,
    /// RFC 3339 generation timestamp.
    pub generated_at: String,
    /// Vault-relative path → file metadata.
    pub files: BTreeMap<String, FileMeta>,
}

impl Default for Manifest {
    fn default() -> Self {
        Self {
            version: 1,
            generated_at: Utc::now().to_rfc3339(),
            files: BTreeMap::new(),
        }
    }
}

/// Result of a single backup run against a target.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupResult {
    /// Whether the backup completed without error.
    pub success: bool,
    /// Which target was used: `"folder"`, `"webdav"`, or `"both"`.
    pub target: String,
    /// Number of files copied/uploaded.
    pub files_copied: usize,
    /// Number of files already up to date (skipped).
    pub files_unchanged: usize,
    /// Number of files removed at the target.
    pub files_deleted: usize,
    /// Run start time.
    pub started_at: DateTime<Utc>,
    /// Run finish time.
    pub finished_at: DateTime<Utc>,
    /// Error message when `success` is false.
    pub error: Option<String>,
}

impl BackupResult {
    /// Construct a failed result for the given target, capturing the error
    /// string and the run timestamps.
    pub fn failure(target: &str, error: String, started_at: DateTime<Utc>) -> Self {
        Self {
            success: false,
            target: target.to_string(),
            files_copied: 0,
            files_unchanged: 0,
            files_deleted: 0,
            started_at,
            finished_at: Utc::now(),
            error: Some(error),
        }
    }
}

/// Return whether a vault-relative path should be excluded from backup.
///
/// `rel` is a forward-slash path relative to the vault root. A path is
/// excluded when it, or any of its parent directories, matches an entry in
/// `exclude`. `.git` is always excluded even if the config omits it.
pub fn should_exclude(rel: &str, exclude: &[String]) -> bool {
    if rel == ".git" || rel.starts_with(".git/") {
        return true;
    }
    let rel_compact = rel.trim_end_matches('/');
    exclude.iter().any(|candidate| {
        let candidate = candidate.trim_end_matches('/');
        rel_compact == candidate || rel_compact.starts_with(&format!("{}/", candidate))
    })
}

/// A file discovered during the source walk: absolute path, vault-relative
/// forward-slash path, size, and last-modified time (Unix seconds).
pub type CollectEntry = (PathBuf, String, u64, u64);

/// Deterministically walk `vault` and collect files eligible for backup.
///
/// The walk is breadth-first over sorted directory entries so the output order
/// is stable. Entries under `.git`, under any entry in `exclude`, and under
/// `self_skip` (the destination when it lives inside the vault, preventing
/// recursive self-copy) are skipped; only regular files are returned.
pub fn collect_files(
    vault: &Path,
    exclude: &[String],
    self_skip: Option<&Path>,
) -> Result<Vec<CollectEntry>, PkmError> {
    let skip_canon = self_skip
        .and_then(|p| p.canonicalize().ok())
        .unwrap_or_default();
    let mut out: Vec<CollectEntry> = Vec::new();
    let mut stack: Vec<PathBuf> = vec![vault.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let mut entries: Vec<std::fs::DirEntry> = std::fs::read_dir(&dir)
            .map_err(PkmError::Io)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(PkmError::Io)?;
        entries.sort_by_key(|e| e.file_name());

        for entry in entries {
            let path = entry.path();
            let rel = path
                .strip_prefix(vault)
                .map(|p| p.to_string_lossy().replace(std::path::MAIN_SEPARATOR, "/"))
                .map_err(|e| PkmError::Internal(e.to_string()))?;
            let file_type = entry.file_type().map_err(PkmError::Io)?;

            if file_type.is_dir() {
                // Skip the destination subtree when it sits inside the vault.
                if !skip_canon.as_os_str().is_empty()
                    && path
                        .canonicalize()
                        .map(|c| c.starts_with(&skip_canon))
                        .unwrap_or(false)
                {
                    continue;
                }
                if should_exclude(&rel, exclude) {
                    continue;
                }
                stack.push(path);
            } else if file_type.is_file() {
                if should_exclude(&rel, exclude) {
                    continue;
                }
                let meta = entry.metadata().map_err(PkmError::Io)?;
                let size = meta.len();
                let mtime = meta
                    .modified()
                    .map(|m| {
                        m.duration_since(UNIX_EPOCH)
                            .map(|d| d.as_secs())
                            .unwrap_or(0)
                    })
                    .unwrap_or(0);
                out.push((path, rel, size, mtime));
            }
        }
    }

    out.sort_by(|a, b| a.1.cmp(&b.1));
    Ok(out)
}

/// Load the manifest from `dir` (the target root).
///
/// Returns `None` when the manifest is absent or unreadable/corrupt — both
/// cases are treated as a first run, so nothing is deleted at the target.
pub fn load_manifest(dir: &Path) -> Option<Manifest> {
    let content = std::fs::read_to_string(dir.join("manifest.json")).ok()?;
    serde_json::from_str(&content).ok()
}

/// Atomically write the manifest to `dir` (the target root).
///
/// Writes to a `.part` file first, then renames into place so a crash mid-write
/// never leaves a truncated manifest behind.
pub fn write_manifest(dir: &Path, files: &BTreeMap<String, FileMeta>) -> Result<(), PkmError> {
    let manifest = Manifest {
        version: 1,
        generated_at: Utc::now().to_rfc3339(),
        files: files.clone(),
    };
    let json = serde_json::to_string_pretty(&manifest)?;
    let part = dir.join("manifest.json.part");
    std::fs::write(&part, json)?;
    std::fs::rename(&part, dir.join("manifest.json"))?;
    Ok(())
}

/// Internal mutable state shared with the background scheduler thread.
#[derive(Default)]
pub struct BackupInner {
    /// The most recent [`BackupResult`] (if any).
    pub last_result: Option<BackupResult>,
    /// Timestamp of the most recent run (if any).
    pub last_run: Option<DateTime<Utc>>,
    /// Whether the scheduler thread is currently running.
    pub running: bool,
}

/// Periodic backup scheduler that runs a configured target on a timer.
///
/// Mirrors [`crate::scheduler::SyncScheduler`] in structure, but is fully
/// independent of git: it only mirrors vault files to the configured target.
/// Backup runs are single-threaded — the scheduler guards a single in-flight
/// run with the internal mutex, so a long upload never overlaps an interval
/// tick.
pub struct BackupScheduler {
    vault: PathBuf,
    config: BackupConfig,
    inner: Arc<Mutex<BackupInner>>,
    running: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
    stop_tx: Option<mpsc::Sender<()>>,
}

impl BackupScheduler {
    /// Create a new backup scheduler for `vault` using the given config.
    pub fn new(vault: PathBuf, config: BackupConfig) -> Self {
        Self {
            vault,
            config,
            inner: Arc::new(Mutex::new(BackupInner::default())),
            running: Arc::new(AtomicBool::new(false)),
            handle: None,
            stop_tx: None,
        }
    }

    /// Run the backup once, honoring the configured `BackupMode`.
    ///
    /// For `Both`, the folder target runs first and the WebDAV target second;
    /// the two results are merged into one: counts are summed, `target` is
    /// `"both"`, `success` is the conjunction of both, and `error` carries the
    /// first error encountered (folder first, then webdav) when either failed.
    pub fn run_now(&self) -> BackupResult {
        let started = Utc::now();
        let exclude = self.config.exclude.clone();

        let result = match self.config.mode {
            BackupMode::Folder => self.run_folder(&exclude, started),
            BackupMode::Webdav => self.run_webdav(&exclude, started),
            BackupMode::Both => {
                let folder = self.run_folder(&exclude, started);
                let webdav = self.run_webdav(&exclude, started);
                merge_both(folder, webdav)
            }
        };

        if let Ok(mut inner) = self.inner.lock() {
            inner.last_result = Some(result.clone());
            inner.last_run = Some(Utc::now());
        }
        result
    }

    fn run_folder(&self, exclude: &[String], started: DateTime<Utc>) -> BackupResult {
        match &self.config.folder.path {
            Some(p) if !p.is_empty() => {
                match crate::backup::folder::backup_to_folder(&self.vault, Path::new(p), exclude) {
                    Ok(result) => result,
                    Err(e) => BackupResult::failure("folder", e.to_string(), started),
                }
            }
            _ => {
                BackupResult::failure("folder", "No backup folder path configured".into(), started)
            }
        }
    }

    fn run_webdav(&self, exclude: &[String], started: DateTime<Utc>) -> BackupResult {
        match crate::backup::webdav::backup_to_webdav_blocking(
            &self.vault,
            &self.config.webdav,
            exclude,
        ) {
            Ok(result) => result,
            Err(e) => BackupResult::failure("webdav", e.to_string(), started),
        }
    }

    /// Start the scheduler thread.
    ///
    /// The loop sleeps for `interval_minutes` (min 60s, 0 → 60) then calls
    /// `run_now`, until `stop()` is called or the scheduler is dropped.
    pub fn start(&mut self) {
        if self.is_running() {
            warn!("backup scheduler is already running");
            return;
        }
        self.running.store(true, Ordering::SeqCst);
        let running = self.running.clone();
        let inner = self.inner.clone();
        let vault = self.vault.clone();
        let config = self.config.clone();
        let interval = Duration::from_secs(config.interval_minutes.max(1) * 60);

        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let handle = std::thread::Builder::new()
            .name("pkm-backup-scheduler".into())
            .spawn(move || {
                info!(
                    "backup scheduler thread started (mode={:?}, interval_minutes={})",
                    config.mode, config.interval_minutes
                );
                loop {
                    let sleep_start = std::time::Instant::now();
                    let mut stop_requested = false;
                    while sleep_start.elapsed() < interval {
                        std::thread::sleep(Duration::from_millis(200));
                        if stop_rx.try_recv().is_ok() || !running.load(Ordering::SeqCst) {
                            stop_requested = true;
                            break;
                        }
                    }
                    if stop_requested || !running.load(Ordering::SeqCst) {
                        break;
                    }
                    let scheduler = BackupScheduler {
                        vault: vault.clone(),
                        config: config.clone(),
                        inner: inner.clone(),
                        running: running.clone(),
                        handle: None,
                        stop_tx: None,
                    };
                    let result = scheduler.run_now();
                    if result.success {
                        info!(
                            "backup run completed: target={}, copied={}, deleted={}",
                            result.target, result.files_copied, result.files_deleted
                        );
                    } else {
                        error!(
                            "backup run failed (target={}): {}",
                            result.target,
                            result.error.as_deref().unwrap_or("unknown")
                        );
                    }
                }
                info!("backup scheduler thread stopped");
            })
            .expect("failed to spawn backup scheduler thread");

        self.handle = Some(handle);
        self.stop_tx = Some(stop_tx);
        if let Ok(mut inner) = self.inner.lock() {
            inner.running = true;
        }
        info!(
            "backup scheduler started (interval_minutes={})",
            self.config.interval_minutes
        );
    }

    /// Stop the scheduler, signalling the thread and waiting for it to finish.
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
        if let Ok(mut inner) = self.inner.lock() {
            inner.running = false;
        }
        info!("backup scheduler stopped");
    }

    /// Whether the scheduler thread is currently running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// The most recent backup result, if any.
    pub fn last_result(&self) -> Option<BackupResult> {
        self.inner.lock().ok().and_then(|g| g.last_result.clone())
    }

    /// Timestamp of the most recent backup run, if any.
    pub fn last_run(&self) -> Option<DateTime<Utc>> {
        self.inner.lock().ok().and_then(|g| g.last_run)
    }
}

impl Drop for BackupScheduler {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Merge a folder run and a webdav run into a single `"both"` result.
fn merge_both(folder: BackupResult, webdav: BackupResult) -> BackupResult {
    let error = if folder.error.is_some() {
        folder.error.clone()
    } else {
        webdav.error.clone()
    };
    BackupResult {
        success: folder.success && webdav.success,
        target: "both".to_string(),
        files_copied: folder.files_copied + webdav.files_copied,
        files_unchanged: folder.files_unchanged + webdav.files_unchanged,
        files_deleted: folder.files_deleted + webdav.files_deleted,
        started_at: folder.started_at,
        finished_at: webdav.finished_at,
        error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pkm_core::default_backup_exclude;
    use std::fs;
    use tempfile::TempDir;

    pub(super) fn setup_vault(td: &TempDir) -> PathBuf {
        let vault = td.path().join("vault");
        fs::create_dir_all(vault.join("pages/sub")).unwrap();
        fs::create_dir_all(vault.join("assets")).unwrap();
        fs::create_dir_all(vault.join(".pkm/search")).unwrap();
        fs::create_dir_all(vault.join(".git")).unwrap();
        fs::write(vault.join("pages/a.md"), "# A\n").unwrap();
        fs::write(vault.join("pages/sub/b.md"), "# B\n").unwrap();
        fs::write(vault.join("assets/x.png"), "png").unwrap();
        fs::write(vault.join(".pkm/blocks.db"), "db").unwrap();
        fs::write(vault.join(".pkm/search/w"), "idx").unwrap();
        fs::write(vault.join(".pkm/config.toml"), "[theme]\n").unwrap();
        fs::write(vault.join(".git/config"), "git").unwrap();
        fs::write(vault.join(".DS_Store"), "ds").unwrap();
        vault
    }

    #[test]
    fn test_backup_config_defaults() {
        let cfg = BackupConfig::default();
        assert!(!cfg.enabled);
        assert_eq!(cfg.mode, BackupMode::Folder);
        assert_eq!(cfg.interval_minutes, 60);
        assert_eq!(cfg.exclude, default_backup_exclude());
        assert_eq!(cfg.exclude.len(), 6);
        assert_eq!(cfg.exclude[0], ".git");
    }

    #[test]
    fn test_should_exclude() {
        let exclude = default_backup_exclude();
        assert!(should_exclude(".git", &exclude));
        assert!(should_exclude(".git/config", &exclude));
        assert!(should_exclude(".git", &[]));
        assert!(should_exclude(".git/config", &[]));
        assert!(should_exclude(".pkm/blocks.db", &exclude));
        assert!(should_exclude(".pkm/search/w", &exclude));
        assert!(should_exclude(".DS_Store", &exclude));
        assert!(!should_exclude("pages/a.md", &exclude));
        assert!(!should_exclude(".pkm/config.toml", &exclude));
    }

    #[test]
    fn test_manifest_roundtrip() {
        let td = TempDir::new().unwrap();
        let dir = td.path().join("backup");
        fs::create_dir_all(&dir).unwrap();
        let mut files = BTreeMap::new();
        files.insert(
            "pages/a.md".into(),
            FileMeta {
                size: 3,
                mtime: 100,
            },
        );
        write_manifest(&dir, &files).unwrap();
        let loaded = load_manifest(&dir).unwrap();
        assert_eq!(loaded.version, 1);
        assert_eq!(loaded.files.len(), 1);
        assert_eq!(loaded.files["pages/a.md"].size, 3);
        // Corrupt manifest → None.
        fs::write(dir.join("manifest.json"), "not json").unwrap();
        assert!(load_manifest(&dir).is_none());
    }

    #[test]
    fn test_collect_files_excludes() {
        let td = TempDir::new().unwrap();
        let vault = setup_vault(&td);
        let exclude = default_backup_exclude();
        let files = collect_files(&vault, &exclude, None).unwrap();
        let rels: Vec<String> = files.iter().map(|f| f.1.clone()).collect();
        assert!(rels.contains(&"pages/a.md".to_string()));
        assert!(rels.contains(&"pages/sub/b.md".to_string()));
        assert!(rels.contains(&"assets/x.png".to_string()));
        assert!(rels.contains(&".pkm/config.toml".to_string()));
        assert!(!rels.contains(&".git/config".to_string()));
        assert!(!rels.contains(&".pkm/blocks.db".to_string()));
        assert!(!rels.contains(&".pkm/search/w".to_string()));
        assert!(!rels.contains(&".DS_Store".to_string()));
        assert_eq!(rels.len(), 4);
    }

    #[test]
    fn test_collect_files_self_skip() {
        let td = TempDir::new().unwrap();
        let vault = setup_vault(&td);
        let dest = vault.join("backups");
        fs::create_dir_all(&dest).unwrap();
        fs::write(dest.join("old.txt"), "old").unwrap();
        let exclude = default_backup_exclude();
        let files = collect_files(&vault, &exclude, Some(&dest)).unwrap();
        let rels: Vec<String> = files.iter().map(|f| f.1.clone()).collect();
        assert!(!rels.contains(&"backups".to_string()));
        assert!(!rels.contains(&"backups/old.txt".to_string()));
        assert!(rels.contains(&"pages/a.md".to_string()));
    }
}

#[cfg(test)]
mod functional {
    use super::tests::setup_vault;
    use super::*;
    use crate::backup::folder::backup_to_folder;
    use pkm_core::default_backup_exclude;
    use pkm_core::WebdavBackupConfig;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_folder_backup_copies_and_excludes() {
        let td = TempDir::new().unwrap();
        let vault = setup_vault(&td);
        let dest = td.path().join("backup");
        let exclude = default_backup_exclude();
        let result = backup_to_folder(&vault, &dest, &exclude).unwrap();
        assert!(result.success, "{:?}", result.error);
        assert!(result.files_copied >= 4);
        assert!(dest.join("pages/a.md").is_file());
        assert!(dest.join("pages/sub/b.md").is_file());
        assert!(dest.join("assets/x.png").is_file());
        assert!(dest.join(".pkm/config.toml").is_file());
        assert!(!dest.join(".git/config").exists());
        assert!(!dest.join(".pkm/blocks.db").exists());
        assert!(!dest.join(".pkm/search/w").exists());
        assert!(!dest.join(".DS_Store").exists());
        assert!(dest.join("manifest.json").is_file());
    }

    #[test]
    fn test_folder_incremental_idempotent() {
        let td = TempDir::new().unwrap();
        let vault = setup_vault(&td);
        let dest = td.path().join("backup");
        let exclude = default_backup_exclude();

        let first = backup_to_folder(&vault, &dest, &exclude).unwrap();
        assert!(first.success);
        let total = first.files_copied;

        // Modify one file → only that file is re-copied.
        fs::write(vault.join("pages/a.md"), "# A (modified)").unwrap();
        let second = backup_to_folder(&vault, &dest, &exclude).unwrap();
        assert!(second.success);
        assert_eq!(second.files_copied, 1);
        assert_eq!(second.files_unchanged, total - 1);

        // No changes → idempotent: nothing copied, everything unchanged.
        let third = backup_to_folder(&vault, &dest, &exclude).unwrap();
        assert!(third.success);
        assert_eq!(third.files_copied, 0);
        assert_eq!(third.files_unchanged, total);
    }

    #[test]
    fn test_folder_delete_propagation() {
        let td = TempDir::new().unwrap();
        let vault = setup_vault(&td);
        let dest = td.path().join("backup");
        let exclude = default_backup_exclude();

        let first = backup_to_folder(&vault, &dest, &exclude).unwrap();
        assert!(first.success);

        // Pre-existing unrelated file at the destination must be left untouched.
        fs::write(dest.join("manual.txt"), "manual").unwrap();

        fs::remove_file(vault.join("pages/a.md")).unwrap();
        let second = backup_to_folder(&vault, &dest, &exclude).unwrap();
        assert!(second.success);
        assert_eq!(second.files_deleted, 1);
        assert!(!dest.join("pages/a.md").exists());
        assert!(dest.join("manual.txt").is_file());
        assert!(dest.join("pages/sub/b.md").is_file());

        // Manifest no longer lists the deleted file.
        let manifest = load_manifest(&dest).unwrap();
        assert!(!manifest.files.contains_key("pages/a.md"));
    }

    #[test]
    fn test_folder_self_skip_functional() {
        let td = TempDir::new().unwrap();
        let vault = setup_vault(&td);
        let dest = vault.join("backups");
        let exclude = default_backup_exclude();

        let result = backup_to_folder(&vault, &dest, &exclude).unwrap();
        assert!(result.success, "{:?}", result.error);
        assert!(dest.join("pages/a.md").is_file());
        assert!(dest.join("pages/sub/b.md").is_file());
        // The first run must not copy the dest into itself.
        assert!(!dest.join("backups/pages/a.md").exists());
    }

    #[tokio::test]
    async fn test_webdav_roundtrip() {
        use dav_server::{fakels::FakeLs, localfs::LocalFs, DavHandler};
        use hyper::server::conn::http1;
        use hyper::service::service_fn;
        use hyper_util::rt::TokioIo;
        use std::convert::Infallible;
        use tokio::net::TcpListener;

        let server_dir = TempDir::new().unwrap();
        let dav_server = DavHandler::builder()
            .filesystem(LocalFs::new(server_dir.path(), false, false, false))
            .locksystem(FakeLs::new())
            .build_handler();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            loop {
                let (stream, _) = listener.accept().await.unwrap();
                let dav_server = dav_server.clone();
                let io = TokioIo::new(stream);
                tokio::spawn(async move {
                    let _ = http1::Builder::new()
                        .serve_connection(
                            io,
                            service_fn(move |req| {
                                let dav_server = dav_server.clone();
                                async move { Ok::<_, Infallible>(dav_server.handle(req).await) }
                            }),
                        )
                        .await;
                });
            }
        });

        let url = format!("http://{}", addr);
        let cfg = WebdavBackupConfig {
            url: Some(url.clone()),
            username: Some("user".into()),
            password: Some("pw".into()),
        };
        let exclude = vec![".git".to_string(), ".DS_Store".to_string()];

        let vault_dir = TempDir::new().unwrap();
        let vault = vault_dir.path().join("vault");
        fs::create_dir_all(vault.join("pages/sub")).unwrap();
        fs::write(vault.join("pages/a.md"), "A").unwrap();
        fs::write(vault.join("pages/sub/b.md"), "B").unwrap();

        // First run: upload both files + manifest.
        let first = crate::backup::webdav::backup_to_webdav(&vault, &cfg, &exclude)
            .await
            .unwrap();
        assert!(first.success, "{:?}", first.error);
        assert_eq!(first.files_copied, 2);
        assert!(server_dir.path().join("pages/a.md").is_file());
        assert!(server_dir.path().join("pages/sub/b.md").is_file());
        assert!(server_dir.path().join("manifest.json").is_file());

        // Modify one file → only that file is re-uploaded.
        fs::write(vault.join("pages/a.md"), "A modified").unwrap();
        let second = crate::backup::webdav::backup_to_webdav(&vault, &cfg, &exclude)
            .await
            .unwrap();
        assert!(second.success);
        assert_eq!(second.files_copied, 1);

        // Delete a local file → its remote copy is removed; a pre-existing
        // remote-only file is left untouched.
        fs::write(server_dir.path().join("remote_only.txt"), "keep").unwrap();
        fs::remove_file(vault.join("pages/sub/b.md")).unwrap();
        let third = crate::backup::webdav::backup_to_webdav(&vault, &cfg, &exclude)
            .await
            .unwrap();
        assert!(third.success);
        assert!(!server_dir.path().join("pages/sub/b.md").exists());
        assert!(server_dir.path().join("remote_only.txt").is_file());
    }
}
