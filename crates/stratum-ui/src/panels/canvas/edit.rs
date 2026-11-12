//! Editing, clipboard, library, export and viewport actions for the whiteboard
//! editor. Every document mutation goes through [`super::model::EditorSession`].

use std::sync::Arc;
use std::time::Instant;

use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_canvas::{Align, Axis, CanvasAction, StylePatch, Tool};
use stratum_i18n::t_global;

use crate::provider::panels::LibraryItemRow;

use super::{model, CanvasView, ExportFormat};

impl CanvasView {
    /// Select a tool. An active inline text edit is committed first, matching
    /// the canvas state's own `set_tool` contract.
    pub fn set_tool(&mut self, tool: Tool, cx: &mut Context<Self>) {
        let editing = self
            .session
            .as_ref()
            .is_some_and(|session| session.state().text_editing().is_some());
        if editing {
            self.commit_text_edit(None, cx);
            // The tool buttons are not focusable, so the inline field would
            // keep keyboard focus after its overlay disappears.
            self.focus_canvas_later(cx);
        }
        if let Some(session) = self.session.as_mut() {
            session.state_mut().set_tool(tool);
        }
        self.rebuild_frame(cx);
        cx.notify();
    }

    /// Stream the inline text field into the active canvas text edit.
    pub(crate) fn update_text_edit(&mut self, value: &str, cx: &mut Context<Self>) {
        let Some(session) = self.session.as_mut() else {
            return;
        };
        if session.state().text_editing().is_none() {
            return;
        }
        let action = session.state_mut().update_text(value);
        if matches!(action, CanvasAction::TextEditing(_)) {
            session.note_edit(Instant::now());
        }
        self.rebuild_frame(cx);
    }

    /// Seed and focus the inline text field when a canvas text edit begins.
    pub(crate) fn begin_text_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let value = self
            .session
            .as_ref()
            .and_then(|session| session.state().text_editing())
            .unwrap_or_default()
            .to_string();
        let input = self.text_input.clone();
        input.update(cx, |input, cx| {
            input.set_value(value, window, cx);
            input.focus(window, cx);
        });
        cx.notify();
    }

    /// Commit the active inline text edit and schedule autosave.
    ///
    /// `window` is available from pointer/key handlers; input subscriptions
    /// pass `None` because gpui has the window borrowed while dispatching the
    /// event, so the field clear is deferred to the end of the effect cycle.
    pub(crate) fn commit_text_edit(&mut self, window: Option<&mut Window>, cx: &mut Context<Self>) {
        let action = {
            let Some(session) = self.session.as_mut() else {
                return;
            };
            let action = session.state_mut().commit_text();
            if action == CanvasAction::DocumentChanged {
                session.note_edit(Instant::now());
            }
            action
        };
        self.clear_text_input(window, cx);
        self.rebuild_frame(cx);
        if action == CanvasAction::DocumentChanged {
            self.schedule_save(cx);
        }
        cx.notify();
    }

    /// Cancel the active inline text edit (Escape) and return focus to the
    /// canvas.
    pub(crate) fn cancel_text_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let action = {
            let Some(session) = self.session.as_mut() else {
                return;
            };
            let action = session.state_mut().cancel();
            if action == CanvasAction::DocumentChanged {
                session.note_edit(Instant::now());
            }
            action
        };
        self.clear_text_input(Some(&mut *window), cx);
        window.focus(&self.focus, cx);
        self.rebuild_frame(cx);
        if action == CanvasAction::DocumentChanged {
            self.schedule_save(cx);
        }
        cx.notify();
    }

    /// Return keyboard focus to the canvas after a committed edit.
    ///
    /// Input subscriptions run while gpui has the window borrowed, so this is
    /// deferred to the end of the effect cycle.
    pub(crate) fn focus_canvas_later(&self, cx: &mut Context<Self>) {
        let focus = self.focus.clone();
        cx.defer(move |cx| {
            let Some(handle) = cx.active_window() else {
                return;
            };
            let _ = cx.update_window(handle, move |_, window, cx| {
                window.focus(&focus, cx);
            });
        });
    }

    /// Clear the inline text field without emitting another `Change` event.
    fn clear_text_input(&mut self, window: Option<&mut Window>, cx: &mut Context<Self>) {
        let input = self.text_input.clone();
        if let Some(window) = window {
            input.update(cx, |input, cx| input.set_value(String::new(), window, cx));
            return;
        }
        // Subscriptions run while the window is borrowed out for dispatch;
        // defer the clear until the effect cycle completes.
        cx.defer(move |cx| {
            let Some(handle) = cx.active_window() else {
                return;
            };
            let _ = cx.update_window(handle, move |_, window, cx| {
                input.update(cx, |input, cx| input.set_value(String::new(), window, cx));
            });
        });
    }

    /// Undo the last document mutation.
    pub fn undo(&mut self, cx: &mut Context<Self>) {
        self.apply_edit(|session, now| session.undo(now), cx);
    }

    /// Redo the last undone mutation.
    pub fn redo(&mut self, cx: &mut Context<Self>) {
        self.apply_edit(|session, now| session.redo(now), cx);
    }

    /// Delete the selection.
    pub fn delete_selection(&mut self, cx: &mut Context<Self>) {
        self.apply_edit(|session, now| session.delete_selection(now), cx);
    }

    /// Duplicate the selection.
    pub fn duplicate_selection(&mut self, cx: &mut Context<Self>) {
        self.apply_edit(|session, now| session.duplicate_selection(now), cx);
    }

    /// Apply a style patch to the selection.
    pub fn apply_style(&mut self, patch: StylePatch, cx: &mut Context<Self>) {
        self.apply_edit(|session, now| session.set_style(&patch, now), cx);
    }

    /// Align the selection.
    pub fn align(&mut self, align: Align, cx: &mut Context<Self>) {
        self.apply_edit(|session, now| session.align(align, now), cx);
    }

    /// Distribute the selection.
    pub fn distribute(&mut self, axis: Axis, cx: &mut Context<Self>) {
        self.apply_edit(|session, now| session.distribute(axis, now), cx);
    }

    /// Copy the selection as Excalidraw clipboard JSON.
    pub fn copy_selection(&mut self, cx: &mut Context<Self>) {
        let Some(session) = self.session.as_ref() else {
            return;
        };
        let payload = session.copy_selection();
        cx.write_to_clipboard(ClipboardItem::new_string(payload));
        cx.notify();
    }

    /// Paste Excalidraw clipboard JSON into the open board.
    pub fn paste(&mut self, cx: &mut Context<Self>) {
        let Some(item) = cx.read_from_clipboard() else {
            return;
        };
        let payload = item.text().unwrap_or_default();
        if payload.is_empty() {
            return;
        }
        let Some(session) = self.session.as_mut() else {
            return;
        };
        match session.paste(&payload, Instant::now()) {
            Ok(_) => {
                self.error = None;
                self.rebuild_frame(cx);
                self.schedule_save(cx);
            }
            Err(error) => {
                self.error = Some(format!(
                    "{} {}",
                    t_global("whiteboard.failed_to_save_whiteboard"),
                    error
                ));
            }
        }
        cx.notify();
    }

    /// Copy a `/whiteboards?...` link for the selection or board.
    pub fn copy_link(&mut self, cx: &mut Context<Self>) {
        let Some(session) = self.session.as_ref() else {
            return;
        };
        let element = session
            .state()
            .selection()
            .first()
            .map(String::as_str)
            .map(str::to_string);
        let link = session.copy_link(element.as_deref());
        cx.write_to_clipboard(ClipboardItem::new_string(link));
        self.message = Some(t_global("whiteboard.link_copied"));
        self.pending_toast = self.message.clone();
        cx.notify();
    }

    /// Resolve and focus a deep-link element or frame.
    pub fn focus_element(&mut self, element: &str, cx: &mut Context<Self>) {
        let Some(session) = self.session.as_mut() else {
            return;
        };
        match session.focus_element(element) {
            Ok(_) => {
                self.error = None;
                self.rebuild_frame(cx);
            }
            Err(error) => {
                self.error = Some(format!(
                    "{} {}",
                    t_global("whiteboard.element_not_found"),
                    error
                ));
            }
        }
        cx.notify();
    }

    /// Export the open board through the provider.
    pub fn export_board(&mut self, format: ExportFormat, cx: &mut Context<Self>) {
        let Some(session) = self.session.as_mut() else {
            return;
        };
        let provider = Arc::clone(&self.provider);
        match session.export(provider.as_ref(), format) {
            Ok(path) => {
                self.message = Some(format!("{} {path}", t_global("whiteboard.exported")));
                self.pending_toast = self.message.clone();
                self.error = None;
            }
            Err(error) => {
                self.error = Some(format!(
                    "{} {}",
                    t_global("whiteboard.failed_to_export"),
                    error
                ));
            }
        }
        cx.notify();
    }

    /// Toggle the library drawer, loading its items when opening.
    pub fn toggle_library(&mut self, cx: &mut Context<Self>) {
        self.library_open = !self.library_open;
        if self.library_open {
            self.reload_library(cx);
        }
        cx.notify();
    }

    /// Reload the library rows.
    pub fn reload_library(&mut self, cx: &mut Context<Self>) {
        match self.provider.whiteboard_library() {
            Ok(items) => {
                self.library_items = items;
                self.error = None;
            }
            Err(error) => {
                self.library_items.clear();
                self.error = Some(error);
            }
        }
        cx.notify();
    }

    /// Insert a library item at the centre of the current viewport.
    pub fn insert_library_item(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(row) = self.library_items.iter().find(|row| row.id == id) else {
            return;
        };
        let Some(item) = row.to_item() else {
            return;
        };
        let Some(session) = self.session.as_mut() else {
            return;
        };
        let viewport = *session.state().viewport();
        let x = (model::FRAME_WIDTH / 2.0 - viewport.x) / viewport.zoom;
        let y = (model::FRAME_HEIGHT / 2.0 - viewport.y) / viewport.zoom;
        match session.insert_library_item(&item, x, y, Instant::now()) {
            Ok(_) => {
                self.error = None;
                self.rebuild_frame(cx);
                self.schedule_save(cx);
            }
            Err(error) => {
                self.error = Some(error);
            }
        }
        cx.notify();
    }

    /// Save the selection to the personal library.
    pub fn save_selection(&mut self, cx: &mut Context<Self>) {
        let name = self.library_value.trim().to_string();
        let name = if name.is_empty() {
            self.session
                .as_ref()
                .map(|session| format!("{} selection", session.name()))
                .unwrap_or_else(|| "Selection".to_string())
        } else {
            name
        };
        let Some(session) = self.session.as_ref() else {
            return;
        };
        let provider = Arc::clone(&self.provider);
        match session.save_selection_to_library(provider.as_ref(), &name) {
            Ok(item) => {
                self.library_items.push(LibraryItemRow::from_item(&item));
                self.message = Some(t_global("whiteboard.selection_saved_to_library"));
                self.pending_toast = self.message.clone();
                self.error = None;
            }
            Err(error) => {
                self.error = Some(error);
            }
        }
        cx.notify();
    }

    /// Toggle the style panel.
    pub fn toggle_style(&mut self, cx: &mut Context<Self>) {
        self.style_open = !self.style_open;
        cx.notify();
    }

    /// Zoom the viewport by `factor` around the frame centre.
    pub fn zoom(&mut self, factor: f64, cx: &mut Context<Self>) {
        if let Some(session) = self.session.as_mut() {
            let centre =
                stratum_canvas::Point::new(model::FRAME_WIDTH / 2.0, model::FRAME_HEIGHT / 2.0);
            session.state_mut().zoom_at(centre, factor);
        }
        self.rebuild_frame(cx);
    }

    /// Reset the viewport to the origin at 100 %.
    pub fn reset_zoom(&mut self, cx: &mut Context<Self>) {
        if let Some(session) = self.session.as_mut() {
            session
                .state_mut()
                .set_viewport(stratum_canvas::Viewport::default());
        }
        self.rebuild_frame(cx);
    }
}
