//! Gallery lifecycle actions for the native Whiteboards panel.
//!
//! These methods own board creation, rename, multi-select deletion and the
//! gallery/editor transition. Document behaviour stays in [`super::model`].

use std::sync::Arc;
use std::time::Instant;

use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_canvas::{PreviewTheme, Tool};
use stratum_i18n::t_global;

use super::{CanvasView, GalleryModel, WhiteboardMode};

impl CanvasView {
    /// Reload the gallery from the provider and regenerate missing previews.
    pub fn reload_gallery(&mut self, cx: &mut Context<Self>) {
        match GalleryModel::load(self.provider.as_ref()) {
            Ok(mut gallery) => {
                gallery.generate_previews(self.provider.as_ref());
                self.gallery = gallery;
                self.error = None;
            }
            Err(error) => {
                self.gallery = GalleryModel::default();
                self.error = Some(error);
            }
        }
        cx.notify();
    }

    /// Create a board from the name field.
    pub fn create_board(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = self.create_value.trim().to_string();
        if name.is_empty() {
            return;
        }
        match self.gallery.create(self.provider.as_ref(), &name) {
            Ok(()) => {
                self.create_value.clear();
                self.create_input.update(cx, |input, cx| {
                    input.set_value(String::new(), window, cx);
                });
                self.error = None;
                cx.notify();
            }
            Err(error) => {
                self.error = Some(format!(
                    "{} {}",
                    t_global("whiteboard.create_failed"),
                    error
                ));
                cx.notify();
            }
        }
    }

    /// Start renaming a gallery row.
    pub fn begin_rename(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.rename_target = Some(name.to_string());
        self.rename_value = name.to_string();
        let rename = self.rename_input.clone();
        let name = name.to_string();
        rename.update(cx, |input, cx| input.set_value(name, window, cx));
        cx.notify();
    }

    /// Cancel the rename form.
    pub fn cancel_rename(&mut self, cx: &mut Context<Self>) {
        self.rename_target = None;
        self.rename_value.clear();
        cx.notify();
    }

    /// Apply the rename form.
    pub fn confirm_rename(&mut self, cx: &mut Context<Self>) {
        let Some(old_name) = self.rename_target.clone() else {
            return;
        };
        let new_name = self.rename_value.trim().to_string();
        if new_name.is_empty() {
            return;
        }
        match self
            .gallery
            .rename(self.provider.as_ref(), &old_name, &new_name)
        {
            Ok(()) => {
                self.rename_target = None;
                self.rename_value.clear();
                self.error = None;
            }
            Err(error) => {
                self.error = Some(format!(
                    "{} {}",
                    t_global("whiteboard.failed_to_rename_whiteboard"),
                    error
                ));
            }
        }
        cx.notify();
    }

    /// Toggle a gallery row's multi-select state.
    pub fn toggle_row(&mut self, name: &str, cx: &mut Context<Self>) {
        self.gallery.toggle_selection(name);
        cx.notify();
    }

    /// Delete the selected gallery rows.
    pub fn delete_selected(&mut self, cx: &mut Context<Self>) {
        match self.gallery.delete_selected(self.provider.as_ref()) {
            Ok(0) => {}
            Ok(_) => {
                self.error = None;
                cx.notify();
            }
            Err(error) => {
                self.error = Some(format!(
                    "{} {}",
                    t_global("whiteboard.failed_to_delete_whiteboard"),
                    error
                ));
                cx.notify();
            }
        }
    }

    /// Open a board in the editor.
    pub fn open_board(
        &mut self,
        name: &str,
        tool: Option<Tool>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match GalleryModel::open(self.provider.as_ref(), name, PreviewTheme::Light) {
            Ok(mut session) => {
                if let Some(tool) = tool {
                    session.state_mut().set_tool(tool);
                }
                self.session = Some(session);
                self.mode = WhiteboardMode::Editor;
                self.error = None;
                self.message = None;
                self.rebuild_frame(cx);
                if let Some(element) = self.start_element.clone() {
                    self.focus_element(&element, cx);
                }
                if let Some(format) = self.start_export.take() {
                    self.export_board(format, cx);
                }
                window.focus(&self.focus, cx);
                cx.notify();
            }
            Err(error) => {
                self.mode = WhiteboardMode::Gallery;
                self.session = None;
                self.frame = None;
                self.frame_image = None;
                self.error = Some(format!(
                    "{} {}",
                    t_global("whiteboard.board_not_found"),
                    error
                ));
                cx.notify();
            }
        }
    }

    /// Flush pending edits and return to the gallery.
    pub fn back_to_gallery(&mut self, cx: &mut Context<Self>) {
        if let Some(session) = self.session.as_mut() {
            let provider = Arc::clone(&self.provider);
            match session.flush(provider.as_ref(), Instant::now()) {
                Ok(_) => {}
                Err(error) => {
                    self.error = Some(format!(
                        "{} {}",
                        t_global("whiteboard.failed_to_load_whiteboard"),
                        error
                    ));
                    cx.notify();
                    return;
                }
            }
        }
        self.session = None;
        self.frame = None;
        self.frame_image = None;
        self.mode = WhiteboardMode::Gallery;
        self.start_element = None;
        self.reload_gallery(cx);
    }
}
