//! Internal helper functions for the git engine.
//!
//! Not part of the public API; used by the sibling modules above.

use gix::bstr::ByteVec;

/// Count commits reachable from `tip`, stopping as soon as `stop` matches.
pub(crate) fn walk_count(
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

/// Recursively look up the blob stored at `path` in `tree`.
pub(crate) fn find_blob_in_tree(
    tree: &gix::Tree,
    path: &str,
    repo: &gix::Repository,
) -> Option<String> {
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
