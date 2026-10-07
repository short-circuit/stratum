//! Git sync engine for vault synchronization.
//!
//! The [`GitEngine`] struct and its core lifecycle operations live here.
//! All methods for this type are defined across sibling modules:
//!
//! * `auth` — SSH key and passphrase credential handling.
//! * `inspect` — read-only inspection (status, log, diff, remotes, branch).
//! * `helpers` — internal walk and tree-lookup utility functions.

mod auth;
mod helpers;
mod inspect;

use chrono::{DateTime, Utc};
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
    askpass_script_path: Option<PathBuf>,
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
            askpass_script_path: None,
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
            askpass_script_path: None,
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

    pub fn push(&self, remote: &str, branch: &str) -> PkmResult<()> {
        if self.get_remote_url(remote).is_none() {
            return Ok(());
        }
        let workdir = self
            .repo
            .workdir()
            .ok_or_else(|| PkmError::Git("no workdir".into()))?;
        let mut cmd = std::process::Command::new("git");
        cmd.current_dir(workdir)
            .arg("push")
            .arg(remote)
            .arg(format!("refs/heads/{branch}:refs/heads/{branch}"))
            .stderr(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped());
        self.inject_ssh_key(&mut cmd);
        let output = cmd
            .output()
            .map_err(|e| PkmError::Git(format!("push execution failed: {e}")))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(PkmError::Git(format!("push failed: {}", stderr.trim())));
        }
        Ok(())
    }

    pub fn pull(&self, remote: &str, branch: &str) -> PkmResult<PullResult> {
        if self.get_remote_url(remote).is_none() {
            return Ok(PullResult {
                success: true,
                conflicts: vec![],
            });
        }
        let workdir = self
            .repo
            .workdir()
            .ok_or_else(|| PkmError::Git("no workdir".into()))?;
        // Step 1: Fetch from remote
        let mut fetch_cmd = std::process::Command::new("git");
        fetch_cmd
            .current_dir(workdir)
            .arg("fetch")
            .arg(remote)
            .arg(branch)
            .stderr(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped());
        self.inject_ssh_key(&mut fetch_cmd);
        let fetch_output = fetch_cmd
            .output()
            .map_err(|e| PkmError::Git(format!("fetch execution failed: {e}")))?;
        if !fetch_output.status.success() {
            let stderr = String::from_utf8_lossy(&fetch_output.stderr);
            return Err(PkmError::Git(format!("fetch failed: {}", stderr.trim())));
        }
        // Step 2: Try fast-forward merge (rebase-like)
        let mut merge_cmd = std::process::Command::new("git");
        merge_cmd
            .current_dir(workdir)
            .arg("merge")
            .arg("--ff-only")
            .arg(format!("{}/{}", remote, branch))
            .stderr(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped());
        let merge_output = merge_cmd
            .output()
            .map_err(|e| PkmError::Git(format!("merge execution failed: {e}")))?;
        if !merge_output.status.success() {
            let stderr = String::from_utf8_lossy(&merge_output.stderr);
            // Check if there are conflicts
            if stderr.contains("conflict") || stderr.contains("would be overwritten") {
                // Run merge with no-commit to detect conflicts
                let status = self.status().ok();
                let conflict_files = status
                    .map(|s| {
                        s.into_iter()
                            .filter(|(_, flags)| flags.is_conflicted())
                            .map(|(path, _)| path)
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                return Ok(PullResult {
                    success: false,
                    conflicts: conflict_files,
                });
            }
            return Err(PkmError::Git(format!("merge failed: {}", stderr.trim())));
        }
        Ok(PullResult {
            success: true,
            conflicts: vec![],
        })
    }

    pub fn repository(&self) -> &gix::Repository {
        &self.repo
    }
}

impl Drop for GitEngine {
    fn drop(&mut self) {
        self.cleanup_askpass_script();
    }
}

#[cfg(test)]
mod tests;
