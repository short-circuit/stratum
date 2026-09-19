use chrono::{DateTime, Utc};
use gix::bstr::{ByteSlice, ByteVec};
use pkm_core::{PkmError, PkmResult};
use std::ops::BitOr;
#[cfg(not(windows))]
use std::os::unix::fs::PermissionsExt;
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

        // Expand any directory paths into the full recursive set of files,
        // matching `git add <dir>` (honours .gitignore, recurses into
        // subdirectories, skips binary/ignored content). Directory expansion
        // delegates to the system git so we do not reimplement ignore rules;
        // single-file staging stays in-process via gix below.
        let file_paths = paths
            .iter()
            .filter_map(|p| {
                let full = workdir.join(p);
                if full.is_dir() {
                    None
                } else {
                    Some(*p)
                }
            })
            .collect::<Vec<_>>();

        let dirs = paths
            .iter()
            .filter(|p| workdir.join(p).is_dir())
            .copied()
            .collect::<Vec<_>>();
        if !dirs.is_empty() {
            self.add_directory_paths(&dirs)?;
        }

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

        // Remove any existing entries for the staged paths (all stages). This
        // is what makes `add()` usable for conflict resolution: after
        // `git merge` writes stage-1/2/3 conflict entries, re-adding the file
        // must replace them with a single resolved stage-0 entry — otherwise
        // the tree built by `commit()` still contains stale conflict stages
        // and `status()` keeps reporting the path as conflicted even after the
        // "resolution" commit.
        let to_remove = file_paths.to_vec();
        state.remove_entries(|_, path, _| to_remove.contains(&path.to_string().as_str()));

        for p in &file_paths {
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

    /// Stage every file under the given directory paths, honouring gitignore,
    /// by delegating to the system `git add <dir>` (the same semantics the
    /// manual "sync all" and conflict-resolution flows depend on). Directory
    /// staging needs ignore rules and recursive walks that gix's index API
    /// does not expose here, so the CLI is the correct tool — and the engine
    /// already shells out to `git` for push/pull/merge.
    fn add_directory_paths(&self, dirs: &[&str]) -> PkmResult<()> {
        let workdir = self
            .repo
            .workdir()
            .ok_or_else(|| PkmError::Git("no working directory".into()))?
            .to_path_buf();
        let mut cmd = std::process::Command::new("git");
        cmd.current_dir(&workdir)
            .arg("add")
            .arg("--")
            .args(dirs)
            .stderr(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped());
        let output = cmd
            .output()
            .map_err(|e| PkmError::Git(format!("git add execution failed: {e}")))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(PkmError::Git(format!(
                "git add {} failed: {}",
                dirs.join(", "),
                stderr.trim()
            )));
        }
        Ok(())
    }

    pub fn commit(&self, message: &str, author: &str) -> PkmResult<String> {
        let tree_id = self
            .write_tree_from_index()
            .map_err(|e| PkmError::Git(format!("build tree: {e}")))?;

        let mut parents: Vec<gix::ObjectId> = self
            .repo
            .head()
            .ok()
            .and_then(|mut h| h.peel_to_commit().ok())
            .map(|c| c.id().detach())
            .into_iter()
            .collect();

        // If a merge is in progress (`.git/MERGE_HEAD`), complete it: the
        // commit must take the in-progress merge head as an additional parent,
        // exactly like `git commit` does. Without this, committing a resolved
        // conflict produced a *linear* commit that did not descend from the
        // upstream branch, so the follow-up `push` was rejected as
        // non-fast-forward.
        let merge_head_path = self.repo.git_dir().join("MERGE_HEAD");
        if let Ok(raw) = std::fs::read_to_string(&merge_head_path) {
            let first_line = raw.lines().next().map(|s| s.trim().to_string());
            if let Some(oid) = first_line
                .filter(|s| !s.is_empty())
                .and_then(|s| gix::ObjectId::from_hex(s.as_bytes()).ok())
            {
                if !parents.contains(&oid) {
                    parents.push(oid);
                }
                // The merge is being completed by this commit.
                let _ = std::fs::remove_file(&merge_head_path);
            }
        }

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

        // Collect every staged path as (full path, tree entry mode, oid).
        // The paths are stored flat in the index (`a/b/c` as one entry), so we
        // must split them into components and rebuild a properly NESTED tree
        // object graph. Writing a flat `Tree { entries: [".pkm/blocks.db"] }`
        // produces a tree with a literal '/' in an entry name, which git
        // itself treats as a corrupt tree and `gix`'s `from_tree()` (used by
        // `status()` full iteration) rejects with `PathSeparator`.
        use std::collections::BTreeMap;
        struct Node {
            // Child name -> (mode, oid) for leaf entries (files/symlinks).
            leaves: Vec<(Vec<u8>, gix::objs::tree::EntryMode, gix::ObjectId)>,
            // Child name -> nested directory contents.
            dirs: BTreeMap<Vec<u8>, Node>,
        }
        impl Node {
            fn new() -> Self {
                Node {
                    leaves: Vec::new(),
                    dirs: BTreeMap::new(),
                }
            }

            /// Insert `(rel_path, mode, oid)` splitting on '/'.
            fn insert(
                &mut self,
                rel_path: &[u8],
                mode: gix::objs::tree::EntryMode,
                oid: gix::ObjectId,
            ) {
                let mut parts = rel_path.split(|b| *b == b'/');
                let first = parts.next().expect("non-empty path");
                let rest: Vec<&[u8]> = parts.collect();
                if rest.is_empty() {
                    self.leaves.push((first.to_vec(), mode, oid));
                } else {
                    self.dirs
                        .entry(first.to_vec())
                        .or_insert_with(Node::new)
                        .insert_with_components(&rest, mode, oid);
                }
            }

            fn insert_with_components(
                &mut self,
                parts: &[&[u8]],
                mode: gix::objs::tree::EntryMode,
                oid: gix::ObjectId,
            ) {
                if parts.len() == 1 {
                    self.leaves.push((parts[0].to_vec(), mode, oid));
                } else {
                    self.dirs
                        .entry(parts[0].to_vec())
                        .or_insert_with(Node::new)
                        .insert_with_components(&parts[1..], mode, oid);
                }
            }
        }

        let mut root = Node::new();
        for entry in state.entries() {
            let mode_val: u32 = entry.mode.bits();
            let file_mode = 0o100644u32;
            let mode = gix::objs::tree::EntryMode::try_from(mode_val)
                .unwrap_or_else(|_| gix::objs::tree::EntryMode::try_from(file_mode).unwrap());
            root.insert(entry.path_in(backing).as_bytes(), mode, entry.id);
        }

        /// Recursively materialise `node` into a tree object and return its id.
        fn write_node(node: &Node, repo: &gix::Repository) -> PkmResult<gix::ObjectId> {
            let mut entries: Vec<gix::objs::tree::Entry> = Vec::new();

            // Directories first (git tree ordering: subtrees sort before
            // blobs within the same name scope, and are compared by name).
            for (name, child) in &node.dirs {
                let child_id = write_node(child, repo)?;
                entries.push(gix::objs::tree::Entry {
                    mode: gix::objs::tree::EntryMode::try_from(0o40000u32)
                        .expect("dir mode is valid"),
                    filename: name.clone().into(),
                    oid: child_id,
                });
            }

            for (name, mode, oid) in &node.leaves {
                entries.push(gix::objs::tree::Entry {
                    mode: *mode,
                    filename: name.clone().into(),
                    oid: *oid,
                });
            }

            // Git requires tree entries to be serialized sorted by filename
            // (byte order). The `gix-object` writer asserts exactly this; no
            // directories-first rule applies (that is only an artifact of how
            // git compares full paths, not a flat name sort).
            entries.sort_by(|a, b| a.filename.cmp(&b.filename));

            let tree = gix::objs::Tree { entries };
            let tree_id = repo
                .write_object(&tree)
                .map_err(|e| PkmError::Git(format!("write tree: {e}")))?;
            Ok(tree_id.detach())
        }

        write_node(&root, &self.repo)
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
        // Step 2: Fast-forward merge when possible; otherwise fall back to a
        // real merge so conflicts are surfaced (not just aborted).
        //
        // `git merge --ff-only` aborts (rc≠0, "Not possible to fast-forward,
        // aborting") as soon as the histories diverge — BEFORE any conflict
        // markers or MERGE_HEAD state exist. The old code returned
        // `Err(merge failed: ...)` in that case, which made the
        // PullResult{success:false, conflicts} path dead code and broke the
        // documented conflict workflow (status never showed a conflicted file,
        // `resolve_conflict_file`/`abort_merge` had no merge state to act on).
        //
        // Fix: when the FF-only merge refuses because the branches diverged,
        // perform a real `git merge` of the fetched branch. A clean merge
        // commits automatically (still reported as success); a textual
        // conflict leaves conflict markers + MERGE_HEAD in the worktree and is
        // reported as `success:false` with the conflicted paths, exactly what
        // the command layer and conflict-resolution UI consume.
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
            // Divergence -> do a real merge so conflicts become real state.
            let divergence = stderr.contains("fast-forward");
            if divergence {
                let mut real_merge = std::process::Command::new("git");
                real_merge
                    .current_dir(workdir)
                    .arg("merge")
                    .arg("--no-edit")
                    .arg(format!("{}/{}", remote, branch))
                    .stderr(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::piped());
                let real_output = real_merge
                    .output()
                    .map_err(|e| PkmError::Git(format!("real merge execution failed: {e}")))?;
                if real_output.status.success() {
                    // Clean auto-merge: committed the join; still a successful pull.
                    return Ok(PullResult {
                        success: true,
                        conflicts: vec![],
                    });
                }
                // Real conflicts: list them from the worktree state. Reading the
                // index via the platform status API immediately after the merge
                // can race on loaded CI runners (in one CI run `status()`
                // observed the index before the merge's conflict stages were
                // visible, yielding an empty list despite a genuine conflict).
                // Enumerate unmerged paths with `git diff --diff-filter=U`,
                // which reflects the on-disk index and is stable under load.
                let workdir = self
                    .repo
                    .workdir()
                    .ok_or_else(|| PkmError::Git("no workdir".into()))?;
                let diff_out = std::process::Command::new("git")
                    .current_dir(workdir)
                    .args(["diff", "--name-only", "--diff-filter=U"])
                    .stderr(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::piped())
                    .output()
                    .map_err(|e| PkmError::Git(format!("conflict enumerate failed: {e}")))?;
                let conflict_files = if diff_out.status.success() {
                    String::from_utf8_lossy(&diff_out.stdout)
                        .lines()
                        .map(str::to_string)
                        .filter(|s| !s.is_empty())
                        .collect::<Vec<_>>()
                } else {
                    // Fall back to the platform status-derived list.
                    self.status()
                        .ok()
                        .map(|s| {
                            s.into_iter()
                                .filter(|(_, flags)| flags.is_conflicted())
                                .map(|(path, _)| path)
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default()
                };
                return Ok(PullResult {
                    success: false,
                    conflicts: conflict_files,
                });
            }
            // Not a divergence: surface the actual error.
            return Err(PkmError::Git(format!("merge failed: {}", stderr.trim())));
        }
        Ok(PullResult {
            success: true,
            conflicts: vec![],
        })
    }

    /// Inject SSH configuration into a git command.
    ///
    /// When an SSH key is configured, sets `GIT_SSH_COMMAND` to use that key
    /// with strict host key checking.
    ///
    /// When a passphrase is also configured, sets `SSH_ASKPASS` (plus
    /// `SSH_ASKPASS_REQUIRE=force`) pointing to the helper script (written by
    /// `set_passphrase`) so the passphrase is never embedded in an environment
    /// variable or command line.
    ///
    /// Note: OpenSSH's `ssh(1)` reads `SSH_ASKPASS`, not `GIT_ASKPASS`
    /// (`GIT_ASKPASS` is git's variable for the HTTP transport only). Failing
    /// to set the SSH_* variables silently broke every push/pull that used a
    /// passphrase-protected key (the key could be loaded but never decrypted),
    /// so this is a correctness fix, not a refactor.
    fn inject_ssh_key(&self, cmd: &mut std::process::Command) {
        if let Some(ref key_path) = self.ssh_key_path {
            let ssh_base = format!(
                "ssh -i {} -o IdentitiesOnly=yes -o StrictHostKeyChecking=accept-new",
                key_path.display()
            );

            if self.passphrase.is_some() {
                if let Some(ref script_path) = self.askpass_script_path {
                    cmd.env("SSH_ASKPASS", script_path);
                    // Without this, ssh ignores SSH_ASKPASS when there is no
                    // controlling terminal / DISPLAY. `force` makes it always
                    // call the askpass program (OpenSSH >= 8.4).
                    cmd.env("SSH_ASKPASS_REQUIRE", "force");
                }
            }

            cmd.env("GIT_SSH_COMMAND", &ssh_base);
        }
    }

    /// Write the SSH_ASKPASS helper script that provides the SSH key passphrase.
    ///
    /// The script uses `printf` with octal-encoded bytes so there are no shell
    /// escaping concerns regardless of passphrase content. The file is written
    /// with 0700 permissions and removed in `Drop`.
    fn write_askpass_script(&mut self) -> PkmResult<PathBuf> {
        let passphrase = self
            .passphrase
            .clone()
            .ok_or_else(|| PkmError::Git("no passphrase set for askpass script".into()))?;

        // Clean up any previous script first
        self.cleanup_askpass_script();

        let path = std::env::temp_dir().join(format!("pkm-askpass-{}", std::process::id()));

        // Encode the passphrase as octal escapes so no shell characters can
        // interfere — printf(1) interprets \NNN in the format string.
        let octal: String = passphrase
            .bytes()
            .map(|b| format!("\\{:03o}", b))
            .collect::<Vec<_>>()
            .join("");
        let script = format!("#!/bin/sh\nprintf '{}'\n", octal);

        std::fs::write(&path, &script)
            .map_err(|e| PkmError::Git(format!("write askpass script: {e}")))?;
        #[cfg(not(windows))]
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))
            .map_err(|e| PkmError::Git(format!("chmod askpass script: {e}")))?;

        tracing::debug!("wrote SSH_ASKPASS script to {}", path.display());

        self.askpass_script_path = Some(path.clone());
        Ok(path)
    }

    /// Remove the askpass script from disk.
    fn cleanup_askpass_script(&mut self) {
        if let Some(ref path) = self.askpass_script_path.take() {
            let _ = std::fs::remove_file(path);
            tracing::debug!("cleaned up SSH_ASKPASS script {}", path.display());
        }
    }

    pub fn status(&self) -> PkmResult<Vec<(String, StatusFlags)>> {
        let mut out = Vec::new();
        let platform = self
            .repo
            .status(gix::progress::Discard)
            .map_err(|e| PkmError::Git(format!("status init: {e}")))?;

        // Full status: changes between HEAD and the index (staged) plus changes
        // between the index and the working tree (unstaged/untracked). This must
        // ALSO report staged changes — the command layer (`sync_vault`) runs
        // `add(".")` BEFORE `status()` to classify files, so an index-vs-HEAD
        // comparison is required for INDEX_* flags to ever be produced.
        let iter = platform
            .into_iter(Vec::<gix::bstr::BString>::new())
            .map_err(|e| PkmError::Git(format!("status iter: {e}")))?;

        for item_res in iter {
            let item =
                item_res.map_err(|e| PkmError::Git(format!("status item (full): {e:#?}")))?;
            let path = item.location().to_string();
            let mut flags = StatusFlags::CURRENT;

            use gix::status::Item;
            match &item {
                // Changes between HEAD and the index (staged).
                Item::TreeIndex(change) => {
                    use gix::diff::index::ChangeRef;
                    match change {
                        ChangeRef::Addition { entry_mode, .. } => {
                            flags = flags | StatusFlags::INDEX_NEW;
                            if entry_mode.is_submodule() {
                                flags = flags | StatusFlags::INDEX_TYPECHANGE
                            }
                        }
                        ChangeRef::Deletion { .. } => flags = flags | StatusFlags::INDEX_DELETED,
                        ChangeRef::Modification { entry_mode, .. } => {
                            flags = flags | StatusFlags::INDEX_MODIFIED;
                            if entry_mode.is_submodule() {
                                flags = flags | StatusFlags::INDEX_TYPECHANGE
                            }
                        }
                        ChangeRef::Rewrite { .. } => flags = flags | StatusFlags::INDEX_RENAMED,
                    }
                }
                // Changes between the index and the working tree (unstaged).
                Item::IndexWorktree(change) => {
                    use gix::status::index_worktree::Item;
                    match change {
                        Item::Modification { status, .. } => {
                            use gix::status::plumbing::index_as_worktree::EntryStatus;
                            match status {
                                EntryStatus::Conflict { .. } => {
                                    flags = flags | StatusFlags::CONFLICTED
                                }
                                EntryStatus::Change(change) => {
                                    use gix::status::plumbing::index_as_worktree::Change;
                                    match change {
                                        Change::Modification { .. }
                                        | Change::SubmoduleModification(_) => {
                                            flags = flags | StatusFlags::WT_MODIFIED
                                        }
                                        Change::Removed => flags = flags | StatusFlags::WT_DELETED,
                                        Change::Type { .. } => {
                                            flags = flags | StatusFlags::WT_TYPECHANGE
                                        }
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
        // If we have both a key and passphrase, ensure the askpass script exists
        if self.passphrase.is_some() && self.ssh_key_path.is_some() {
            if self.askpass_script_path.is_none() {
                let _ = self.write_askpass_script();
            }
        } else {
            self.cleanup_askpass_script();
        }
    }

    pub fn ssh_key_path(&self) -> Option<&PathBuf> {
        self.ssh_key_path.as_ref()
    }

    pub fn set_passphrase(&mut self, passphrase: Option<String>) {
        self.passphrase = passphrase;
        // Recreate the askpass script with the new passphrase
        self.cleanup_askpass_script();
        if self.passphrase.is_some() && self.ssh_key_path.is_some() {
            let _ = self.write_askpass_script();
        }
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

impl Drop for GitEngine {
    fn drop(&mut self) {
        self.cleanup_askpass_script();
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
