//! Pointer handling for the native block editor.
//!
//! Mouse hits are translated into caret placements and Ctrl+hover link
//! previews; Ctrl+click on a tag or wiki-link records a navigation request for
//! the shell. Right-click places the caret and opens the per-block context menu
//! anchored at the pointer.

use gpui_kit::component::ActiveTheme;
use gpui_kit::*;
use stratum_editor::menus::{link_at, LinkHitKind};

use crate::panels::editor::context_menu::ContextMenuState;
use crate::panels::editor::overlay::OverlayState;
use crate::panels::editor::rows::{
    inline_offset, marker_prefix_len, row_font_size, row_index_at_point, row_text_left,
    source_offset_for_visible, visible_runs,
};
use crate::panels::editor::EditorView;

use super::refresh_overlay;

/// Handle one mouse-down event, placing the caret under the pointer.
///
/// A left press anywhere in the editor also dismisses an open context menu;
/// clicks inside the popup itself stop propagation before reaching this handler.
pub fn handle_mouse_down(
    view: &mut EditorView,
    event: &MouseDownEvent,
    window: &mut Window,
    cx: &mut Context<EditorView>,
) {
    window.focus(&view.focus, cx);
    if view.read_mode {
        cx.notify();
        return;
    }
    view.context_menu = None;
    let _ = place_caret(view, event.position, window, cx);
    cx.notify();
}

/// Handle one right-click: place the caret and open the block context menu.
pub fn handle_right_mouse_down(
    view: &mut EditorView,
    event: &MouseDownEvent,
    window: &mut Window,
    cx: &mut Context<EditorView>,
) {
    window.focus(&view.focus, cx);
    if view.read_mode {
        cx.notify();
        return;
    }
    let Some(index) = place_caret(view, event.position, window, cx) else {
        cx.notify();
        return;
    };
    let Some(row) = view.rows.get(index) else {
        return;
    };
    let viewport = view.scroll.bounds();
    let x = f32::from(event.position.x) - f32::from(viewport.origin.x);
    let y = f32::from(event.position.y) - f32::from(viewport.origin.y);
    view.context_menu = Some(ContextMenuState {
        block_id: row.block_id,
        x,
        y,
    });
    view.overlay = OverlayState::None;
    cx.notify();
}

/// Place the caret at a window-space point, returning the hit row index.
fn place_caret(
    view: &mut EditorView,
    position: Point<Pixels>,
    window: &mut Window,
    cx: &mut Context<EditorView>,
) -> Option<usize> {
    let index = row_index_at_point(view, position)?;
    let row = view.rows.get(index).cloned()?;
    let content = view
        .state
        .document()
        .block(row.block_id)
        .map(|block| block.content.clone())?;
    let theme = cx.theme();
    let (text, runs) = visible_runs(&row, theme);
    let font_size = row_font_size(&row, theme);
    let viewport = view.scroll.bounds();
    let text_x = f32::from(position.x)
        - f32::from(viewport.origin.x)
        - row_text_left(row.depth)
        - inline_offset(view, &row);
    let layout = window
        .text_system()
        .layout_line(&text, font_size, &runs, None);
    let visible_byte = layout
        .index_for_x(px(text_x.max(0.0)))
        .unwrap_or(text.len());
    let prefix = marker_prefix_len(&content, row.marker.map(|marker| marker.as_str()));
    let source = source_offset_for_visible(&row, visible_byte) + prefix;
    let _ = view.state.set_caret(row.block_id, source);
    refresh_overlay(view);
    Some(index)
}

/// Handle a mouse move, showing the Ctrl+hover link preview.
pub fn handle_mouse_move(
    view: &mut EditorView,
    event: &MouseMoveEvent,
    window: &mut Window,
    cx: &mut Context<EditorView>,
) {
    if !event.modifiers.control {
        if view.hover_preview.take().is_some() {
            cx.notify();
        }
        return;
    }
    let Some(index) = row_index_at_point(view, event.position) else {
        if view.hover_preview.take().is_some() {
            cx.notify();
        }
        return;
    };
    let Some(row) = view.rows.get(index).cloned() else {
        return;
    };
    let theme = cx.theme();
    let (text, runs) = visible_runs(&row, theme);
    let font_size = row_font_size(&row, theme);
    let viewport = view.scroll.bounds();
    let text_x = f32::from(event.position.x)
        - f32::from(viewport.origin.x)
        - row_text_left(row.depth)
        - inline_offset(view, &row);
    let layout = window
        .text_system()
        .layout_line(&text, font_size, &runs, None);
    let visible_byte = layout
        .index_for_x(px(text_x.max(0.0)))
        .unwrap_or(text.len());
    let Some(content) = view
        .state
        .document()
        .block(row.block_id)
        .map(|block| block.content.clone())
    else {
        return;
    };
    let prefix = marker_prefix_len(&content, row.marker.map(|marker| marker.as_str()));
    let source = source_offset_for_visible(&row, visible_byte) + prefix;
    let target = link_at(view.state.document(), row.block_id, source).map(|hit| hit.target);
    if target != view.hover_preview {
        view.hover_preview = target;
        cx.notify();
    }
}

/// Handle a click. Ctrl+click on a tag records a navigation request for the
/// shell; Ctrl+click on a wiki-link previews its target.
pub fn handle_click(
    view: &mut EditorView,
    event: &ClickEvent,
    _window: &mut Window,
    cx: &mut Context<EditorView>,
) {
    if !event.modifiers().control {
        return;
    }
    let Some(position) = event.mouse_position() else {
        return;
    };
    let Some(index) = row_index_at_point(view, position) else {
        return;
    };
    let Some(row) = view.rows.get(index).cloned() else {
        return;
    };
    let theme = cx.theme();
    let (text, runs) = visible_runs(&row, theme);
    let font_size = row_font_size(&row, theme);
    let viewport = view.scroll.bounds();
    let text_x = f32::from(position.x)
        - f32::from(viewport.origin.x)
        - row_text_left(row.depth)
        - inline_offset(view, &row);
    let layout = _window
        .text_system()
        .layout_line(&text, font_size, &runs, None);
    let visible_byte = layout
        .index_for_x(px(text_x.max(0.0)))
        .unwrap_or(text.len());
    let Some(content) = view
        .state
        .document()
        .block(row.block_id)
        .map(|block| block.content.clone())
    else {
        return;
    };
    let prefix = marker_prefix_len(&content, row.marker.map(|marker| marker.as_str()));
    let source = source_offset_for_visible(&row, visible_byte) + prefix;
    if let Some(hit) = link_at(view.state.document(), row.block_id, source) {
        if matches!(hit.kind, LinkHitKind::Tag | LinkHitKind::WikiLink) {
            view.pending_navigation = Some(hit.target);
            cx.refresh_windows();
        }
    }
}
