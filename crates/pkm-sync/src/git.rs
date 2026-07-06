use chrono::{DateTime, Utc};
use gix::bstr::{ByteSlice, ByteVec};
use pkm_core::{PkmError, PkmResult};
use std::ops::BitOr;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusFlags(u32);

impl StatusFlags {
    pub const CURRENT: Self = Self(0);
    pub const INDEX_NEW: Self = Self(1 << 0);
    pub const INDEX_MODIFIED: Self = Self(1 << 1);
    pub const INDEX_DELETED: Self = Self(1 << 2);
    pub const INDEX_RENAMED: Self = Self(1 << 3);
    pub const INDEX_TYPECHANGE: Self = Self(1 << 4);
    pub const WT_NEW: Self = Self(1 << 5);
    pub const WT_MODIFIED: Self = Self(1 << 6);
    pub const WT_DELETED: Self = Self(1 << 7);
    pub const WT_RENAMED: Self = Self(1 << 8);
    pub const WT_TYPECHANGE: Self = Self(1 << 9);
    pub const CONFLICTED: Self = Self(1 << 10);
    pub const IGNORED: Self = Self(1 << 11);

    pub fn is_conflicted(&self) -> bool {
        self.0 & Self::CONFLICTED.0 != 0
    }
    pub fn intersects(&self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
    pub fn contains(&self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
    pub fn is_current(&self) -> bool {
        self.0 == 0
    }
}

impl BitOr for StatusFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub hash: String,
    pub author: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct PullResult {
    pub success: bool,
    pub conflicts: Vec<String>,
}

pub struct GitEngine {
    repo: gix::Repository,
    ssh_key_path: Option<PathBuf>,
    passphrase: Option<String>,
}

impl GitEngine {
    pub fn init<P: AsRef<Path>>(path: P) -> PkmResult<Self> {
        let path = path.as_ref();
        let repo = if path.join(".git").exists() {
            gix::open(path).map_err(|e| PkmError::Git(format!("open repo: {e}")))?
        } else {
            gix::init(path).map_err(|e| PkmError::Git(format!("init repo: {e}")))?
        };
        Ok(Self {
            repo,
            ssh_key_path: None,
            passphrase: None,
        })
    }

    pub fn clone<P: AsRef<Path>>(url: &str, path: P) -> PkmResult<Self> {
        let mut prep = gix::prepare_clone(url, path.as_ref())
            .map_err(|e| PkmError::Git(format!("prepare clone: {e}")))?;
        let (mut prep_co, _) = prep
            .fetch_then_checkout(gix::progress::Discard, &AtomicBool::new(false))
            .map_err(|e| PkmError::Git(format!("clone fetch: {e}")))?;
        let (repo, _) = prep_co
            .main_worktree(gix::progress::Discard, &AtomicBool::new(false))
            .map_err(|e| PkmError::Git(format!("clone checkout: {e}")))?;
        Ok(Self {
            repo,
            ssh_key_path: None,
            passphrase: None,
        })
    }

    pub fn add(&self, paths: &[&str]) -> PkmResult<()> {
        let workdir = self
            .repo
            .workdir()
            .ok_or_else(|| PkmError::Git("no working directory".into()))?
            .to_path_buf();

        let index_file = self
            .repo
            .open_index()
            .or_else(|_| {
                Ok(gix::index::File::from_state(
                    gix::index::State::new(self.repo.object_hash()),
                    self.repo.index_path(),
                ))
            })
            .map_err(|e: gix::index::file::init::Error| {
                PkmError::Git(format!("open index: {e}"))
            })?;

        let mut state = index_file.into_parts().0;

        for p in paths {
            let full_path = workdir.join(p);
            let content =
                std::fs::read(&full_path).map_err(|e| PkmError::Git(format!("read {p}: {e}")))?;
            let blob_id = self
                .repo
                .write_blob(&content)
                .map_err(|e| PkmError::Git(format!("write blob {p}: {e}")))?;

            let meta = std::fs::metadata(&full_path)
                .map_err(|e| PkmError::Git(format!("stat {p}: {e}")))?;

            let mode = if meta.file_type().is_symlink() {
                gix::index::entry::Mode::SYMLINK
            } else {
                gix::index::entry::Mode::FILE
            };

            let (mtime_secs, mtime_nsecs) = meta
                .modified()
                .ok()
                .map(|t| {
                    let d = t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
                    (d.as_secs() as u32, d.subsec_nanos())
                })
                .unwrap_or((0, 0));

            state.dangerously_push_entry(
                gix::index::entry::Stat {
                    mtime: gix::index::entry::stat::Time {
                        secs: mtime_secs,
                        nsecs: mtime_nsecs,
                    },
                    ctime: gix::index::entry::stat::Time::default(),
                    dev: 0,
                    ino: 0,
                    uid: 0,
                    gid: 0,
                    size: meta.len() as u32,
                },
                blob_id.detach(),
                gix::index::entry::Flags::empty(),
                mode,
                p.as_ref(),
            );
        }

        state.sort_entries();

        let mut new_index = gix::index::File::from_state(state, self.repo.index_path());
        new_index
            .write(gix::index::write::Options {
                extensions: gix::index::write::Extensions::default(),
                skip_hash: false,
            })
            .map_err(|e| PkmError::Git(format!("write index: {e}")))?;

        Ok(())
    }

    pub fn commit(&self, message: &str, author: &str) -> PkmResult<String> {
        let tree_id = self
            .write_tree_from_index()
            .map_err(|e| PkmError::Git(format!("build tree: {e}")))?;

        let parents: Vec<gix::ObjectId> = self
            .repo
            .head()
            .ok()
            .and_then(|mut h| h.peel_to_commit().ok())
            .map(|c| c.id().detach())
            .into_iter()
            .collect();

        let sig = gix::actor::Signature {
            name: author.into(),
            email: "sync@pkm.local".into(),
            time: gix::date::Time::now_utc(),
        };
        let mut time_buf = gix::date::parse::TimeBuf::default();
        let sig_ref = sig.to_ref(&mut time_buf);

        let commit_id = self
            .repo
            .commit_as(sig_ref, sig_ref, "HEAD", message, tree_id, parents)
            .map_err(|e| PkmError::Git(format!("commit: {e}")))?;

        Ok(commit_id.to_string())
    }

    fn write_tree_from_index(&self) -> PkmResult<gix::ObjectId> {
        let index_file = self
            .repo
            .open_index()
            .map_err(|e| PkmError::Git(format!("open index: {e}")))?;
        let state: &gix::index::State = &index_file;

        if state.entries().is_empty() {
            return Ok(gix::ObjectId::empty_tree(self.repo.object_hash()));
        }

        let backing = state.path_backing();
        let mut entries: Vec<gix::objs::tree::Entry> = state
            .entries()
            .iter()
            .map(|entry| {
                let mode_val: u32 = entry.mode.bits();
                let file_mode = 0o100644u32;
                gix::objs::tree::Entry {
                    mode: gix::objs::tree::EntryMode::try_from(mode_val).unwrap_or_else(|_| {
                        gix::objs::tree::EntryMode::try_from(file_mode).unwrap()
                    }),
                    filename: entry.path_in(backing).to_owned(),
                    oid: entry.id,
                }
            })
            .collect();

        entries.sort_by(|a, b| a.filename.cmp(&b.filename));

        let tree = gix::objs::Tree { entries };
        let tree_id = self
            .repo
            .write_object(&tree)
            .map_err(|e| PkmError::Git(format!("write tree: {e}")))?;
        Ok(tree_id.detach())
    }

    pub fn push(&self, remote: &str, _branch: &str) -> PkmResult<()> {
        if self.get_remote_url(remote).is_none() {
            return Ok(());
        }
        Err(PkmError::Git("push not yet implemented via gix".into()))
    }

    pub fn pull(&self, remote: &str, _branch: &str) -> PkmResult<PullResult> {
        if self.get_remote_url(remote).is_none() {
            return Ok(PullResult {
                success: true,
                conflicts: vec![],
            });
        }
        Err(PkmError::Git("pull not yet implemented via gix".into()))
    }

    pub fn status(&self) -> PkmResult<Vec<(String, StatusFlags)>> {
        let mut out = Vec::new();
        let platform = self
            .repo
            .status(gix::progress::Discard)
            .map_err(|e| PkmError::Git(format!("status init: {e}")))?;

        let iter = platform
            .into_index_worktree_iter(Vec::<gix::bstr::BString>::new())
            .map_err(|e| PkmError::Git(format!("status iter: {e}")))?;

        for item_res in iter {
            let item = item_res.map_err(|e| PkmError::Git(format!("status item: {e}")))?;
            let path = item.rela_path().to_string();
            let mut flags = StatusFlags::CURRENT;

            use gix::status::index_worktree::Item;
            match &item {
                Item::Modification { status, .. } => {
                    use gix::status::plumbing::index_as_worktree::EntryStatus;
                    match status {
                        EntryStatus::Conflict { .. } => flags = flags | StatusFlags::CONFLICTED,
                        EntryStatus::Change(change) => {
                            use gix::status::plumbing::index_as_worktree::Change;
                            match change {
                                Change::Modification { .. } | Change::SubmoduleModification(_) => {
                                    flags = flags | StatusFlags::WT_MODIFIED
                                }
                                Change::Removed => flags = flags | StatusFlags::WT_DELETED,
                                Change::Type { .. } => flags = flags | StatusFlags::WT_TYPECHANGE,
                            }
                        }
                        _ => {}
                    }
                }
                Item::DirectoryContents { entry, .. } => {
                    if entry.status == gix::dir::entry::Status::Untracked {
                        flags = flags | StatusFlags::WT_NEW;
                    }
                }
                Item::Rewrite { .. } => {
                    flags = flags | StatusFlags::WT_RENAMED;
                }
            }

            out.push((path, flags));
        }
        Ok(out)
    }

    pub fn log(&self, max_count: usize) -> PkmResult<Vec<CommitInfo>> {
        let head_commit = match self
            .repo
            .head()
            .ok()
            .and_then(|mut h| h.peel_to_commit().ok())
        {
            Some(c) => c,
            None => return Ok(Vec::new()),
        };
        let head_oid = head_commit.id().detach();

        let revwalk = self.repo.rev_walk([head_oid]);

        let mut commits = Vec::new();
        let mut count = 0;
        let _ = revwalk.selected(|id| {
            if count >= max_count {
                return false;
            }
            count += 1;

            let oid = gix::ObjectId::from(id);
            if let Ok(commit_obj) = self.repo.find_object(oid) {
                let commit = commit_obj.into_commit();
                let ts_secs = commit.committer().map(|s| s.seconds()).unwrap_or_default();
                let timestamp = DateTime::from_timestamp(ts_secs, 0).unwrap_or_default();

                commits.push(CommitInfo {
                    hash: oid.to_string(),
                    author: commit
                        .author()
                        .map(|s| s.name.as_bstr().to_string())
                        .unwrap_or_default(),
                    message: commit
                        .message()
                        .map(|m| m.title.to_string())
                        .unwrap_or_else(|_| String::new()),
                    timestamp,
                });
            }
            true
        });
        Ok(commits)
    }

    pub fn diff(&self, path: &str) -> PkmResult<String> {
        let mut output = String::new();
        let head_tree_id = self
            .repo
            .head()
            .ok()
            .and_then(|mut h| h.peel_to_commit().ok())
            .and_then(|c| c.tree_id().ok());

        if let Some(tree_id) = head_tree_id {
            if let Ok(old_tree_obj) = self.repo.find_object(tree_id) {
                let old_tree = old_tree_obj.into_tree();
                let workdir = self
                    .repo
                    .workdir()
                    .ok_or_else(|| PkmError::Git("no workdir".into()))?;
                let full_path = workdir.join(path);

                let old_content = find_blob_in_tree(&old_tree, path, &self.repo);
                let new_content = std::fs::read(&full_path).ok();

                let old_lines: Vec<&str> =
                    old_content.as_deref().unwrap_or("").split('\n').collect();
                let new_text = new_content
                    .as_ref()
                    .map(|c| std::str::from_utf8(c).unwrap_or(""))
                    .unwrap_or("");
                let new_lines: Vec<&str> = new_text.split('\n').collect();

                output.push_str(&format!("--- a/{path}\n+++ b/{path}\n"));
                for i in 0..old_lines.len().max(new_lines.len()) {
                    let o = old_lines.get(i).copied().unwrap_or("");
                    let n = new_lines.get(i).copied().unwrap_or("");
                    if o != n {
                        if i < old_lines.len() {
                            output.push_str(&format!("-{o}\n"));
                        }
                        if i < new_lines.len() {
                            output.push_str(&format!("+{n}\n"));
                        }
                    } else {
                        output.push_str(&format!(" {o}\n"));
                    }
                }
            }
        }
        Ok(output)
    }

    pub fn get_remote_url(&self, remote: &str) -> Option<String> {
        // First try the resolved config (cached)
        if let Ok(r) = self.repo.find_remote(remote) {
            if let Some(url) = r.url(gix::remote::Direction::Fetch) {
                return Some(url.to_string());
            }
        }
        // Fallback: read directly from config file (covers freshly written config)
        let config_path = self.repo.git_dir().join("config");
        let content = std::fs::read_to_string(&config_path).ok()?;
        let pattern = format!("[remote \"{remote}\"]");
        let pattern2 = format!("[remote '{remote}']");
        let section_start = content.find(&pattern).or_else(|| content.find(&pattern2))?;
        let after_section = &content[section_start..];
        for line in after_section.lines() {
            if line.trim_start().starts_with("url =") {
                let url = line.trim_start().trim_start_matches("url =").trim();
                if !url.is_empty() {
                    return Some(url.to_string());
                }
            }
            if line.starts_with('[') && !line.starts_with(&pattern) && !line.starts_with(&pattern2)
            {
                break;
            }
        }
        None
    }

    pub fn set_remote(&self, name: &str, url: &str) -> PkmResult<()> {
        let config_path = self.repo.git_dir().join("config");
        let content = std::fs::read_to_string(&config_path).unwrap_or_default();
        let mut new_content = String::new();

        // Strip any existing section for this remote name
        let mut skip = false;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') {
                if trimmed.starts_with(&format!("[remote \"{name}\"]"))
                    || trimmed.starts_with(&format!("[remote '{name}']"))
                    || trimmed == format!("[remote \"{name}\"]")
                    || trimmed == format!("[remote '{name}']")
                {
                    skip = true;
                    continue;
                }
                skip = false;
            }
            if !skip {
                new_content.push_str(line);
                new_content.push('\n');
            }
        }

        // Append the new remote section
        new_content.push_str(&format!(
            "[remote \"{name}\"]\n\turl = {url}\n\tfetch = +refs/heads/*:refs/remotes/{name}/*\n"
        ));

        std::fs::write(&config_path, new_content)
            .map_err(|e| PkmError::Git(format!("write config: {e}")))?;
        Ok(())
    }

    pub fn repository(&self) -> &gix::Repository {
        &self.repo
    }

    pub fn set_ssh_key_path(&mut self, path: Option<PathBuf>) {
        self.ssh_key_path = path;
    }

    pub fn ssh_key_path(&self) -> Option<&PathBuf> {
        self.ssh_key_path.as_ref()
    }

    pub fn set_passphrase(&mut self, passphrase: Option<String>) {
        self.passphrase = passphrase;
    }

    pub fn passphrase(&self) -> Option<&str> {
        self.passphrase.as_deref()
    }

    #[allow(clippy::result_large_err)]
    pub fn credentials_callback(
        &self,
    ) -> impl FnMut(gix::credentials::helper::Action) -> gix::credentials::protocol::Result
           + Clone
           + 'static {
        let _key_path = self.ssh_key_path.clone();
        let _passphrase = self.passphrase.clone();
        move |action: gix::credentials::helper::Action| gix::credentials::builtin(action)
    }

    pub fn get_current_branch(&self) -> Option<String> {
        self.repo
            .head()
            .ok()
            .and_then(|h| h.try_into_referent())
            .map(|r| r.name().as_bstr().to_string())
            .map(|name| {
                name.strip_prefix("refs/heads/")
                    .unwrap_or(&name)
                    .to_string()
            })
    }

    pub fn ahead_behind(&self, remote_branch: &str) -> PkmResult<(usize, usize)> {
        let head_commit = self
            .repo
            .head()
            .ok()
            .and_then(|mut h| h.peel_to_commit().ok())
            .ok_or_else(|| PkmError::Git("no local HEAD commit".into()))?;
        let local_oid = head_commit.id().detach();

        // Try to find tracking branch (e.g., refs/remotes/origin/main)
        let tracking_names = [
            format!("refs/remotes/{remote_branch}"),
            format!("refs/remotes/origin/{remote_branch}"),
            format!("refs/heads/{remote_branch}"),
        ];
        let mut upstream_id = None;
        for tn in &tracking_names {
            if let Ok(mut r) = self.repo.find_reference(tn) {
                if let Ok(pid) = r.peel_to_id() {
                    upstream_id = Some(pid.detach());
                    break;
                }
            }
        }
        let upstream_oid = upstream_id.ok_or_else(|| {
            PkmError::Git(format!("no tracking branch found for '{remote_branch}'"))
        })?;

        // Walk commits reachable from local but not from upstream
        let ahead = walk_count(&self.repo, local_oid, |id| id == upstream_oid);
        let behind = walk_count(&self.repo, upstream_oid, |id| id == local_oid);

        Ok((ahead, behind))
    }
}

fn walk_count(
    repo: &gix::Repository,
    tip: gix::ObjectId,
    stop: impl Fn(&gix::hash::oid) -> bool,
) -> usize {
    let revwalk = repo.rev_walk([tip]);
    let mut count = 0;
    let _ = revwalk.selected(|id| {
        if stop(id) {
            return false;
        }
        count += 1;
        true
    });
    count
}

fn find_blob_in_tree(tree: &gix::Tree, path: &str, repo: &gix::Repository) -> Option<String> {
    let path = path.trim_start_matches('/');
    for entry_id in tree.iter() {
        let entry = entry_id.expect("valid entry");
        let name = entry.filename().to_string();
        if name == path {
            if let Ok(obj) = repo.find_object(entry.oid()) {
                let blob = obj.into_blob();
                return Some(blob.data.clone().into_string_lossy());
            }
        } else if path.starts_with(&name) && path.as_bytes().get(name.len()) == Some(&b'/') {
            if let Ok(obj) = repo.find_object(entry.oid()) {
                let subtree = obj.into_tree();
                let sub_path = &path[name.len() + 1..];
                return find_blob_in_tree(&subtree, sub_path, repo);
            }
        }
    }
    None
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
}
