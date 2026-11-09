//! Structural key handling for the native block editor.
//!
//! Key-down events only carry structural keys; text arrives through the input
//! handler. Control-key chords cover undo/redo, clipboard, selection and marker
//! toggling. Caret movement helpers keep the overlay in sync.

use gpui_kit::*;

use crate::panels::editor::overlay::OverlayState;
use crate::panels::editor::EditorView;

use super::overlay_ops::{apply_overlay, overlay_len, refresh_overlay};
use super::{finish_edit, next_char_boundary, prev_char_boundary};

/// Handle one key-down event. Returns whether the editor consumed it.
pub fn handle_key_down(
    view: &mut EditorView,
    event: &KeyDownEvent,
    window: &mut Window,
    cx: &mut Context<EditorView>,
) -> bool {
    if view.read_mode {
        return false;
    }
    let keystroke = &event.keystroke;
    let modifiers = keystroke.modifiers;

    if keystroke.key == "escape" && view.context_menu.is_some() {
        view.context_menu = None;
        cx.notify();
        return true;
    }

    if modifiers.control {
        return handle_control_key(view, keystroke, cx);
    }

    match keystroke.key.as_str() {
        "escape" => {
            if view.overlay != OverlayState::None {
                view.overlay = OverlayState::None;
                cx.notify();
                return true;
            }
            false
        }
        "up" => {
            if view.overlay.selected().is_some() {
                let len = overlay_len(view);
                view.overlay.move_selection(-1, len);
                cx.notify();
                return true;
            }
            move_caret_row(view, -1, modifiers.shift, cx);
            true
        }
        "down" => {
            if view.overlay.selected().is_some() {
                let len = overlay_len(view);
                view.overlay.move_selection(1, len);
                cx.notify();
                return true;
            }
            move_caret_row(view, 1, modifiers.shift, cx);
            true
        }
        "left" => {
            move_caret_char(view, -1, modifiers.shift, cx);
            true
        }
        "right" => {
            move_caret_char(view, 1, modifiers.shift, cx);
            true
        }
        "home" => {
            set_caret_boundary(view, false, modifiers.shift, cx);
            true
        }
        "end" => {
            set_caret_boundary(view, true, modifiers.shift, cx);
            true
        }
        "backspace" => {
            let result = view.state.backspace();
            finish_edit(view, result, cx);
            true
        }
        "delete" => {
            let result = view.state.delete_forward();
            finish_edit(view, result, cx);
            true
        }
        "enter" => {
            if view.overlay.selected().is_some() {
                apply_overlay(view, window, cx);
            } else {
                let result = view.state.insert_newline();
                finish_edit(view, result, cx);
            }
            true
        }
        "tab" => {
            let result = if modifiers.shift {
                view.state.outdent()
            } else {
                view.state.indent()
            };
            finish_edit(view, result, cx);
            true
        }
        _ => false,
    }
}

fn handle_control_key(
    view: &mut EditorView,
    keystroke: &Keystroke,
    cx: &mut Context<EditorView>,
) -> bool {
    match keystroke.key.as_str() {
        "enter" => {
            let marker = if view.state.caret().is_some() {
                view.state
                    .set_marker(Some(stratum_editor::MARKER_OPTIONS[0].marker))
            } else {
                Ok(())
            };
            finish_edit(view, marker, cx);
            true
        }
        "z" if keystroke.modifiers.shift => {
            let result = view.state.redo().map(|_| ());
            finish_edit(view, result, cx);
            true
        }
        "z" => {
            let result = view.state.undo().map(|_| ());
            finish_edit(view, result, cx);
            true
        }
        "c" => {
            let payload = view.state.copy_selection();
            if !payload.plain_text.is_empty() {
                cx.write_to_clipboard(ClipboardItem::new_string(payload.plain_text));
            }
            cx.notify();
            true
        }
        "x" => {
            let result = view.state.cut_selection();
            match result {
                Ok(payload) => {
                    if !payload.plain_text.is_empty() {
                        cx.write_to_clipboard(ClipboardItem::new_string(payload.plain_text));
                    }
                    view.after_edit(cx);
                }
                Err(error) => view.save.fail(error.to_string()),
            }
            cx.notify();
            true
        }
        "v" => {
            let text = cx
                .read_from_clipboard()
                .and_then(|item| item.text())
                .unwrap_or_default();
            if !text.is_empty() {
                let payload = stratum_editor::normalize_paste(&text);
                let result = view.state.insert_text(&payload);
                finish_edit(view, result, cx);
            }
            true
        }
        "a" => {
            if let Some(caret) = view.state.caret() {
                let content = view
                    .state
                    .document()
                    .block(caret.block_id)
                    .map(|block| block.content.clone())
                    .unwrap_or_default();
                let anchor = stratum_editor::Caret {
                    block_id: caret.block_id,
                    byte_offset: 0,
                };
                let head = stratum_editor::Caret {
                    block_id: caret.block_id,
                    byte_offset: content.len(),
                };
                let _ = view.state.set_selection(anchor, head);
                cx.notify();
            }
            true
        }
        _ => false,
    }
}

fn move_caret_row(view: &mut EditorView, delta: isize, extend: bool, cx: &mut Context<EditorView>) {
    let Some(caret) = view.state.caret() else {
        return;
    };
    let ids = view.state.document().ordered_ids();
    let Some(index) = ids.iter().position(|id| *id == caret.block_id) else {
        return;
    };
    let next = index as isize + delta;
    if next < 0 || next >= ids.len() as isize {
        return;
    }
    let target = ids[next as usize];
    let offset = view
        .state
        .document()
        .block(target)
        .map(|block| caret.byte_offset.min(block.content.len()))
        .unwrap_or(0);
    let anchor = view
        .state
        .selection()
        .map(|selection| selection.anchor)
        .unwrap_or(caret);
    if extend {
        let _ = view.state.set_selection(
            anchor,
            stratum_editor::Caret {
                block_id: target,
                byte_offset: offset,
            },
        );
    } else {
        let _ = view.state.set_caret(target, offset);
    }
    refresh_overlay(view);
    cx.notify();
}

fn move_caret_char(
    view: &mut EditorView,
    delta: isize,
    extend: bool,
    cx: &mut Context<EditorView>,
) {
    let Some(caret) = view.state.caret() else {
        return;
    };
    let content = view
        .state
        .document()
        .block(caret.block_id)
        .map(|block| block.content.clone())
        .unwrap_or_default();
    let offset = if delta < 0 {
        prev_char_boundary(&content, caret.byte_offset)
    } else {
        next_char_boundary(&content, caret.byte_offset)
    };
    let anchor = view
        .state
        .selection()
        .map(|selection| selection.anchor)
        .unwrap_or(caret);
    let head = stratum_editor::Caret {
        block_id: caret.block_id,
        byte_offset: offset,
    };
    if extend {
        let _ = view.state.set_selection(anchor, head);
    } else {
        let _ = view.state.set_caret(head.block_id, head.byte_offset);
    }
    refresh_overlay(view);
    cx.notify();
}

fn set_caret_boundary(
    view: &mut EditorView,
    end: bool,
    extend: bool,
    cx: &mut Context<EditorView>,
) {
    let Some(caret) = view.state.caret() else {
        return;
    };
    let content = view
        .state
        .document()
        .block(caret.block_id)
        .map(|block| block.content.clone())
        .unwrap_or_default();
    let offset = if end { content.len() } else { 0 };
    let anchor = view
        .state
        .selection()
        .map(|selection| selection.anchor)
        .unwrap_or(caret);
    let head = stratum_editor::Caret {
        block_id: caret.block_id,
        byte_offset: offset,
    };
    if extend {
        let _ = view.state.set_selection(anchor, head);
    } else {
        let _ = view.state.set_caret(head.block_id, head.byte_offset);
    }
    refresh_overlay(view);
    cx.notify();
}
