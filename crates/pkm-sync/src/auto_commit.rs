use crate::git::{CommitInfo, GitEngine};
use pkm_core::PkmError;
use std::time::{Duration, Instant};
use tracing::info;

/// Tracks pending file changes and batches them into periodic commits.
pub struct AutoCommitEngine {
    git: GitEngine,
    interval: Duration,
    enabled: bool,
    pending_since: Option<Instant>,
    pending_paths: Vec<String>,
    /// Timestamp of the last successful commit.
    pub last_commit_time: Option<Instant>,
}

impl AutoCommitEngine {
    /// Create a new `AutoCommitEngine` wrapping an existing `GitEngine`.
    ///
    /// `interval_secs` controls how often pending changes are flushed.
    pub fn new(git: GitEngine, interval_secs: u64) -> Self {
        Self {
            git,
            interval: Duration::from_secs(interval_secs),
            enabled: true,
            pending_since: None,
            pending_paths: Vec::new(),
            last_commit_time: None,
        }
    }

    /// Enable auto-committing (enabled by default).
    pub fn enable(&mut self) {
        self.enabled = true;
        info!("auto-commit enabled");
    }

    /// Disable auto-committing; pending changes are retained but not flushed.
    pub fn disable(&mut self) {
        self.enabled = false;
        info!("auto-commit disabled");
    }

    /// Returns whether the engine is currently enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Record a file change. The file is staged immediately; the commit
    /// happens when `commit_pending()` is called (or the interval fires).
    pub fn record_change(&mut self, path: &str) -> Result<(), PkmError> {
        if !self.enabled {
            return Ok(());
        }
        self.git.add(&[path])?;
        if !self.pending_paths.contains(&path.to_string()) {
            self.pending_paths.push(path.to_string());
        }
        if self.pending_since.is_none() {
            self.pending_since = Some(Instant::now());
        }
        Ok(())
    }

    /// Commit all pending changes in a single batch.
    ///
    /// Returns the list of commits created (typically one, but the API is
    /// future-proof). Returns an empty vec if nothing was pending.
    pub fn commit_pending(&mut self) -> Result<Vec<CommitInfo>, PkmError> {
        if self.pending_paths.is_empty() {
            return Ok(Vec::new());
        }

        let message = if self.pending_paths.len() == 1 {
            format!("auto-commit: {}", self.pending_paths[0])
        } else {
            format!(
                "auto-commit: {} files [{}]",
                self.pending_paths.len(),
                self.pending_paths.join(", ")
            )
        };

        let _hash = self.git.commit(&message, "pkm-auto-commit")?;
        self.last_commit_time = Some(Instant::now());
        self.pending_since = None;
        self.pending_paths.clear();

        // Fetch the log to return commit info.
        let log = self.git.log(1)?;
        Ok(log)
    }

    /// Check whether the interval has elapsed and, if so, commit.
    /// This is meant to be called periodically from a timer loop.
    pub fn tick(&mut self) -> Result<Vec<CommitInfo>, PkmError> {
        if !self.enabled || self.pending_paths.is_empty() {
            return Ok(Vec::new());
        }

        if let Some(since) = self.pending_since {
            if since.elapsed() >= self.interval {
                return self.commit_pending();
            }
        }
        Ok(Vec::new())
    }

    /// Return the number of pending (staged but uncommitted) files.
    pub fn pending_count(&self) -> usize {
        self.pending_paths.len()
    }

    /// Return a reference to the underlying `GitEngine`.
    pub fn git_engine(&self) -> &GitEngine {
        &self.git
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::Duration;
    use tempfile::TempDir;

    fn setup() -> (TempDir, AutoCommitEngine) {
        let td = TempDir::new().unwrap();
        let git = GitEngine::init(td.path()).unwrap();
        let engine = AutoCommitEngine::new(git, 1); // 1-second interval
        (td, engine)
    }

    #[test]
    fn test_record_and_commit_pending() {
        let (td, mut engine) = setup();

        fs::write(td.path().join("note.md"), "# Hello").unwrap();
        engine.record_change("note.md").unwrap();
        assert_eq!(engine.pending_count(), 1);

        let commits = engine.commit_pending().unwrap();
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].author, "pkm-auto-commit");
        assert!(commits[0].message.starts_with("auto-commit:"));
        assert_eq!(engine.pending_count(), 0);
    }

    #[test]
    fn test_batch_commit_multiple_files() {
        let (td, mut engine) = setup();

        fs::write(td.path().join("a.md"), "a").unwrap();
        fs::write(td.path().join("b.md"), "b").unwrap();
        engine.record_change("a.md").unwrap();
        engine.record_change("b.md").unwrap();

        let commits = engine.commit_pending().unwrap();
        assert_eq!(commits.len(), 1);
        assert!(commits[0].message.contains("2 files"));
    }

    #[test]
    fn test_disabled_does_not_stage() {
        let (td, mut engine) = setup();
        engine.disable();
        assert!(!engine.is_enabled());

        fs::write(td.path().join("ignored.md"), "data").unwrap();
        engine.record_change("ignored.md").unwrap();
        assert_eq!(engine.pending_count(), 0);
    }

    #[test]
    fn test_tick_commits_after_interval() {
        let (td, mut engine) = setup();

        fs::write(td.path().join("tick.md"), "tick content").unwrap();
        engine.record_change("tick.md").unwrap();

        // Tick immediately — interval hasn't elapsed
        let res = engine.tick().unwrap();
        assert!(res.is_empty());

        // Wait for the interval to pass (use a short interval = 1s from setup)
        std::thread::sleep(Duration::from_millis(1100));
        let res = engine.tick().unwrap();
        assert_eq!(res.len(), 1, "expected tick to auto-commit after interval");
    }

    #[test]
    fn test_no_pending_returns_empty() {
        let (_td, mut engine) = setup();
        let commits = engine.commit_pending().unwrap();
        assert!(commits.is_empty());
    }

    #[test]
    fn test_last_commit_time_set() {
        let (td, mut engine) = setup();
        fs::write(td.path().join("time.md"), "content").unwrap();
        engine.record_change("time.md").unwrap();
        assert!(engine.last_commit_time.is_none());

        engine.commit_pending().unwrap();
        assert!(engine.last_commit_time.is_some());
    }
}
