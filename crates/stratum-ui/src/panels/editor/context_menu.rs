//! Per-block context menu for the native editor.
//!
//! Right-clicking a row places the caret in that block and opens a popup
//! anchored at the pointer. The menu mirrors the shipped React block menu: read
//! aloud, copy, cut, toggle the task marker and set/clear a reminder. Every
//! entry resolves its label through [`stratum_i18n::t_global`]; the label keys
//! are release-gated by [`CONTEXT_MENU_ITEMS`].
//!
//! The menu only mutates the engine through [`stratum_editor::EditorState`]:
//! clipboard, marker and reminder operations stay headless and unit-tested. A
//! selected entry always closes the menu, marks the document dirty and schedules
//! the normal debounced save. Block drag-and-drop reordering lives in the
//! editor's [`crate::panels::editor::drag`] module; the context menu still owns
//! its own actions.

use chrono::{Duration, Utc};
use gpui_kit::component::ActiveTheme;
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_editor::MARKER_OPTIONS;
use stratum_i18n::t_global;
use uuid::Uuid;

use crate::panels::editor::rows::visible_text;
use crate::panels::editor::EditorView;

/// Width of the context-menu popup in pixels.
pub const MENU_WIDTH: f32 = 220.0;
/// Height of one menu entry, used to clamp the popup inside the viewport.
pub const MENU_ITEM_HEIGHT: f32 = 28.0;
/// Inner padding of the popup.
pub const MENU_PADDING: f32 = 8.0;

/// One action offered by the block context menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextAction {
    /// Speak the visible block text aloud through the provider seam.
    ReadAloud,
    /// Copy the block selection (or the whole block) to the clipboard.
    Copy,
    /// Cut the block selection (or the whole block) to the clipboard.
    Cut,
    /// Toggle a TODO marker on the block.
    ToggleMarker,
    /// Set the block's `due` reminder one hour from now.
    SetReminder,
    /// Remove the block's `due` reminder.
    ClearReminder,
}

/// A context-menu entry: an action plus its translation key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContextMenuItem {
    /// Action run when the entry is selected.
    pub action: ContextAction,
    /// Translation key for the entry label.
    pub label_key: &'static str,
}

/// The context-menu entries in display order.
pub const CONTEXT_MENU_ITEMS: &[ContextMenuItem] = &[
    ContextMenuItem {
        action: ContextAction::ReadAloud,
        label_key: "editor.read_aloud",
    },
    ContextMenuItem {
        action: ContextAction::Copy,
        label_key: "editor.copy",
    },
    ContextMenuItem {
        action: ContextAction::Cut,
        label_key: "editor.cut",
    },
    ContextMenuItem {
        action: ContextAction::ToggleMarker,
        label_key: "editor.task_marker",
    },
    ContextMenuItem {
        action: ContextAction::SetReminder,
        label_key: "reminders.chip.title",
    },
    ContextMenuItem {
        action: ContextAction::ClearReminder,
        label_key: "reminders.chip.clear",
    },
];

/// An open context menu, anchored relative to the editor list viewport.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContextMenuState {
    /// Block the menu was opened on (`pkm_block::BlockId`).
    pub block_id: Uuid,
    /// Pointer x offset inside the list viewport.
    pub x: f32,
    /// Pointer y offset inside the list viewport.
    pub y: f32,
}

/// Render the open context menu, if any.
///
/// The popup is placed in the list viewport coordinate space (the same space
/// [`ContextMenuState`] stores), clamped so it stays inside the viewport. The
/// popup swallows mouse-down events so clicking an entry cannot also place the
/// caret on the row behind it.
pub fn context_menu_popup(view: &EditorView, cx: &mut Context<EditorView>) -> AnyElement {
    let Some(menu) = view.context_menu else {
        return div().into_any_element();
    };
    let theme = cx.theme();
    let bounds = view.scroll.bounds();
    let viewport_width = f32::from(bounds.size.width);
    let viewport_height = f32::from(bounds.size.height);
    let menu_height = CONTEXT_MENU_ITEMS.len() as f32 * MENU_ITEM_HEIGHT + MENU_PADDING;
    let x = menu.x.min((viewport_width - MENU_WIDTH).max(8.0)).max(8.0);
    let y = menu
        .y
        .min((viewport_height - menu_height).max(8.0))
        .max(8.0);

    let mut list = div().flex().flex_col().gap(px(1.0));
    for (index, item) in CONTEXT_MENU_ITEMS.iter().enumerate() {
        let action = item.action;
        let label = t_global(item.label_key);
        list = list.child(
            div()
                .id(("context-menu-item", index))
                .flex()
                .items_center()
                .px(px(10.0))
                .py(px(6.0))
                .rounded(px(4.0))
                .text_size(px(12.0))
                .text_color(theme.colors.foreground)
                .cursor_pointer()
                .hover(|style| style.bg(theme.colors.secondary))
                .on_click(cx.listener(move |this, _, _, cx| {
                    cx.stop_propagation();
                    apply_context_action(this, action, cx);
                }))
                .child(label),
        );
    }

    div()
        .absolute()
        .left(px(x))
        .top(px(y))
        .w(px(MENU_WIDTH))
        .rounded(px(8.0))
        .bg(theme.colors.background)
        .border_1()
        .border_color(theme.colors.border)
        .shadow_md()
        .p(px(4.0))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|_, _, _, cx| cx.stop_propagation()),
        )
        .on_mouse_down(
            MouseButton::Right,
            cx.listener(|_, _, _, cx| cx.stop_propagation()),
        )
        .child(list)
        .into_any_element()
}

/// Close the menu and run `action` on the block it was opened for.
///
/// Document-changing entries mark the document dirty and schedule the debounced
/// save through [`EditorView::after_edit`]. Read-aloud only speaks: a synthesis
/// error is surfaced through the save chip, while a playback error is logged by
/// the provider.
pub fn apply_context_action(
    view: &mut EditorView,
    action: ContextAction,
    cx: &mut Context<EditorView>,
) {
    let Some(menu) = view.context_menu else {
        return;
    };
    view.context_menu = None;
    match run_action(view, action, menu.block_id, cx) {
        Ok(()) => view.after_edit(cx),
        Err(error) => {
            view.save.fail(error);
            cx.notify();
        }
    }
}

/// Run one menu action, returning a user-visible error message on failure.
fn run_action(
    view: &mut EditorView,
    action: ContextAction,
    block_id: Uuid,
    cx: &mut Context<EditorView>,
) -> Result<(), String> {
    match action {
        ContextAction::ReadAloud => {
            let text = block_visible_text(view, block_id);
            if text.trim().is_empty() {
                return Ok(());
            }
            view.provider.speak(text)
        }
        ContextAction::Copy => {
            select_whole_block(view, block_id);
            let payload = view.state.copy_selection();
            if !payload.plain_text.is_empty() {
                cx.write_to_clipboard(ClipboardItem::new_string(payload.plain_text));
            }
            Ok(())
        }
        ContextAction::Cut => {
            select_whole_block(view, block_id);
            let payload = view
                .state
                .cut_selection()
                .map_err(|error| error.to_string())?;
            if !payload.plain_text.is_empty() {
                cx.write_to_clipboard(ClipboardItem::new_string(payload.plain_text));
            }
            Ok(())
        }
        ContextAction::ToggleMarker => {
            focus_block(view, block_id);
            let has_marker = view
                .state
                .document()
                .block(block_id)
                .map(|block| block.marker.is_some())
                .unwrap_or(false);
            let result = if has_marker {
                view.state.clear_marker()
            } else {
                view.state.set_marker(Some(MARKER_OPTIONS[0].marker))
            };
            result.map_err(|error| error.to_string())
        }
        ContextAction::SetReminder => {
            let due = (Utc::now() + Duration::hours(1)).to_rfc3339();
            view.state
                .set_block_reminder(block_id, Some(&due))
                .map_err(|error| error.to_string())
        }
        ContextAction::ClearReminder => view
            .state
            .set_block_reminder(block_id, None)
            .map_err(|error| error.to_string()),
    }
}

/// The visible (delimiter-stripped) text of a decorated row.
fn block_visible_text(view: &EditorView, block_id: Uuid) -> String {
    view.rows
        .iter()
        .find(|row| row.block_id == block_id)
        .map(visible_text)
        .unwrap_or_default()
}

/// Select the whole block when no selection is active.
fn select_whole_block(view: &mut EditorView, block_id: Uuid) {
    if view.state.selection().is_some() {
        return;
    }
    let Some(content) = view
        .state
        .document()
        .block(block_id)
        .map(|block| block.content.clone())
    else {
        return;
    };
    let anchor = stratum_editor::Caret {
        block_id,
        byte_offset: 0,
    };
    let head = stratum_editor::Caret {
        block_id,
        byte_offset: content.len(),
    };
    let _ = view.state.set_selection(anchor, head);
}

/// Put the caret back into the menu's block before a caret-driven operation.
fn focus_block(view: &mut EditorView, block_id: Uuid) {
    let offset = view
        .state
        .caret()
        .filter(|caret| caret.block_id == block_id)
        .map(|caret| caret.byte_offset)
        .unwrap_or(0);
    let _ = view.state.set_caret(block_id, offset);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::core::prelude::v1::test;
    use stratum_i18n::{t, Locale};

    #[test]
    fn context_menu_entries_use_catalogued_keys() {
        assert!(!CONTEXT_MENU_ITEMS.is_empty());
        for item in CONTEXT_MENU_ITEMS {
            for locale in Locale::ALL {
                assert_ne!(
                    t(locale, item.label_key),
                    item.label_key,
                    "{} missing in {}",
                    item.label_key,
                    locale.code()
                );
            }
        }
    }
}
