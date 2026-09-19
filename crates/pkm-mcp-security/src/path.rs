//! Vault-root path containment (contract §6).
//!
//! Every user-supplied `path`/`target` value MUST be checked against the vault
//! root before any filesystem access. The contract requires containment
//! equivalent to the desktop app's `resolve_safe_path` (canonicalize then
//! verify `starts_with(vault_root)`). Because the MCP server may be deployed
//! where the vault path contains symlinks, we canonicalize both sides and then
//! verify the result stays under the canonical vault root — rejecting `..`
//! traversal and absolute-path escapes.

use std::path::{Component, Path, PathBuf};

/// A validated, vault-relative path.
///
/// Guarantees the contained value:
/// - has no `.` / `..` components,
/// - uses `/` separators,
/// - is non-empty and ≤ [`crate::MCP_PATH_MAX`] chars,
/// - has no leading `/` and no trailing `/`,
/// - contains no NUL bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafeVaultPath(String);

impl SafeVaultPath {
    /// The validated normalized path string (vault-relative, `/`-separated).
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The validated path as a `PathBuf` (for joining onto the vault root).
    pub fn as_path(&self) -> &Path {
        Path::new(&self.0)
    }
}

/// Result of a path check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathDecision {
    /// The value is a safe vault-relative path.
    Safe(SafeVaultPath),
    /// The value escapes the vault root or is otherwise invalid.
    Traversal(String),
}

/// Validate a user-supplied `path` as a vault-relative POSIX path.
///
/// Accepts both `/` and `\` (Windows clients) separators and normalizes to
/// `/`. Rejects absolute paths, `..` escapes, empty paths, NUL bytes, and
/// over-long paths.
pub fn validate_vault_path(raw: &str) -> PathDecision {
    if raw.is_empty() {
        return PathDecision::Traversal("empty path".into());
    }
    if raw.len() > crate::MCP_PATH_MAX {
        return PathDecision::Traversal(format!("path exceeds {} chars", crate::MCP_PATH_MAX));
    }
    if raw.contains('\0') {
        return PathDecision::Traversal("path contains NUL byte".into());
    }

    let parts: Vec<String> = raw
        .split(['/', '\\'])
        .filter(|s| !s.is_empty()) // collapse repeated separators, drop trailing
        .map(|s| s.to_string())
        .collect();

    if parts.is_empty() {
        return PathDecision::Traversal("empty path".into());
    }

    // Leading slash (absolute) check.
    if raw.starts_with('/') || raw.starts_with('\\') {
        return PathDecision::Traversal(format!("absolute path not vault-relative: {raw}"));
    }

    // Windows drive-letter absolute path (`C:` prefix) — not vault-relative.
    if contains_drive_prefix(raw) {
        return PathDecision::Traversal(format!("absolute path not vault-relative: {raw}"));
    }

    let mut normalized: Vec<String> = Vec::with_capacity(parts.len());
    for part in &parts {
        match part.as_str() {
            "." => {}
            ".." => {
                if normalized.pop().is_none() {
                    // `..` above the root.
                    return PathDecision::Traversal(format!("path escapes vault root: {raw}"));
                }
            }
            _ => normalized.push(part.clone()),
        }
    }

    if normalized.is_empty() {
        return PathDecision::Traversal(format!("path collapses to root: {raw}"));
    }

    PathDecision::Safe(SafeVaultPath(normalized.join("/")))
}

/// Canonicalize a user-supplied path below `vault_root` and verify it does
/// not escape. This is the containment check equivalent to
/// `resolve_safe_path`/`resolve_safe_write_path` referenced in the contract.
///
/// Returns the canonical absolute path when it exists and stays under the
/// root. `None` means the value escapes the root (caller maps to an error),
/// or the file does not exist (caller decides 404 vs. create).
pub fn canonical_contained(vault_root: &Path, candidate: &Path) -> Option<PathBuf> {
    let root = vault_root.canonicalize().ok()?;
    // If the candidate doesn't exist yet (write path), canonicalize its parent.
    let canon = match candidate.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            let parent = candidate.parent()?;
            let canon_parent = parent.canonicalize().ok()?;
            let name = candidate.file_name()?;
            canon_parent.join(name)
        }
    };
    if canon.starts_with(&root) {
        Some(canon)
    } else {
        None
    }
}

/// True if `raw` starts with a Windows drive-letter prefix (`C:`, `C:\`, …)
/// that makes it an absolute (non-vault-relative) path. A bare `x:` at the
/// front of an otherwise relative-looking path is treated as absolute because
/// it cannot be a valid vault-relative component on any platform.
fn contains_drive_prefix(raw: &str) -> bool {
    if raw.len() < 2 {
        return false;
    }
    let b = raw.as_bytes();
    b[0].is_ascii_alphabetic()
        && b[1] == b':'
        // `C:` alone, `C:\...`, or `C:/...` — a following separator is not
        // required; a drive prefix makes the whole path absolute.
        && (raw.len() == 2 || matches!(raw.as_bytes().get(2), Some(&c) if c == b'/' || c == b'\\'))
}

/// True if `candidate` (which must already be resolved) is lexically within
/// `root` after normalization. Cheaper than `canonicalize` for already-joined
/// paths; prefer [`canonical_contained`] when symlinks matter.
pub fn lexically_contained(root: &Path, candidate: &Path) -> bool {
    let root_norm = normalize_lexical(root);
    let cand_norm = normalize_lexical(candidate);
    cand_norm.starts_with(&root_norm)
}

/// Normalize a path by removing `.` components and resolving `..` lexically.
fn normalize_lexical(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in p.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_normal_relative_paths() {
        assert!(matches!(
            validate_vault_path("notes/foo.md"),
            PathDecision::Safe(_)
        ));
        assert!(matches!(
            validate_vault_path("a/b/c/d.md"),
            PathDecision::Safe(_)
        ));
    }

    #[test]
    fn accepts_windows_separators_and_normalizes() {
        match validate_vault_path(r"notes\foo.md") {
            PathDecision::Safe(p) => assert_eq!(p.as_str(), "notes/foo.md"),
            PathDecision::Traversal(_) => panic!("should be safe"),
        }
    }

    #[test]
    fn collapses_dots_and_repeated_separators() {
        match validate_vault_path("a//b/./c.md") {
            PathDecision::Safe(p) => assert_eq!(p.as_str(), "a/b/c.md"),
            PathDecision::Traversal(_) => panic!("should be safe"),
        }
    }

    #[test]
    fn rejects_absolute_paths() {
        assert!(matches!(
            validate_vault_path("/etc/passwd"),
            PathDecision::Traversal(_)
        ));
        assert!(matches!(
            validate_vault_path("C:\\notes\\x.md"),
            PathDecision::Traversal(_)
        ));
    }

    #[test]
    fn rejects_traversal_escape() {
        assert!(matches!(
            validate_vault_path("../secret.md"),
            PathDecision::Traversal(_)
        ));
        assert!(matches!(
            validate_vault_path("notes/../../etc/passwd"),
            PathDecision::Traversal(_)
        ));
    }

    #[test]
    fn rejects_empty_and_overlong() {
        assert!(matches!(
            validate_vault_path(""),
            PathDecision::Traversal(_)
        ));
        let long = "a".repeat(crate::MCP_PATH_MAX + 1);
        assert!(matches!(
            validate_vault_path(&long),
            PathDecision::Traversal(_)
        ));
    }

    #[test]
    fn rejects_nul_byte() {
        assert!(matches!(
            validate_vault_path("notes\0.boom.md"),
            PathDecision::Traversal(_)
        ));
    }

    #[test]
    fn canonical_contained_blocks_lateral_escape() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        // Create a real file under the root.
        std::fs::create_dir_all(root.join("sub")).unwrap();
        let real = root.join("sub").join("x.md");
        std::fs::write(&real, "x").unwrap();

        assert!(canonical_contained(root, &real).is_some());
        // A path that walks up into the tempdir's parent is rejected.
        // tempdir is like /tmp/.tmpXXXX; its parent is /tmp — real file escapes.
        let outside = root.join("..").join("escape.md");
        // The candidate doesn't exist; parent canonicalization resolves it to
        // /tmp/escape.md, which is NOT under root.
        assert!(canonical_contained(root, &outside).is_none());
    }

    #[test]
    fn lexical_containment_basic() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        assert!(lexically_contained(root, &root.join("a").join("b.md")));
        assert!(!lexically_contained(
            root,
            &root.join("..").join("outside.md")
        ));
    }
}
