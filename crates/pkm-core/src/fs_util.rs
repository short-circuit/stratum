//! Filesystem utilities for recursive file collection and text truncation.
//!
//! Provides [`MdCollector`] — a configurable builder for recursively walking
//! directories and collecting files by extension. This is the single shared
//! implementation that replaces four near-identical copies in the codebase:
//!
//! - `pkm-index/src/rebuild.rs` — `collect_md_files`
//! - `src-tauri/src/commands/page.rs` — `find_md_files`
//! - `crates/pkm-cli/src/main.rs` — `collect_md_files`
//!
//! Also provides [`truncate_text`] as a shared replacement for the private
//! `snippet_from_text` in `src-tauri/src/commands/search.rs`.

use crate::PkmError;
use crate::PkmResult;
use std::path::{Path, PathBuf};

/// Builder for recursive file collection.
///
/// # Examples
///
/// ```ignore
/// use pkm_core::fs_util::MdCollector;
///
/// // Collect all .md files, skipping hidden directories (rebuild.rs style)
/// let files = MdCollector::new()
///     .skip_hidden_dirs(true)
///     .collect(vault_path)?;
///
/// // Collect .md + extensionless files, skip specific dirs (page.rs style)
/// let files = MdCollector::new()
///     .include_extensionless(true)
///     .skip_dirs(vec![".pkm", "templates", ".git"])
///     .collect_relative(vault_path, vault_path)?;
///
/// // Collect .md files with a depth limit (pkm-cli style)
/// let files = MdCollector::new()
///     .max_depth(8)
///     .collect(vault)?;
/// ```
#[derive(Default)]
pub struct MdCollector {
    /// File extensions to include (without dot, e.g. `"md"`).
    /// Default: `["md"]`.
    extensions: Vec<String>,

    /// Whether to also include files that have no extension.
    /// Default: `false`.
    include_extensionless: bool,

    /// Skip directories whose name starts with `.`.
    /// Default: `false`.
    skip_hidden_dirs: bool,

    /// Specific directory basenames to skip (e.g. `".pkm"`, `"templates"`).
    /// Default: empty.
    skip_dirs: Vec<String>,

    /// Maximum recursion depth. `0` means only the given directory, `None` means no limit.
    /// Default: `None`.
    max_depth: Option<usize>,
}

impl MdCollector {
    /// Create a new `MdCollector` with default settings.
    ///
    /// Defaults: collects `.md` files recursively, does not skip hidden dirs,
    /// does not include extensionless files, no depth limit.
    pub fn new() -> Self {
        Self {
            extensions: vec!["md".to_string()],
            include_extensionless: false,
            skip_hidden_dirs: false,
            skip_dirs: Vec::new(),
            max_depth: None,
        }
    }

    /// Set the file extensions to collect (without leading dot).
    ///
    /// Default: `["md"]`.
    pub fn extensions(mut self, exts: Vec<&str>) -> Self {
        self.extensions = exts.into_iter().map(|s| s.to_string()).collect();
        self
    }

    /// Include files that have no extension.
    ///
    /// Default: `false`.
    pub fn include_extensionless(mut self, val: bool) -> Self {
        self.include_extensionless = val;
        self
    }

    /// Skip directories whose name starts with `.`.
    ///
    /// Default: `false`.
    pub fn skip_hidden_dirs(mut self, val: bool) -> Self {
        self.skip_hidden_dirs = val;
        self
    }

    /// Specific directory basenames to skip during traversal.
    ///
    /// Default: empty.
    pub fn skip_dirs(mut self, dirs: Vec<&str>) -> Self {
        self.skip_dirs = dirs.into_iter().map(|s| s.to_string()).collect();
        self
    }

    /// Maximum recursion depth. `0` means only the given directory is scanned,
    /// no subdirectories. `None` (default) means no limit.
    pub fn max_depth(mut self, depth: usize) -> Self {
        self.max_depth = Some(depth);
        self
    }

    /// Recursively walk `dir`, collecting matching file paths.
    ///
    /// Returns full (absolute or relative — whatever is passed in) paths.
    /// Errors if `dir` does not exist or is not readable.
    pub fn collect(&self, dir: &Path) -> PkmResult<Vec<PathBuf>> {
        let mut files = Vec::new();
        if !dir.exists() {
            return Err(PkmError::NotFound(format!(
                "Directory not found: {}",
                dir.display()
            )));
        }
        self.collect_recursive(dir, &mut files, 0)?;
        files.sort();
        Ok(files)
    }

    /// Recursively walk `dir`, collecting paths relative to `base`.
    ///
    /// Same as [`collect`](MdCollector::collect) but strips `base` from each path
    /// and returns `String` paths suitable for use as vault-relative identifiers.
    pub fn collect_relative(&self, dir: &Path, base: &Path) -> PkmResult<Vec<String>> {
        let absolute = self.collect(dir)?;
        let result = absolute
            .into_iter()
            .map(|p| {
                p.strip_prefix(base)
                    .unwrap_or(&p)
                    .to_string_lossy()
                    .to_string()
            })
            .collect();
        Ok(result)
    }

    fn collect_recursive(
        &self,
        dir: &Path,
        files: &mut Vec<PathBuf>,
        depth: usize,
    ) -> std::io::Result<()> {
        // Depth limit check
        if let Some(max) = self.max_depth {
            if depth > max {
                return Ok(());
            }
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            // Get the filename for skip checks
            let file_name = path.file_name().and_then(|s| s.to_str());

            if path.is_dir() {
                // Skip hidden dirs
                if self.skip_hidden_dirs {
                    if let Some(name) = file_name {
                        if name.starts_with('.') {
                            continue;
                        }
                    }
                }
                // Skip specific named dirs
                if let Some(name) = file_name {
                    if self.skip_dirs.iter().any(|d| d == name) {
                        continue;
                    }
                }
                self.collect_recursive(&path, files, depth + 1)?;
            } else if path.is_file() {
                let ext = path.extension().and_then(|s| s.to_str());
                let has_valid_ext = ext.is_some_and(|e| self.extensions.iter().any(|x| x == e));
                let is_extensionless = ext.is_none() && self.include_extensionless;
                if has_valid_ext || is_extensionless {
                    files.push(path);
                }
            }
        }

        Ok(())
    }
}

/// Truncate text to a maximum length, appending `...` if truncated.
///
/// This is the shared replacement for the private `snippet_from_text` function
/// found in `src-tauri/src/commands/search.rs`.
pub fn truncate_text(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        return text.to_string();
    }
    let end = text
        .char_indices()
        .take(max_len)
        .last()
        .map(|(i, c)| i + c.len_utf8())
        .unwrap_or(max_len);
    format!("{}...", &text[..end])
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_vault(dir: &Path) {
        std::fs::create_dir_all(dir.join("subdir")).unwrap();
        std::fs::create_dir_all(dir.join(".pkm")).unwrap();
        std::fs::create_dir_all(dir.join(".git")).unwrap();
        std::fs::create_dir_all(dir.join("templates")).unwrap();

        std::fs::write(dir.join("note-a.md"), "Note A").unwrap();
        std::fs::write(dir.join("note-b.md"), "Note B").unwrap();
        std::fs::write(dir.join("subdir").join("note-c.md"), "Note C").unwrap();
        std::fs::write(dir.join("subdir").join("note-d.markdown"), "Note D").unwrap();
        std::fs::write(dir.join(".pkm").join("cache.md"), "cache").unwrap();
        std::fs::write(dir.join(".git").join("config.md"), "config").unwrap();
        std::fs::write(dir.join("templates").join("tpl.md"), "template").unwrap();
        std::fs::write(dir.join("readme.txt"), "Not markdown.").unwrap();
        std::fs::write(dir.join("extensionless"), "No extension").unwrap();
    }

    // ── Default behavior (collect .md files only, no skip) ──

    #[test]
    fn test_collect_default() {
        let dir = TempDir::new().unwrap();
        create_test_vault(dir.path());

        let files = MdCollector::new()
            .collect(dir.path())
            .expect("collect should succeed");
        // With defaults: no skip-hidden, so all .md files including hidden dirs
        // Files: note-a.md, note-b.md, subdir/note-c.md, .pkm/cache.md, .git/config.md, templates/tpl.md
        assert_eq!(files.len(), 6);
    }

    // ── rebuild.rs style: skip_hidden_dirs(true) ──

    #[test]
    fn test_collect_skip_hidden() {
        let dir = TempDir::new().unwrap();
        create_test_vault(dir.path());

        let files = MdCollector::new()
            .skip_hidden_dirs(true)
            .collect(dir.path())
            .expect("collect should succeed");
        // Skips .pkm, .git — still includes templates (doesn't start with '.')
        let filenames: Vec<String> = files
            .iter()
            .map(|f| f.file_name().unwrap().to_str().unwrap().to_string())
            .collect();
        assert!(filenames.contains(&"note-a.md".to_string()));
        assert!(filenames.contains(&"note-b.md".to_string()));
        assert!(filenames.contains(&"note-c.md".to_string()));
        assert!(filenames.contains(&"tpl.md".to_string()));
        assert!(!filenames.contains(&"cache.md".to_string()));
        assert!(!filenames.contains(&"config.md".to_string()));
        assert_eq!(filenames.len(), 4);
    }

    // ── page.rs style: include_extensionless + skip specific dirs ──

    #[test]
    fn test_collect_include_extensionless_and_skip_dirs() {
        let dir = TempDir::new().unwrap();
        create_test_vault(dir.path());

        let files = MdCollector::new()
            .include_extensionless(true)
            .skip_dirs(vec![".pkm", "templates", ".git"])
            .collect(dir.path())
            .expect("collect should succeed");
        // Should include: note-a.md, note-b.md, subdir/note-c.md, extensionless
        // But NOT: .pkm/cache.md, .git/config.md, templates/tpl.md, readme.txt
        let filenames: Vec<String> = files
            .iter()
            .map(|f| f.file_name().unwrap().to_str().unwrap().to_string())
            .collect();
        assert!(filenames.contains(&"note-a.md".to_string()));
        assert!(filenames.contains(&"extensionless".to_string()));
        assert!(!filenames.contains(&"cache.md".to_string()));
        assert!(!filenames.contains(&"tpl.md".to_string()));
        assert!(!filenames.contains(&"config.md".to_string()));
        assert!(!filenames.contains(&"readme.txt".to_string()));
    }

    #[test]
    fn test_collect_relative() {
        let dir = TempDir::new().unwrap();
        create_test_vault(dir.path());

        let files = MdCollector::new()
            .skip_hidden_dirs(true)
            .collect_relative(dir.path(), dir.path())
            .expect("collect_relative should succeed");
        assert!(files.contains(&"note-a.md".to_string()));
        assert!(files.contains(&"subdir/note-c.md".to_string()));
        assert!(!files.contains(&"cache.md".to_string()));
    }

    // ── pkm-cli style: max_depth + only .md ──

    #[test]
    fn test_collect_max_depth() {
        let dir = TempDir::new().unwrap();
        create_test_vault(dir.path());

        // depth 0 means we don't enter subdirectories
        let files_depth0 = MdCollector::new()
            .max_depth(0)
            .collect(dir.path())
            .expect("collect should succeed");
        let filenames: Vec<String> = files_depth0
            .iter()
            .map(|f| f.file_name().unwrap().to_str().unwrap().to_string())
            .collect();
        // Only files in the root directory (note-a.md, note-b.md, .pkm/cache.md not in root listing)
        // Wait, depth 0 means no recursion into subdirs. So only files directly in dir.
        assert!(filenames.contains(&"note-a.md".to_string()));
        assert!(filenames.contains(&"note-b.md".to_string()));
        // subdir/note-c.md should NOT be present (depth 0)
        assert!(!filenames.contains(&"note-c.md".to_string()));
    }

    #[test]
    fn test_collect_markdown_extension() {
        let dir = TempDir::new().unwrap();
        create_test_vault(dir.path());

        let files = MdCollector::new()
            .extensions(vec!["md", "markdown"])
            .skip_hidden_dirs(true)
            .collect(dir.path())
            .expect("collect should succeed");
        let filenames: Vec<String> = files
            .iter()
            .map(|f| f.file_name().unwrap().to_str().unwrap().to_string())
            .collect();
        assert!(filenames.contains(&"note-d.markdown".to_string()));
        assert!(filenames.contains(&"note-c.md".to_string()));
    }

    // ── Error cases ──

    #[test]
    fn test_collect_nonexistent_directory() {
        let dir = TempDir::new().unwrap();
        let nonexistent = dir.path().join("does-not-exist");
        let result = MdCollector::new().collect(&nonexistent);
        assert!(result.is_err());
        match result {
            Err(PkmError::NotFound(_)) => {} // expected
            _ => panic!("Expected NotFound error"),
        }
    }

    // ── truncate_text ──

    #[test]
    fn test_truncate_text_short() {
        assert_eq!(truncate_text("hello", 80), "hello");
    }

    #[test]
    fn test_truncate_text_exact() {
        assert_eq!(truncate_text("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_text_long() {
        let long = "a".repeat(200);
        let result = truncate_text(&long, 80);
        assert_eq!(result.len(), 83); // 80 chars + "..."
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_truncate_text_unicode() {
        let text = "héllo wörld";
        let result = truncate_text(text, 6);
        assert!(result.len() <= 10); // 6 chars + "..." = 9, but unicode means careful
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_truncate_text_empty() {
        assert_eq!(truncate_text("", 80), "");
    }
}
