//! Input wiring for the native block editor.
//!
//! The platform delivers text through [`EntityInputHandler`] (IME, dead keys and
//! normal typing all arrive as `replace_text_in_range`), while key-down events
//! only carry structural keys. Every edit is delegated to [`stratum_editor`]:
//! this module maps UTF-16 platform offsets onto the engine's UTF-8 byte
//! offsets, applies slash/wiki/marker completions and translates mouse hits into
//! caret placements and Ctrl+hover link previews.
//!
//! Text characters are deliberately left to propagate: the platform inserts
//! them through the input handler after the key listener returns. Structural
//! keys stop propagation so they cannot also be inserted as text.
//!
//! The module is split so each file stays under the repository limit:
//!
//! - [`keys`] — structural key handling, caret movement and clipboard actions.
//! - [`mouse`] — pointer hit testing, caret placement, Ctrl+hover previews and
//!   the right-click context menu.
//! - [`overlay_ops`] — slash/wiki/marker completion application.
//! - [`ime`] — the [`EntityInputHandler`] implementation.
//!
//! The caret helpers shared by those files live here.

use gpui_kit::component::ActiveTheme;
use gpui_kit::*;

use crate::panels::editor::rows::{
    inline_offset, marker_prefix_len, row_font_size, row_text_left, visible_prefix,
};
use crate::panels::editor::EditorView;

mod ime;
mod keys;
mod mouse;
pub(crate) mod overlay_ops;

pub use keys::handle_key_down;
pub use mouse::{handle_click, handle_mouse_down, handle_mouse_move, handle_right_mouse_down};

pub(crate) use overlay_ops::refresh_overlay;

#[cfg(test)]
mod tests;

/// Apply an edit result, refreshing overlays and save state on success.
pub(crate) fn finish_edit(
    view: &mut EditorView,
    result: pkm_core::PkmResult<()>,
    cx: &mut Context<EditorView>,
) {
    match result {
        Ok(()) => {
            refresh_overlay(view);
            view.after_edit(cx);
        }
        Err(error) => {
            view.save.fail(error.to_string());
            cx.notify();
        }
    }
}

/// Step one character back from `offset`, clamping to a char boundary.
pub(crate) fn prev_char_boundary(text: &str, offset: usize) -> usize {
    let len = text.len();
    let offset = offset.min(len);
    if offset == 0 {
        return 0;
    }
    // A platform caret offset can land inside a multi-byte character; floor it
    // to that character's start instead of slicing on a non-boundary.
    let mut boundary = offset;
    while boundary > 0 && !text.is_char_boundary(boundary) {
        boundary -= 1;
    }
    if boundary < offset {
        return boundary;
    }
    text[..boundary]
        .char_indices()
        .next_back()
        .map(|(index, _)| index)
        .unwrap_or(0)
}

/// Step one character forward from `offset`, clamping to a char boundary.
pub(crate) fn next_char_boundary(text: &str, offset: usize) -> usize {
    let len = text.len();
    let offset = offset.min(len);
    // Ceil an interior offset to the next character boundary before advancing.
    let mut boundary = offset;
    while boundary < len && !text.is_char_boundary(boundary) {
        boundary += 1;
    }
    if boundary > offset {
        return boundary;
    }
    text[boundary..]
        .chars()
        .next()
        .map(|character| boundary + character.len_utf8())
        .unwrap_or(len)
}

/// Bounds and font size of the text before the caret, for IME placement.
pub(crate) fn caret_text_prefix(
    view: &EditorView,
    window: &mut Window,
    cx: &mut Context<EditorView>,
) -> Option<(Bounds<Pixels>, Pixels)> {
    let caret = view.state.caret()?;
    let row_index = view
        .state
        .document()
        .ordered_ids()
        .iter()
        .position(|id| *id == caret.block_id)?;
    let row = view.rows.get(row_index)?.clone();
    let content = view.state.document().block(row.block_id)?.content.clone();
    let prefix_len = marker_prefix_len(&content, row.marker.map(|marker| marker.as_str()));
    let source = caret.byte_offset.saturating_sub(prefix_len);
    let theme = cx.theme();
    let (text, runs) = visible_prefix(&row, theme, source);
    let font_size = row_font_size(&row, theme);
    let layout = window
        .text_system()
        .layout_line(&text, font_size, &runs, None);
    let x = layout.x_for_index(text.len());
    let viewport = view.scroll.bounds();
    let top = view.layout.offsets().get(row_index).copied().unwrap_or(0.0);
    let y = viewport.origin.y + px(top) - view.scroll.offset().y;
    let origin = point(
        viewport.origin.x + px(row_text_left(row.depth) + inline_offset(view, &row)) + x,
        y,
    );
    Some((
        Bounds {
            origin,
            size: size(px(2.0), px(f32::from(font_size) + 6.0)),
        },
        font_size,
    ))
}
