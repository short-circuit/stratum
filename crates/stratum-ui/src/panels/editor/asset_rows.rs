//! Attachment row rendering for the native block editor.
//!
//! [`crate::panels::editor::rows`] calls this module for rows that embed an
//! asset. An image row with a decoded thumbnail shows the thumbnail sized into
//! [`stratum_editor::THUMBNAIL_BOX`] plus the shipped `AttachmentPanel` caption
//! (basename, MIME, size). A non-image row shows the file chip with the
//! `FileText` icon. An image whose thumbnail is not decoded yet keeps the
//! existing chip fallback rendered by the row builder, so scrolling never waits
//! for a decode.
//!
//! All geometry and labels come from the headless model: `display_size` keeps
//! the aspect ratio, `attachment_label` composes the caption, and the theme
//! supplies the border/muted colors.

use std::sync::Arc;

use gpui_kit::component::{Icon, Sizable, Theme};
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_editor::{display_name, AssetKind, AssetThumbnail, DecoratedRow, THUMBNAIL_BOX};

use crate::panels::editor::asset_state::attachment_label;
use crate::panels::editor::EditorView;

/// Whether the row uses the attachment layout instead of the text chip.
///
/// Image rows keep the shipped chip until their thumbnail is decoded; file
/// rows always use the file chip. This predicate is pure so the contract tests
/// can check the fallback without a window.
pub fn renders_as_asset(row: &DecoratedRow) -> bool {
    match &row.asset {
        Some(asset) => asset.kind == AssetKind::File || row.thumbnail.is_some(),
        None => false,
    }
}

/// Build the attachment element for a row that [`renders_as_asset`] accepted.
///
/// The provider's attachment record (when known) supplies the MIME and size
/// label; unknown records fall back to the basename only.
pub fn asset_row_element(view: &EditorView, row: &DecoratedRow, theme: &Theme) -> AnyElement {
    let Some(asset) = &row.asset else {
        return div().into_any_element();
    };
    let record = view.attachment_row(&asset.path);
    match asset.kind {
        AssetKind::Image => match &row.thumbnail {
            Some(thumbnail) => image_element(&asset.path, record, thumbnail, theme),
            None => chip_element(display_name(&asset.path), theme),
        },
        AssetKind::File => file_element(&asset.path, record, theme),
    }
}

/// Image thumbnail plus the shipped attachment caption line.
fn image_element(
    ref_path: &str,
    record: Option<&stratum_editor::AttachmentRow>,
    thumbnail: &AssetThumbnail,
    theme: &Theme,
) -> AnyElement {
    let (width, height) = thumbnail.display_size(THUMBNAIL_BOX.0, THUMBNAIL_BOX.1);
    let image = Image::from_bytes(ImageFormat::Png, thumbnail.png.as_ref().clone());
    div()
        .flex()
        .flex_col()
        .items_start()
        .gap(px(4.0))
        .child(
            img(Arc::new(image))
                .w(px(width.max(1.0)))
                .h(px(height.max(1.0)))
                .rounded(px(6.0))
                .border_1()
                .border_color(theme.colors.border),
        )
        .child(
            div()
                .text_size(px(11.0))
                .text_color(theme.colors.muted_foreground)
                .child(attachment_label(ref_path, record)),
        )
        .into_any_element()
}

/// File chip: icon, basename and the MIME/size details when known.
fn file_element(
    ref_path: &str,
    record: Option<&stratum_editor::AttachmentRow>,
    theme: &Theme,
) -> AnyElement {
    div()
        .flex()
        .items_center()
        .gap(px(6.0))
        .px(px(8.0))
        .py(px(4.0))
        .rounded(px(6.0))
        .border_1()
        .border_color(theme.colors.border)
        .bg(theme.colors.secondary)
        .child(
            Icon::new(gpui_kit::assets::IconName::FileText)
                .with_size(px(14.0))
                .text_color(theme.colors.muted_foreground),
        )
        .child(
            div()
                .text_size(px(12.0))
                .text_color(theme.colors.muted_foreground)
                .child(attachment_label(ref_path, record)),
        )
        .into_any_element()
}

/// Chip fallback for an image row whose thumbnail is not available.
fn chip_element(label: &str, theme: &Theme) -> AnyElement {
    div()
        .flex()
        .items_center()
        .px(px(8.0))
        .py(px(4.0))
        .rounded(px(4.0))
        .bg(theme.colors.secondary)
        .child(
            div()
                .text_size(px(12.0))
                .text_color(theme.colors.primary)
                .child(label.to_string()),
        )
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::core::prelude::v1::test;
    use chrono::Utc;
    use stratum_editor::{dto_to_blocks, render_page, BlockDto, EditorDocument, RenderMode};
    use uuid::Uuid;

    fn row_for(content: &str) -> DecoratedRow {
        let blocks = dto_to_blocks(vec![BlockDto {
            id: Uuid::new_v4().to_string(),
            content: content.to_string(),
            parent_id: None,
            left_id: None,
            properties: Vec::new(),
            marker: None,
            priority: None,
            collapsed: false,
            heading_level: None,
        }]);
        let document = EditorDocument::from_blocks("pages/Test.md", blocks).expect("document");
        render_page(&document, None, RenderMode::StageA, Utc::now())
            .into_iter()
            .next()
            .expect("row")
    }

    fn thumbnail() -> AssetThumbnail {
        AssetThumbnail {
            png: Arc::new(vec![1, 2, 3]),
            width: 320,
            height: 180,
        }
    }

    #[test]
    fn image_rows_keep_the_chip_until_decoded() {
        let mut row = row_for("![[assets/photo.png]]");
        assert_eq!(
            row.asset.as_ref().map(|asset| asset.kind),
            Some(AssetKind::Image)
        );
        assert!(!renders_as_asset(&row));
        row.thumbnail = Some(Arc::new(thumbnail()));
        assert!(renders_as_asset(&row));
    }

    #[test]
    fn file_rows_always_use_the_file_chip() {
        let row = row_for("![[assets/report.pdf]]");
        assert_eq!(
            row.asset.as_ref().map(|asset| asset.kind),
            Some(AssetKind::File)
        );
        assert!(renders_as_asset(&row));
    }

    #[test]
    fn plain_rows_are_not_asset_rows() {
        let row = row_for("just text");
        assert!(!renders_as_asset(&row));
    }

    #[test]
    fn mixed_image_rows_do_not_promote_even_with_a_thumbnail() {
        let mut row = row_for("see ![[assets/photo.png]] above");
        assert!(
            row.asset.is_none(),
            "mixed content must not carry an asset span"
        );
        assert!(!renders_as_asset(&row));

        // A decoded thumbnail must not flip a mixed row into an asset row.
        row.thumbnail = Some(Arc::new(thumbnail()));
        assert!(row.asset.is_none());
        assert!(!renders_as_asset(&row));
    }

    #[test]
    fn mixed_file_rows_do_not_promote() {
        let row = row_for("report: ![[assets/report.pdf]] attached");
        assert!(row.asset.is_none());
        assert!(!renders_as_asset(&row));
    }

    #[test]
    fn thumbnails_fit_the_shared_box_without_upscaling() {
        let thumbnail = thumbnail();
        assert_eq!(
            thumbnail.display_size(THUMBNAIL_BOX.0, THUMBNAIL_BOX.1),
            (320.0, 180.0)
        );
        let small = AssetThumbnail {
            png: Arc::new(Vec::new()),
            width: 64,
            height: 32,
        };
        assert_eq!(
            small.display_size(THUMBNAIL_BOX.0, THUMBNAIL_BOX.1),
            (64.0, 32.0)
        );
    }
}
