//! Attachment state and orchestration for the native block editor.
//!
//! This module owns the view-side half of the attachment feature: the decoded
//! thumbnail cache, the attachment list of the open page, the import/delete
//! flows and the lazy thumbnail budget. The pure models (classification, row
//! labels, size formatting, cache) live in `stratum_editor::assets`; the
//! persistence lives behind [`crate::provider::ShellData`]. The view only wires
//! those pieces together.
//!
//! Imports never touch the UI thread: file reads and provider calls run on the
//! gpui background executor, and the result is applied back on the entity. The
//! thumbnail budget is shared with the math/Mermaid renderer
//! ([`crate::panels::editor::EditorView::asset_budget`]) so at most one new
//! decode happens per frame.
//!
//! Every user-visible label resolves through `stratum_i18n`; the error mapping
//! keeps the provider's `file_too_large` sentinel intact so the panel can show
//! the shipped size-limit message instead of a generic failure.

use std::sync::Arc;

use gpui_kit::*;
use stratum_editor::{
    display_name, format_size, AssetKind, AssetThumbnail, AssetThumbnailCache, AttachmentRow,
    DecoratedRow,
};
use uuid::Uuid;

use crate::panels::editor::rows::row_index_at_point;
use crate::panels::editor::EditorView;

/// Provider error sentinel for an oversized non-image upload.
///
/// Matches `"file_too_large"` in `src/lib/assetUpload.ts`; the panel maps it to
/// the shipped `asset.file_too_large` label.
pub const FILE_TOO_LARGE: &str = "file_too_large";

/// MIME type for a clipboard image format.
///
/// Delegates to gpui's canonical mapping so a pasted JPEG is uploaded as
/// `image/jpeg` instead of being re-labelled from its `paste.*` file name.
pub fn mime_for_image_format(format: ImageFormat) -> &'static str {
    format.mime_type()
}

/// The label key for a failed upload.
///
/// The provider passes its error string through unchanged, so the oversized
/// non-image sentinel is recognised here and everything else falls back to the
/// generic failure label.
pub fn upload_error_key(error: &str) -> &'static str {
    if error == FILE_TOO_LARGE {
        "asset.file_too_large"
    } else {
        "asset.upload_failed"
    }
}

/// One thumbnail task chosen for the visible range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThumbnailTask {
    /// The thumbnail is already cached; attach it to the row at `index`.
    Cached {
        /// Row index in the decorated row list.
        index: usize,
        /// Cached thumbnail.
        thumbnail: Arc<AssetThumbnail>,
    },
    /// The provider must decode `path` for the row at `index`.
    Fetch {
        /// Row index in the decorated row list.
        index: usize,
        /// Vault-relative asset path.
        path: String,
    },
}

/// Choose the next thumbnail task for a visible range.
///
/// Returns `None` when the range has no image row that still needs a
/// thumbnail, or when `budget` is exhausted. Rows that already carry a
/// thumbnail are skipped; the first remaining image row is returned, preferring
/// an already-cached thumbnail over a provider decode. Paths in `failed` are
/// skipped so one corrupt image cannot starve every later thumbnail. Callers
/// attach a [`ThumbnailTask::Cached`] result without spending budget and spend
/// one unit for a [`ThumbnailTask::Fetch`], so the loop always makes progress.
pub fn next_thumbnail_task(
    rows: &[DecoratedRow],
    cache: &AssetThumbnailCache,
    failed: &std::collections::HashSet<String>,
    range: std::ops::Range<usize>,
    budget: usize,
) -> Option<ThumbnailTask> {
    if budget == 0 {
        return None;
    }
    for index in range {
        let row = rows.get(index)?;
        if row.thumbnail.is_some() {
            continue;
        }
        let Some(asset) = &row.asset else {
            continue;
        };
        if asset.kind != AssetKind::Image {
            continue;
        }
        if failed.contains(&asset.path) {
            continue;
        }
        if let Some(thumbnail) = cache.get(&asset.path) {
            return Some(ThumbnailTask::Cached { index, thumbnail });
        }
        return Some(ThumbnailTask::Fetch {
            index,
            path: asset.path.clone(),
        });
    }
    None
}

/// The block a drop at `position` should insert the attachment reference into.
///
/// The pointer position is resolved against the virtual layout; when it lands
/// outside the list the caret block is used instead. A drop reported at the
/// window origin (some platforms do this for external drags) falls back to the
/// last pointer position recorded from mouse events.
pub fn drop_target_block(view: &EditorView, position: Point<Pixels>) -> Option<Uuid> {
    let position = if position.x == px(0.0) && position.y == px(0.0) {
        view.last_pointer
    } else {
        position
    };
    row_index_at_point(view, position)
        .and_then(|index| view.rows.get(index).map(|row| row.block_id))
        .or_else(|| view.state.caret().map(|caret| caret.block_id))
}

/// The display label for an attachment row.
///
/// Mirrors the shipped `AttachmentPanel` row: the basename, followed by the
/// MIME and the `formatBytes` size label when the provider knows the record.
/// Without a record only the basename is shown (the chip fallback).
pub fn attachment_label(ref_path: &str, record: Option<&AttachmentRow>) -> String {
    let name = display_name(ref_path);
    let Some(record) = record else {
        return name.to_string();
    };
    let mut parts = vec![name.to_string()];
    if !record.mime.is_empty() {
        parts.push(record.mime.clone());
    }
    if record.size > 0 {
        parts.push(record.size_label.clone());
    }
    parts.join(" · ")
}

/// Size label for a list of attachment rows, or `None` when the list is empty.
pub fn total_size_label(rows: &[AttachmentRow]) -> Option<String> {
    if rows.is_empty() {
        return None;
    }
    Some(format_size(rows.iter().map(|row| row.size).sum()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::core::prelude::v1::test;
    use chrono::Utc;
    use stratum_editor::{
        asset_kind, dto_to_blocks, render_page, BlockDto, EditorDocument, RenderMode,
    };
    use stratum_i18n::t_global;

    /// Build decorated rows for a sequence of block contents.
    fn rows_for(contents: &[&str]) -> Vec<DecoratedRow> {
        let blocks = dto_to_blocks(
            contents
                .iter()
                .map(|content| BlockDto {
                    id: Uuid::new_v4().to_string(),
                    content: content.to_string(),
                    parent_id: None,
                    left_id: None,
                    properties: Vec::new(),
                    marker: None,
                    priority: None,
                    collapsed: false,
                    heading_level: None,
                })
                .collect(),
        );
        let document = EditorDocument::from_blocks("pages/Test.md", blocks).expect("document");
        render_page(&document, None, RenderMode::StageA, Utc::now())
    }

    /// Build decorated rows for one asset embed.
    fn asset_rows(content: &str) -> Vec<DecoratedRow> {
        rows_for(&[content])
    }

    fn thumbnail() -> AssetThumbnail {
        AssetThumbnail {
            png: Arc::new(vec![1, 2, 3]),
            width: 64,
            height: 32,
        }
    }

    fn record(ref_path: &str, mime: &str, size: u64) -> AttachmentRow {
        AttachmentRow {
            ref_path: ref_path.to_string(),
            name: display_name(ref_path).to_string(),
            mime: mime.to_string(),
            size,
            size_label: format_size(size),
            kind: asset_kind(ref_path),
        }
    }

    #[test]
    fn thumbnail_task_prefers_cache_then_fetch() {
        let rows = asset_rows("![[assets/photo.png]]");
        assert!(rows[0].thumbnail.is_none());
        let failed = std::collections::HashSet::new();
        let cache = AssetThumbnailCache::new(4);
        assert_eq!(
            next_thumbnail_task(&rows, &cache, &failed, 0..rows.len(), 1),
            Some(ThumbnailTask::Fetch {
                index: 0,
                path: "assets/photo.png".to_string()
            })
        );
        assert_eq!(
            next_thumbnail_task(&rows, &cache, &failed, 0..rows.len(), 0),
            None
        );

        let mut cache = AssetThumbnailCache::new(4);
        cache.insert("assets/photo.png", thumbnail());
        let Some(ThumbnailTask::Cached { index, thumbnail }) =
            next_thumbnail_task(&rows, &cache, &failed, 0..rows.len(), 1)
        else {
            panic!("cached thumbnail expected");
        };
        assert_eq!(index, 0);
        assert_eq!(thumbnail.width, 64);
    }

    #[test]
    fn thumbnail_task_skips_file_rows_and_empty_ranges() {
        let rows = asset_rows("![[assets/report.pdf]]");
        let failed = std::collections::HashSet::new();
        let cache = AssetThumbnailCache::new(4);
        assert_eq!(
            next_thumbnail_task(&rows, &cache, &failed, 0..rows.len(), 1),
            None
        );
        assert_eq!(
            next_thumbnail_task(&rows, &cache, &failed, 5..9, 1),
            None,
            "out-of-range indices are ignored"
        );
    }

    #[test]
    fn thumbnail_task_skips_failed_paths_in_favour_of_the_next_image() {
        let rows = rows_for(&["![[assets/broken.png]]", "![[assets/photo.png]]"]);
        let cache = AssetThumbnailCache::new(4);
        let mut failed = std::collections::HashSet::new();
        failed.insert("assets/broken.png".to_string());
        assert_eq!(
            next_thumbnail_task(&rows, &cache, &failed, 0..rows.len(), 1),
            Some(ThumbnailTask::Fetch {
                index: 1,
                path: "assets/photo.png".to_string()
            })
        );
    }

    #[test]
    fn image_format_mime_types_match_the_upload_policy() {
        assert_eq!(mime_for_image_format(ImageFormat::Png), "image/png");
        assert_eq!(mime_for_image_format(ImageFormat::Jpeg), "image/jpeg");
        assert_eq!(mime_for_image_format(ImageFormat::Webp), "image/webp");
    }

    #[test]
    fn upload_errors_map_to_panel_labels() {
        assert_eq!(upload_error_key(FILE_TOO_LARGE), "asset.file_too_large");
        assert_eq!(upload_error_key("disk full"), "asset.upload_failed");
        assert_ne!(t_global(upload_error_key(FILE_TOO_LARGE)), FILE_TOO_LARGE);
        assert_ne!(
            t_global(upload_error_key("disk full")),
            "asset.upload_failed"
        );
    }

    #[test]
    fn attachment_labels_include_record_details_only_when_known() {
        assert_eq!(attachment_label("assets/photo.png", None), "photo.png");
        let known = record("assets/photo.png", "image/png", 2048);
        assert_eq!(
            attachment_label("assets/photo.png", Some(&known)),
            "photo.png · image/png · 2.0 KB"
        );
        let zero = record("assets/empty.png", "", 0);
        assert_eq!(
            attachment_label("assets/empty.png", Some(&zero)),
            "empty.png"
        );
    }

    #[test]
    fn total_size_label_sums_known_rows() {
        assert_eq!(total_size_label(&[]), None);
        let rows = vec![
            record("assets/a.png", "image/png", 1024),
            record("assets/b.pdf", "application/pdf", 1024),
        ];
        assert_eq!(total_size_label(&rows), Some("2.0 KB".to_string()));
    }
}
