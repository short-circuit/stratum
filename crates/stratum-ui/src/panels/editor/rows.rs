//! Row, span and caret elements for the native block editor.
//!
//! The view never parses block content: it renders the [`DecoratedRow`] values
//! produced by `stratum_editor::render_page`. This module turns those values into
//! gpui elements and keeps the small amount of geometry needed for the caret,
//! mouse hit testing and the Ctrl+hover preview.
//!
//! Styling follows the shipped React editor: syntax delimiters are muted in
//! Stage A and hidden on the focused line, inline emphasis/code/links/tags keep
//! their semantic styling, and marker/reminder chips sit in the row gutter.
//!
//! Every helper that does not touch gpui is pure so the contract tests can
//! exercise the source/visible offset mapping without a window.

use std::sync::Arc;

use gpui_kit::component::{ActiveTheme, Icon, Sizable, Theme};
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_editor::{BlockKind, DecoratedRow, DecoratedSpan, SpanKind};

use crate::panels::editor::EditorView;

/// Width used for virtual-list item measurement.
pub const ROW_WIDTH: f32 = 1000.0;
/// Left padding shared by every row.
pub const ROW_TEXT_LEFT: f32 = 32.0;
/// Extra indent per nesting level.
pub const ROW_DEPTH_INDENT: f32 = 18.0;
/// Width reserved for the collapse chevron.
pub const CHEVRON_WIDTH: f32 = 20.0;
/// Width reserved for a marker/priority badge.
pub const BADGE_WIDTH: f32 = 60.0;
/// Width reserved for a reminder chip.
pub const REMINDER_WIDTH: f32 = 96.0;

/// A resolved inline text style.
struct SpanStyle {
    font: Font,
    color: Hsla,
    background: Option<Hsla>,
    underline: Option<UnderlineStyle>,
    strikethrough: Option<StrikethroughStyle>,
}

/// The left offset of the row text, including nesting indentation.
pub fn row_text_left(depth: usize) -> f32 {
    ROW_TEXT_LEFT + depth as f32 * ROW_DEPTH_INDENT
}

/// The width of the inline gutter elements rendered before the text.
///
/// The value is deterministic so the caret and mouse hit testing share the same
/// geometry as the rendered row.
pub fn inline_offset(view: &EditorView, row: &DecoratedRow) -> f32 {
    let mut offset = 0.0;
    if !view
        .state
        .document()
        .tree()
        .children(row.block_id)
        .is_empty()
    {
        offset += CHEVRON_WIDTH;
    }
    if row.badge.is_some() {
        offset += BADGE_WIDTH;
    }
    if row.reminder.is_some() {
        offset += REMINDER_WIDTH;
    }
    offset
}

/// The font size used by a row.
pub fn row_font_size(row: &DecoratedRow, theme: &Theme) -> Pixels {
    match row.kind {
        BlockKind::Heading(1) => px(f32::from(theme.font_size) * 1.6),
        BlockKind::Heading(2) => px(f32::from(theme.font_size) * 1.35),
        BlockKind::Heading(3) => px(f32::from(theme.font_size) * 1.15),
        BlockKind::Code { .. } | BlockKind::Mermaid => theme.mono_font_size,
        _ => theme.font_size,
    }
}

/// Concatenated visible text for a row (hidden delimiters omitted).
pub fn visible_text(row: &DecoratedRow) -> String {
    let mut text = String::new();
    for span in &row.spans {
        if !span.hidden {
            text.push_str(&span.text);
        }
    }
    text
}

/// Visible text and styled runs for a row.
pub fn visible_runs(row: &DecoratedRow, theme: &Theme) -> (String, Vec<TextRun>) {
    visible_prefix(row, theme, usize::MAX)
}

/// Visible text and styled runs up to `source_offset` bytes of the stripped
/// block content.
///
/// The decorated spans preserve the source length in their `text` field (the
/// renderer synthesises delimiters with the same byte length as the source), so
/// walking the spans gives an exact source-to-visible mapping without parsing
/// the content again.
pub fn visible_prefix(
    row: &DecoratedRow,
    theme: &Theme,
    source_offset: usize,
) -> (String, Vec<TextRun>) {
    let mut text = String::new();
    let mut runs = Vec::new();
    let mut remaining = source_offset;
    for span in &row.spans {
        if remaining == 0 {
            break;
        }
        let take = remaining.min(span.text.len());
        let take = floor_char_boundary(&span.text, take);
        if !span.hidden {
            let style = span_style(span, row, theme);
            text.push_str(&span.text[..take]);
            runs.push(TextRun {
                len: take,
                font: style.font,
                color: style.color,
                background_color: style.background,
                underline: style.underline,
                strikethrough: style.strikethrough,
            });
        }
        remaining -= take;
        if take < span.text.len() {
            break;
        }
    }
    (text, runs)
}

/// Convert a visible byte offset back into a source offset.
pub fn source_offset_for_visible(row: &DecoratedRow, visible_offset: usize) -> usize {
    let mut source = 0;
    let mut visible = 0;
    for span in &row.spans {
        let len = span.text.len();
        if !span.hidden {
            if visible + len >= visible_offset {
                return source + (visible_offset - visible);
            }
            visible += len;
        }
        source += len;
    }
    source
}

/// Convert a source offset into a visible byte offset.
pub fn visible_offset_for_source(row: &DecoratedRow, source_offset: usize) -> usize {
    let mut source = 0;
    let mut visible = 0;
    for span in &row.spans {
        let len = span.text.len();
        if source + len >= source_offset {
            if span.hidden {
                return visible;
            }
            return visible + (source_offset - source).min(len);
        }
        source += len;
        if !span.hidden {
            visible += len;
        }
    }
    visible
}

/// Number of bytes removed from the start of a block before decoration.
///
/// `stratum_editor::render` strips a leading marker keyword; the caret and mouse
/// helpers subtract the same prefix so offsets map back onto the stored content.
pub fn marker_prefix_len(content: &str, keyword: Option<&str>) -> usize {
    let Some(keyword) = keyword else {
        return 0;
    };
    let trimmed_start = content.len() - content.trim_start().len();
    let rest = &content[trimmed_start..];
    if let Some(after) = rest.strip_prefix(keyword) {
        if after.is_empty() || after.starts_with(char::is_whitespace) {
            let after_start = trimmed_start + keyword.len();
            let whitespace =
                content[after_start..].len() - content[after_start..].trim_start().len();
            return after_start + whitespace;
        }
    }
    0
}

/// The caret position in row-local pixels, if this row owns the caret.
pub fn caret_position(
    view: &EditorView,
    row: &DecoratedRow,
    window: &mut Window,
    theme: &Theme,
) -> Option<(Pixels, Pixels)> {
    if view.read_mode {
        return None;
    }
    let caret = view.state.caret()?;
    if caret.block_id != row.block_id {
        return None;
    }
    let content = view.state.document().block(row.block_id)?.content.clone();
    let prefix = marker_prefix_len(&content, row.marker.map(|marker| marker.as_str()));
    let source_offset = caret.byte_offset.saturating_sub(prefix);
    let (text, runs) = visible_prefix(row, theme, source_offset);
    let font_size = row_font_size(row, theme);
    let layout = window
        .text_system()
        .layout_line(&text, font_size, &runs, None);
    let x = layout.x_for_index(text.len());
    Some((x, font_size))
}

/// The row index under a window-space point, if the point is inside the list.
pub fn row_index_at_point(view: &EditorView, point: Point<Pixels>) -> Option<usize> {
    let viewport = view.scroll.bounds();
    if point.x < viewport.origin.x
        || point.x > viewport.origin.x + viewport.size.width
        || point.y < viewport.origin.y
        || point.y > viewport.origin.y + viewport.size.height
    {
        return None;
    }
    let content_y =
        f32::from(point.y) - f32::from(viewport.origin.y) - f32::from(view.scroll.offset().y);
    view.layout.block_at_offset(content_y)
}

/// Build one row element.
pub fn row_element(
    view: &EditorView,
    index: usize,
    row: &DecoratedRow,
    window: &mut Window,
    cx: &mut Context<EditorView>,
) -> AnyElement {
    let theme = cx.theme();
    let text_left = row_text_left(row.depth);
    let height = view
        .layout
        .heights()
        .get(index)
        .copied()
        .unwrap_or(24.0)
        .max(stratum_editor::virtualize::MIN_ROW_HEIGHT);
    let font_size = row_font_size(row, theme);
    let inline = inline_offset(view, row);

    let mut element = div()
        .relative()
        .id(("editor-row", index))
        .flex()
        .items_start()
        .h(px(height))
        .w_full()
        .pl(px(text_left))
        .pr(px(12.0))
        .text_size(font_size)
        .text_color(theme.colors.foreground)
        .cursor_text();

    // The indent gutter is the drag affordance: it covers only the empty space
    // left of the row text, carries no listeners (the press bubbles to the root
    // handler) and stays cheap by rendering no children or text.
    if !view.read_mode {
        element = element.child(
            div()
                .absolute()
                .left(px(0.0))
                .top(px(0.0))
                .w(px(text_left))
                .h_full()
                .id(("editor-gutter", index))
                .cursor_grab(),
        );
    }

    if !view
        .state
        .document()
        .tree()
        .children(row.block_id)
        .is_empty()
    {
        let collapsed = row.collapsed;
        let block_id = row.block_id;
        element = element.child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .id(("editor-chevron", index))
                .w(px(CHEVRON_WIDTH))
                .h_full()
                .cursor_pointer()
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(caret) = this.state.caret() {
                        let _ = this.state.set_caret(block_id, caret.byte_offset);
                    }
                    let _ = this.state.toggle_collapsed();
                    this.rebuild_rows();
                    this.save.mark_dirty();
                    this.schedule_save(cx);
                    cx.notify();
                }))
                .child(
                    Icon::new(if collapsed {
                        gpui_kit::assets::IconName::ChevronRight
                    } else {
                        gpui_kit::assets::IconName::ChevronDown
                    })
                    .with_size(px(14.0))
                    .text_color(theme.colors.muted_foreground),
                ),
        );
    }

    if let Some(badge) = &row.badge {
        element = element.child(marker_badge(badge, theme));
    }
    if let Some(chip) = &row.reminder {
        let label = EditorView::reminder_label(chip);
        element = element.child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .w(px(REMINDER_WIDTH))
                .h_full()
                .text_size(px(10.0))
                .text_color(theme.colors.warning)
                .child(label),
        );
    }

    if let Some(math) = &row.math {
        let image = Image::from_bytes(ImageFormat::Png, math.png.clone());
        element = element.child(
            img(Arc::new(image))
                .w(px(math.width as f32))
                .h(px(math.height as f32)),
        );
    } else if let Some(svg) = &row.mermaid {
        let image = Image::from_bytes(ImageFormat::Svg, svg.as_bytes().to_vec());
        element = element.child(
            img(Arc::new(image))
                .max_w(px(ROW_WIDTH - text_left - inline - 24.0))
                .max_h(px(height)),
        );
    } else {
        let (text, runs) = visible_runs(row, theme);
        element = element.child(StyledText::new(text).with_runs(runs));
    }

    if let Some((x, caret_font)) = caret_position(view, row, window, theme) {
        element = element.child(
            div()
                .absolute()
                .left(px(text_left + inline + f32::from(x)))
                .top(px(4.0))
                .w(px(2.0))
                .h(px(f32::from(caret_font) + 6.0))
                .bg(theme.colors.caret),
        );
    }

    element.into_any_element()
}

fn marker_badge(badge: &stratum_editor::MarkerBadge, theme: &Theme) -> impl IntoElement {
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

fn span_style(span: &DecoratedSpan, row: &DecoratedRow, theme: &Theme) -> SpanStyle {
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
            // Asset references render as a distinct chip with the path text;
            // thumbnails and attachment controls stay in the React panel until
            // phase B adds them.
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

fn floor_char_boundary(text: &str, mut index: usize) -> usize {
    index = index.min(text.len());
    while index > 0 && !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

#[cfg(test)]
mod tests;
