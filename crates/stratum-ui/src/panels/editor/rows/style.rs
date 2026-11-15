//! Inline span styling and gutter badges for the native editor rows.
//!
//! These helpers were split out of [`super`] so the row renderer stays under the
//! repository file-length limit. [`span_style`] resolves a [`DecoratedSpan`]
//! into a font/colour/decoration set; [`marker_badge`] renders the marker and
//! priority gutter chip.

use gpui_kit::component::Theme;
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_editor::{BlockKind, DecoratedRow, DecoratedSpan, MarkerBadge, SpanKind};

use super::BADGE_WIDTH;

/// A resolved inline text style.
pub(super) struct SpanStyle {
    pub(super) font: Font,
    pub(super) color: Hsla,
    pub(super) background: Option<Hsla>,
    pub(super) underline: Option<UnderlineStyle>,
    pub(super) strikethrough: Option<StrikethroughStyle>,
}

/// Render the marker/priority gutter chip.
pub(super) fn marker_badge(badge: &MarkerBadge, theme: &Theme) -> impl IntoElement {
    let marker = badge
        .marker
        .map(|marker| marker.as_str())
        .unwrap_or_default();
    let priority = badge
        .priority
        .map(|priority| priority.as_str())
        .unwrap_or_default();
    let label = if priority.is_empty() {
        marker.to_string()
    } else if marker.is_empty() {
        format!("[{priority}]")
    } else {
        format!("{marker} [{priority}]")
    };
    div()
        .flex()
        .items_center()
        .justify_center()
        .w(px(BADGE_WIDTH))
        .h_full()
        .text_size(px(10.0))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(theme.colors.accent_foreground)
        .child(label)
}

/// Resolve the visual style for a decorated span.
pub(super) fn span_style(span: &DecoratedSpan, row: &DecoratedRow, theme: &Theme) -> SpanStyle {
    let base_font = font(theme.font_family.clone());
    let mono_font = font(theme.mono_font_family.clone());
    let mut font = base_font;
    let mut color = if span.syntax {
        theme.colors.muted_foreground
    } else {
        theme.colors.foreground
    };
    let mut background = None;
    let mut underline = None;
    let mut strikethrough = None;

    if matches!(&row.kind, BlockKind::Heading(_)) {
        font = font.bold();
    }

    match &span.kind {
        SpanKind::Bold => font = font.bold(),
        SpanKind::Italic => font = font.italic(),
        SpanKind::Strike => {
            strikethrough = Some(StrikethroughStyle {
                thickness: px(1.0),
                color: Some(color),
            });
        }
        SpanKind::Code => {
            font = mono_font.clone();
            if !span.syntax {
                background = Some(theme.colors.secondary);
            }
        }
        SpanKind::Highlight => {
            background = Some(theme.colors.warning.opacity(0.25));
        }
        SpanKind::Link { .. } | SpanKind::WikiLink { .. } | SpanKind::BlockRef { .. } => {
            color = theme.colors.link;
            underline = Some(UnderlineStyle {
                thickness: px(1.0),
                color: Some(color),
                wavy: false,
            });
        }
        SpanKind::Asset { .. } => {
            // The chip fallback keeps the path text on the theme's primary /
            // secondary pair; decoded images and file attachments are rendered
            // by `asset_rows` instead of the decorated spans.
            color = theme.colors.primary;
            background = Some(theme.colors.secondary);
        }
        SpanKind::Tag => color = theme.colors.primary,
        SpanKind::Math { .. } => {
            font = mono_font.clone();
            color = theme.colors.link;
        }
        SpanKind::Text => {}
    }

    if matches!(&row.kind, BlockKind::Code { .. } | BlockKind::Mermaid) {
        font = mono_font;
        color = theme.colors.foreground;
    }

    SpanStyle {
        font,
        color,
        background,
        underline,
        strikethrough,
    }
}

/// Clamp `index` down to the nearest UTF-8 character boundary.
pub(super) fn floor_char_boundary(text: &str, mut index: usize) -> usize {
    index = index.min(text.len());
    while index > 0 && !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}
