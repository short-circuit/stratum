//! Read-only inspection operations for the git engine.
//!
//! Status, log, diff and remote/branch queries. These only ever read
//! from the repository or the working tree.

use super::helpers::{find_blob_in_tree, walk_count};
use super::CommitInfo;
use crate::git::StatusFlags;
use chrono::DateTime;
use gix::bstr::ByteSlice;
use pkm_core::{PkmError, PkmResult};

impl super::GitEngine {
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
