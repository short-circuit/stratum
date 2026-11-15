//! OS drag-and-drop classification.
//!
//! gpui already delivers native drops: a window element can listen for
//! `ExternalPaths` (the editor imports dropped files as attachments). This module
//! holds the shell-level rule the shipped webview never had: dropping a note
//! from the file manager onto the app opens that note. Classification is pure so
//! it can be tested without a window.

use std::path::{Path, PathBuf};

/// What a dropped path means for the shell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DroppedItem {
    /// A Markdown note inside the vault: open it in the editor.
    Page(String),
    /// A file the editor can attach (image, PDF, audio, video).
    Asset(PathBuf),
    /// Anything else: the shell ignores it.
    Ignored,
}

/// The extensions the editor imports as attachments.
pub const ASSET_EXTENSIONS: [&str; 15] = [
    "png", "jpg", "jpeg", "gif", "webp", "svg", "bmp", "pdf", "mp3", "wav", "flac", "ogg", "m4a",
    "mp4", "webm",
];

/// Classify dropped paths against the vault root.
///
/// A `.md` file inside the vault becomes a [`DroppedItem::Page`] with a
/// vault-relative path using `/` separators (the same shape the router and the
/// provider expect). Known attachment types become [`DroppedItem::Asset`];
/// everything else is ignored.
pub fn classify(vault_root: &Path, paths: &[PathBuf]) -> Vec<DroppedItem> {
    paths
        .iter()
        .map(|path| classify_one(vault_root, path))
        .collect()
}

/// The first dropped note that opens as a page, if any.
pub fn dropped_page(vault_root: &Path, paths: &[PathBuf]) -> Option<String> {
    classify(vault_root, paths)
        .into_iter()
        .find_map(|item| match item {
            DroppedItem::Page(path) => Some(path),
            _ => None,
        })
}

fn classify_one(vault_root: &Path, path: &Path) -> DroppedItem {
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .unwrap_or_default();

    let absolute = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let root = vault_root
        .canonicalize()
        .unwrap_or_else(|_| vault_root.to_path_buf());

    if extension == "md" {
        if let Ok(relative) = absolute.strip_prefix(&root) {
            let mut page = String::new();
            for component in relative.components() {
                if !page.is_empty() {
                    page.push('/');
                }
                page.push_str(&component.as_os_str().to_string_lossy());
            }
            if !page.is_empty() {
                return DroppedItem::Page(page);
            }
        }
        return DroppedItem::Ignored;
    }

    if ASSET_EXTENSIONS.contains(&extension.as_str()) {
        return DroppedItem::Asset(path.to_path_buf());
    }

    DroppedItem::Ignored
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_vault_notes_open_as_pages() {
        let root = PathBuf::from("/vault");
        let dropped = classify(&root, &[PathBuf::from("/vault/projects/plan.md")]);
        assert_eq!(dropped, [DroppedItem::Page("projects/plan.md".into())]);
        assert_eq!(
            dropped_page(&root, &[PathBuf::from("/vault/projects/plan.md")]),
            Some("projects/plan.md".into())
        );
    }

    #[test]
    fn out_of_vault_notes_are_ignored() {
        let root = PathBuf::from("/vault");
        assert_eq!(
            classify(&root, &[PathBuf::from("/elsewhere/plan.md")]),
            [DroppedItem::Ignored]
        );
    }

    #[test]
    fn attachments_are_assets() {
        let root = PathBuf::from("/vault");
        let dropped = classify(
            &root,
            &[
                PathBuf::from("/tmp/sketch.PNG"),
                PathBuf::from("/tmp/report.pdf"),
                PathBuf::from("/tmp/notes.txt"),
            ],
        );
        assert_eq!(
            dropped[0],
            DroppedItem::Asset(PathBuf::from("/tmp/sketch.PNG"))
        );
        assert_eq!(
            dropped[1],
            DroppedItem::Asset(PathBuf::from("/tmp/report.pdf"))
        );
        assert_eq!(dropped[2], DroppedItem::Ignored);
    }

    #[test]
    fn mixed_drops_pick_the_first_note() {
        let root = PathBuf::from("/vault");
        let paths = [
            PathBuf::from("/tmp/shot.png"),
            PathBuf::from("/vault/Inbox.md"),
        ];
        assert_eq!(dropped_page(&root, &paths), Some("Inbox.md".into()));
    }
}
