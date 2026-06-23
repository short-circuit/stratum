use chrono::{DateTime, Utc};
use git2::{Oid, Repository, Signature, Status};
use pkm_core::{PkmError, PkmResult};
use std::path::Path;

/// Information about a single commit.
#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub hash: String,
    pub author: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
}

/// Result of a pull operation.
#[derive(Debug, Clone)]
pub struct PullResult {
    pub success: bool,
    pub conflicts: Vec<String>,
}

/// A Git engine wrapping `git2` for PKM vault operations.
pub struct GitEngine {
    repo: Repository,
}

impl GitEngine {
    /// Open an existing repo at `path`, or initialise a new one if none exists.
    pub fn init<P: AsRef<Path>>(path: P) -> PkmResult<Self> {
        let path = path.as_ref();
        let repo = if path.join(".git").exists() {
            Repository::open(path).map_err(|e| PkmError::Git(format!("open repo: {e}")))?
        } else {
            Repository::init(path).map_err(|e| PkmError::Git(format!("init repo: {e}")))?
        };
        Ok(Self { repo })
    }

    /// Clone a remote repository to a local path.
    pub fn clone<P: AsRef<Path>>(url: &str, path: P) -> PkmResult<Self> {
        let repo = Repository::clone(url, path.as_ref())
            .map_err(|e| PkmError::Git(format!("clone repo: {e}")))?;
        Ok(Self { repo })
    }

    /// Stage one or more files (relative to the repo root).
    pub fn add(&self, paths: &[&str]) -> PkmResult<()> {
        let mut index = self
            .repo
            .index()
            .map_err(|e| PkmError::Git(format!("open index: {e}")))?;
        for p in paths {
            index
                .add_path(Path::new(p))
                .map_err(|e| PkmError::Git(format!("add {p}: {e}")))?;
        }
        index
            .write()
            .map_err(|e| PkmError::Git(format!("write index: {e}")))?;
        Ok(())
    }

    /// Create a commit on the current branch.
    /// Returns the commit hash (SHA-1 hex).
    pub fn commit(&self, message: &str, author: &str) -> PkmResult<String> {
        let sig = Signature::now(author, "sync@pkm.local")
            .map_err(|e| PkmError::Git(format!("create signature: {e}")))?;

        let mut index = self
            .repo
            .index()
            .map_err(|e| PkmError::Git(format!("open index: {e}")))?;
        let tree_oid = index
            .write_tree()
            .map_err(|e| PkmError::Git(format!("write tree: {e}")))?;
        let tree = self
            .repo
            .find_tree(tree_oid)
            .map_err(|e| PkmError::Git(format!("find tree: {e}")))?;

        let parent_commit: Option<git2::Commit> = self
            .repo
            .head()
            .ok()
            .and_then(|h| h.peel_to_commit().ok());

        let oid = if let Some(ref parent) = parent_commit {
            self.repo
                .commit(
                    Some("HEAD"),
                    &sig,
                    &sig,
                    message,
                    &tree,
                    &[parent],
                )
                .map_err(|e| PkmError::Git(format!("commit: {e}")))?
        } else {
            self.repo
                .commit(
                    Some("HEAD"),
                    &sig,
                    &sig,
                    message,
                    &tree,
                    &[] as &[&git2::Commit],
                )
                .map_err(|e| PkmError::Git(format!("commit (first): {e}")))?
        };

        Ok(oid.to_string())
    }

    /// Push the current branch to a remote.
    pub fn push(&self, remote: &str, branch: &str) -> PkmResult<()> {
        let mut rem = self
            .repo
            .find_remote(remote)
            .map_err(|e| PkmError::Git(format!("find remote {remote}: {e}")))?;
        let refspec = format!("refs/heads/{branch}:refs/heads/{branch}");
        rem.push(&[&refspec], None)
            .map_err(|e| PkmError::Git(format!("push: {e}")))?;
        Ok(())
    }

    /// Pull changes from a remote branch.
    /// Returns a `PullResult` indicating success or listing conflicting files.
    pub fn pull(&self, remote: &str, branch: &str) -> PkmResult<PullResult> {
        let mut rem = self
            .repo
            .find_remote(remote)
            .map_err(|e| PkmError::Git(format!("find remote {remote}: {e}")))?;

        // Fetch without credentials callback for now (uses defaults / ssh-agent).
        {
            let mut fetch_opts = git2::FetchOptions::new();
            fetch_opts
                .download_tags(git2::AutotagOption::All)
                .update_fetchhead(true);
            rem.fetch(&[branch], Some(&mut fetch_opts), None)
                .map_err(|e| PkmError::Git(format!("fetch: {e}")))?;
        }

        // Get the fetched commit via FETCH_HEAD
        let fetch_head_ref = self
            .repo
            .find_reference("FETCH_HEAD")
            .map_err(|e| PkmError::Git(format!("FETCH_HEAD reference: {e}")))?;
        let fetch_commit: git2::Commit = fetch_head_ref
            .peel_to_commit()
            .map_err(|e| PkmError::Git(format!("peel FETCH_HEAD: {e}")))?;
        let fetch_oid: Oid = fetch_commit.id();

        // Check if we already have the remote tip — nothing to pull.
        let local_commit: Option<git2::Commit> = self
            .repo
            .head()
            .ok()
            .and_then(|h| h.peel_to_commit().ok());

        if let Some(ref local) = local_commit {
            if local.id() == fetch_oid {
                return Ok(PullResult {
                    success: true,
                    conflicts: vec![],
                });
            }
        }

        // Merge the fetched commit into HEAD.
        let annotated = self
            .repo
            .find_annotated_commit(fetch_oid)
            .map_err(|e| PkmError::Git(format!("annotated commit: {e}")))?;

        // Try a normal merge (fast-forward possible).
        let merge_result = self.repo.merge(&[&annotated], None, None);

        match merge_result {
            Ok(()) => {
                // Check for conflicts
                let index = self
                    .repo
                    .index()
                    .map_err(|e| PkmError::Git(format!("post-merge index: {e}")))?;
                let has_conflicts = index.has_conflicts();
                let mut conflicts = Vec::new();
                if has_conflicts {
                    // Collect conflict paths
                    if let Ok(idx) = self.repo.index() {
                        if let Ok(conflict_iter) = idx.conflicts() {
                            for entry in conflict_iter.flatten() {
                                if let Some(ours) = entry.our {
                                    if let Ok(path_str) = std::str::from_utf8(&ours.path) {
                                        conflicts.push(path_str.to_string());
                                    }
                                }
                            }
                        }
                    }
                    return Ok(PullResult {
                        success: false,
                        conflicts,
                    });
                }

                // If no merge commit was created (fast-forward), update HEAD.
                let head_commit: Option<git2::Commit> = self
                    .repo
                    .head()
                    .ok()
                    .and_then(|h| h.peel_to_commit().ok());
                let already_at_fetch = head_commit.as_ref().map(|c| c.id()) == Some(fetch_oid);

                if !already_at_fetch {
                    // Fast-forward: set HEAD directly.
                    self.repo
                        .set_head_detached(fetch_oid)
                        .map_err(|e| PkmError::Git(format!("ff set_head: {e}")))?;
                    self.repo
                        .checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
                        .map_err(|e| PkmError::Git(format!("ff checkout: {e}")))?;
                }

                Ok(PullResult {
                    success: true,
                    conflicts: vec![],
                })
            }
            Err(e) => {
                // Merge failed — abort and report
                let _ = self.repo.cleanup_state();
                Err(PkmError::Git(format!("merge failed: {e}")))
            }
        }
    }

    /// Return a list of (path, status_flags) for all changed files.
    pub fn status(&self) -> PkmResult<Vec<(String, Status)>> {
        let statuses = self
            .repo
            .statuses(Some(
                git2::StatusOptions::new()
                    .include_untracked(true)
                    .recurse_untracked_dirs(true),
            ))
            .map_err(|e| PkmError::Git(format!("status: {e}")))?;

        let mut out = Vec::with_capacity(statuses.len());
        for entry in statuses.iter() {
            let path = entry
                .path()
                .map(|p| p.to_string())
                .unwrap_or_default();
            out.push((path, entry.status()));
        }
        Ok(out)
    }

    /// Retrieve the recent commit log.
    pub fn log(&self, max_count: usize) -> PkmResult<Vec<CommitInfo>> {
        let mut revwalk = self
            .repo
            .revwalk()
            .map_err(|e| PkmError::Git(format!("revwalk: {e}")))?;
        revwalk
            .push_head()
            .map_err(|e| PkmError::Git(format!("push HEAD: {e}")))?;
        revwalk.set_sorting(git2::Sort::TIME).ok();

        let mut commits = Vec::new();
        for (i, oid_res) in revwalk.enumerate() {
            if i >= max_count {
                break;
            }
            let oid: Oid = oid_res.map_err(|e| PkmError::Git(format!("walk oid: {e}")))?;
            let commit: git2::Commit = self
                .repo
                .find_commit(oid)
                .map_err(|e| PkmError::Git(format!("find commit: {e}")))?;
            let timestamp: DateTime<Utc> =
                DateTime::from_timestamp(commit.time().seconds(), 0).unwrap_or_default();
            commits.push(CommitInfo {
                hash: oid.to_string(),
                author: commit.author().name().unwrap_or("unknown").to_string(),
                message: commit.message().unwrap_or("").trim().to_string(),
                timestamp,
            });
        }
        Ok(commits)
    }

    /// Return a unified diff for a given file path.
    pub fn diff(&self, path: &str) -> PkmResult<String> {
        let diff = self
            .repo
            .diff_index_to_workdir(
                self.repo.index().ok().as_ref(),
                Some(
                    git2::DiffOptions::new()
                        .pathspec(path)
                        .show_untracked_content(true),
                ),
            )
            .map_err(|e| PkmError::Git(format!("diff: {e}")))?;

        let mut output = String::new();
        diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line: git2::DiffLine| {
            let line_str: &str = std::str::from_utf8(line.content()).unwrap_or("<binary>");
            let prefix: char = match line.origin() {
                '+' => '+',
                '-' => '-',
                ' ' => ' ',
                _ => ' ',
            };
            output.push(prefix);
            output.push_str(line_str);
            if !line_str.ends_with('\n') {
                output.push('\n');
            }
            true
        })
        .map_err(|e| PkmError::Git(format!("diff print: {e}")))?;

        Ok(output)
    }

    /// Return the remote URL for the given remote name (defaults to "origin").
    pub fn get_remote_url(&self, remote: &str) -> Option<String> {
        self.repo
            .find_remote(remote)
            .ok()
            .and_then(|r: git2::Remote| r.url().map(|u: &str| u.to_string()))
    }

    /// Set (add or update) a remote.
    pub fn set_remote(&self, name: &str, url: &str) -> PkmResult<()> {
        // Try to find existing remote; if found, update it; otherwise create.
        if self.repo.find_remote(name).is_ok() {
            self.repo
                .remote_set_url(name, url)
                .map_err(|e| PkmError::Git(format!("set remote url: {e}")))?;
        } else {
            self.repo
                .remote(name, url)
                .map_err(|e| PkmError::Git(format!("add remote: {e}")))?;
        }
        Ok(())
    }

    /// Access the underlying repository (for advanced use).
    pub fn repository(&self) -> &Repository {
        &self.repo
    }
}

#[cfg(test)]
mod tests {
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
        // status on fresh repo is empty
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

        // Write a file inside the repo
        let file_path = td.path().join("hello.md");
        fs::write(&file_path, "Hello, world!").unwrap();

        engine.add(&["hello.md"]).unwrap();
        let hash = engine.commit("Initial commit", "Test User").unwrap();
        assert_eq!(hash.len(), 40); // SHA-1 hex

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
            .any(|(p, s)| p == "new.md" && s.contains(Status::WT_NEW));
        assert!(untracked, "expected new.md to be untracked");
    }

    #[test]
    fn test_diff_shows_changes() {
        let (_td, engine) = init_repo();
        let file_path = _td.path().join("file.md");
        fs::write(&file_path, "line1\nline2\n").unwrap();
        engine.add(&["file.md"]).unwrap();
        engine.commit("base", "Tester").unwrap();

        // Modify the file
        fs::write(&file_path, "line1\nline2 changed\n").unwrap();

        let diff = engine.diff("file.md").unwrap();
        assert!(diff.contains("line2 changed"));
        assert!(diff.contains('-') || diff.contains('+'));
    }

    #[test]
    fn test_clone_and_log() {
        let td_orig = TempDir::new().unwrap();
        let orig = GitEngine::init(td_orig.path()).unwrap();

        // Create a commit on origin
        fs::write(td_orig.path().join("readme.md"), "# Test").unwrap();
        orig.add(&["readme.md"]).unwrap();
        orig.commit("first", "Alice").unwrap();

        // Clone to a temp dir
        let td_clone = TempDir::new().unwrap();
        let cloned =
            GitEngine::clone(td_orig.path().to_str().unwrap(), td_clone.path()).unwrap();
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
}
