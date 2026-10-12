//! Source vault format auto-detection.
//!
//! [`detect_format`] inspects the source directory's structure to guess which
//! importer should handle it. Detection is heuristic and never authoritative:
//! callers may override the format explicitly, and a wrong guess falls back to
//! the generic importer (which copies `.md` files verbatim).

use crate::report::ImportFormat;
use std::path::Path;

/// Detect the source vault format from its directory structure.
///
/// Heuristics, in order of precedence:
/// 1. A `.obsidian/` directory exists → [`ImportFormat::Obsidian`].
/// 2. Top-level `pages` **and** `journals` directories both exist →
///    [`ImportFormat::Logseq`].
/// 3. Otherwise → [`ImportFormat::Generic`].
///
/// This never returns [`ImportFormat::Auto`] — it always resolves to a
/// concrete importer, or `Generic` as the safe default.
pub fn detect_format(source_path: &Path) -> ImportFormat {
    if source_path.join(".obsidian").is_dir() {
        ImportFormat::Obsidian
    } else if source_path.join("pages").is_dir() && source_path.join("journals").is_dir() {
        ImportFormat::Logseq
    } else {
        ImportFormat::Generic
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn detect_obsidian() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join(".obsidian")).unwrap();
        assert_eq!(detect_format(dir.path()), ImportFormat::Obsidian);
    }

    #[test]
    fn detect_logseq() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("pages")).unwrap();
        std::fs::create_dir_all(dir.path().join("journals")).unwrap();
        assert_eq!(detect_format(dir.path()), ImportFormat::Logseq);
    }

    #[test]
    fn detect_generic_when_only_one_logseq_dir() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("pages")).unwrap();
        assert_eq!(detect_format(dir.path()), ImportFormat::Generic);
    }

    #[test]
    fn detect_generic_default() {
        let dir = TempDir::new().unwrap();
        assert_eq!(detect_format(dir.path()), ImportFormat::Generic);
    }
}
