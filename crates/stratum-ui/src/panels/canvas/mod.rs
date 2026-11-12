//! Native Whiteboards panel.
//!
//! [`CanvasView`] ports the shipped desktop whiteboard surface on top of the
//! headless [`stratum_canvas`] crate: the board gallery (create, rename,
//! multi-select delete, previews and element counts), the editor with tool,
//! style, align/distribute, clipboard, library and export actions, and the
//! 800 ms autosave/flush lifecycle.
//!
//! All document behaviour lives in [`model`]; this module owns entity state and
//! routes actions through the [`crate::provider::PanelData`] seam. The gpui
//! rendering is split into [`gallery`], [`editor`], [`canvas`], [`toolbar`] and
//! [`library`] so every file stays inside the repository length limit.
//!
//! [`WHITEBOARD_I18N_KEYS`] is the release gate list for every string this
//! surface renders.

use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::component::ActiveTheme;
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_canvas::{Modifiers, Point, PointerEvent, Tool};
use stratum_i18n::t_global;

use crate::panels::PanelRequests;
use crate::provider::panels::LibraryItemRow;
use crate::provider::ShellData;

mod actions;
mod edit;
mod helpers;
mod runtime;

pub(crate) use helpers::{button, fill_patch, opacity_patch, stroke_patch};

// The drawing surface module shares the panel's name by design; the panel
// module owns the gpui view while `canvas` owns the rasterized board surface.
#[allow(clippy::module_inception)]
pub mod canvas;
pub mod editor;
pub mod gallery;
pub mod library;
pub mod model;
#[cfg(test)]
mod tests;
pub mod toolbar;

pub use model::{
    tool_from_env, CanvasStart, EditorSession, ExportFormat, GalleryModel, SaveState, ViewportFrame,
};

/// Every translation key the Whiteboards surface renders.
///
/// The contract test walks this list against all eight catalogs, so adding a
/// visible string without a catalog entry fails the build instead of leaking a
/// raw key into the UI.
pub const WHITEBOARD_I18N_KEYS: &[&str] = &[
    "mobile_nav.whiteboards",
    "whiteboard.align",
    "whiteboard.align_bottom",
    "whiteboard.align_center",
    "whiteboard.align_left",
    "whiteboard.align_middle",
    "whiteboard.align_right",
    "whiteboard.align_top",
    "whiteboard.back_to_gallery",
    "whiteboard.background_color",
    "whiteboard.board_not_found",
    "whiteboard.cancel",
    "whiteboard.copy_link",
    "whiteboard.create",
    "whiteboard.create_failed",
    "whiteboard.delete",
    "whiteboard.delete_label",
    "whiteboard.delete_quot",
    "whiteboard.delete_selection",
    "whiteboard.delete_whiteboard",
    "whiteboard.distribute_horizontal",
    "whiteboard.distribute_vertical",
    "whiteboard.duplicate_selection",
    "whiteboard.element_count",
    "whiteboard.element_not_found",
    "whiteboard.export",
    "whiteboard.export_png",
    "whiteboard.export_svg",
    "whiteboard.exported",
    "whiteboard.failed_to_delete_whiteboard",
    "whiteboard.failed_to_export",
    "whiteboard.failed_to_load_whiteboard",
    "whiteboard.failed_to_rename_whiteboard",
    "whiteboard.fill_style",
    "whiteboard.insert",
    "whiteboard.library",
    "whiteboard.library_empty",
    "whiteboard.library_item",
    "whiteboard.link_copied",
    "whiteboard.new",
    "whiteboard.new_name",
    "whiteboard.new_whiteboard",
    "whiteboard.no_whiteboards_yet_create_one_to_get_started",
    "whiteboard.open_board",
    "whiteboard.opacity",
    "whiteboard.rename",
    "whiteboard.rename_whiteboard",
    "whiteboard.rename_whiteboard_label",
    "whiteboard.save_selection_to_library",
    "whiteboard.saved",
    "whiteboard.select_board_to_open",
    "whiteboard.selected_count",
    "whiteboard.selection_saved_to_library",
    "whiteboard.stroke_color",
    "whiteboard.stroke_width",
    "whiteboard.style",
    "whiteboard.this_cannot_be_undone",
    "whiteboard.tool_arrow",
    "whiteboard.tool_diamond",
    "whiteboard.tool_draw",
    "whiteboard.tool_ellipse",
    "whiteboard.tool_eraser",
    "whiteboard.tool_frame",
    "whiteboard.tool_hand",
    "whiteboard.tool_label",
    "whiteboard.tool_line",
    "whiteboard.tool_rectangle",
    "whiteboard.tool_select",
    "whiteboard.tool_text",
    "whiteboard.undo",
    "whiteboard.unsaved",
    "whiteboard.whiteboard_name",
    "whiteboard.whiteboards",
    "whiteboard.zoom_in",
    "whiteboard.zoom_out",
    "whiteboard.zoom_reset",
];

/// Which surface the panel shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WhiteboardMode {
    /// Board gallery.
    Gallery,
    /// Board editor.
    Editor,
}

/// The Whiteboards panel entity.
pub struct CanvasView {
    provider: Arc<dyn ShellData>,
    mode: WhiteboardMode,
    gallery: GalleryModel,
    error: Option<String>,
    message: Option<String>,
    create_input: Entity<InputState>,
    create_value: String,
    _create_subscription: Subscription,
    rename_input: Entity<InputState>,
    rename_value: String,
    _rename_subscription: Subscription,
    rename_target: Option<String>,
    session: Option<EditorSession>,
    frame: Option<ViewportFrame>,
    frame_image: Option<Arc<RenderImage>>,
    geometry: Rc<Cell<canvas::CanvasGeometry>>,
    library_open: bool,
    library_items: Vec<LibraryItemRow>,
    library_input: Entity<InputState>,
    library_value: String,
    _library_subscription: Subscription,
    text_input: Entity<InputState>,
    _text_subscription: Subscription,
    /// Suppresses the blur-commit for a start-state text edit until the user
    /// actually interacts: window activation blurs the freshly focused field
    /// before the canvas has painted, which would close the edit immediately.
    hold_text_edit: bool,
    style_open: bool,
    focus: FocusHandle,
    save_generation: u64,
    pending_navigation: Option<String>,
    pending_toast: Option<String>,
    start_export: Option<ExportFormat>,
    start_element: Option<String>,
}

impl CanvasView {
    /// Build the panel and load the gallery.
    ///
    /// `start` carries the deterministic capture state (`STRATUM_WHITEBOARD*`)
    /// or a `/whiteboards?...` deep link; a board in the start state opens the
    /// editor immediately.
    pub fn new(
        provider: Arc<dyn ShellData>,
        start: CanvasStart,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let create_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t_global("whiteboard.whiteboard_name"))
        });
        let create_subscription =
            cx.subscribe(&create_input, |this: &mut Self, input, event, cx| {
                if matches!(event, InputEvent::Change) {
                    this.create_value = input.read(cx).value().to_string();
                    cx.notify();
                }
            });
        let rename_input =
            cx.new(|cx| InputState::new(window, cx).placeholder(t_global("whiteboard.new_name")));
        let rename_subscription =
            cx.subscribe(&rename_input, |this: &mut Self, input, event, cx| {
                if matches!(event, InputEvent::Change) {
                    this.rename_value = input.read(cx).value().to_string();
                    cx.notify();
                }
            });
        let library_input = cx
            .new(|cx| InputState::new(window, cx).placeholder(t_global("whiteboard.library_item")));
        let library_subscription =
            cx.subscribe(&library_input, |this: &mut Self, input, event, cx| {
                if matches!(event, InputEvent::Change) {
                    this.library_value = input.read(cx).value().to_string();
                    cx.notify();
                }
            });
        // The inline text editor: Change streams the buffer into the active
        // `CanvasState` text edit, Enter/Blur commits it, and Escape is handled
        // by the canvas key listener so it can cancel through the canvas state.
        let text_input = cx.new(|cx| InputState::new(window, cx));
        let text_subscription =
            cx.subscribe(
                &text_input,
                |this: &mut Self, input, event, cx| match event {
                    InputEvent::Change => {
                        let value = input.read(cx).value().to_string();
                        this.update_text_edit(&value, cx);
                    }
                    InputEvent::PressEnter { .. } => {
                        this.commit_text_edit(None, cx);
                        this.focus_canvas_later(cx);
                    }
                    InputEvent::Blur => {
                        // A start-state edit stays open through window
                        // activation; the first real interaction clears the hold.
                        if !this.hold_text_edit {
                            this.commit_text_edit(None, cx);
                        }
                    }
                    InputEvent::Focus => {}
                },
            );
        let mut view = Self {
            provider,
            mode: WhiteboardMode::Gallery,
            gallery: GalleryModel::default(),
            error: None,
            message: None,
            create_input,
            create_value: String::new(),
            _create_subscription: create_subscription,
            rename_input,
            rename_value: String::new(),
            _rename_subscription: rename_subscription,
            rename_target: None,
            session: None,
            frame: None,
            frame_image: None,
            geometry: Rc::new(Cell::new(canvas::CanvasGeometry::default())),
            library_open: start.library_open,
            library_items: Vec::new(),
            library_input,
            library_value: String::new(),
            _library_subscription: library_subscription,
            text_input,
            _text_subscription: text_subscription,
            hold_text_edit: false,
            style_open: start.style_open,
            focus: cx.focus_handle(),
            save_generation: 0,
            pending_navigation: None,
            pending_toast: None,
            start_export: start.export,
            start_element: start.element.clone(),
        };
        let start_text_edit = start.text_edit;
        view.reload_gallery(cx);
        if let Some(board) = start.board {
            view.open_board(&board, start.tool, window, cx);
        }
        // Capture/start-state hook: open a fresh inline text edit so the
        // overlay is visible without a pointer interaction. The mode and
        // session guards keep the gallery and failed opens inert.
        if start_text_edit && view.mode == WhiteboardMode::Editor {
            if let Some(session) = view.session.as_mut() {
                session.state_mut().set_tool(Tool::Text);
                let point = Point::new(60.0, 40.0);
                session.state_mut().pointer_down(PointerEvent {
                    point,
                    screen: point,
                    modifiers: Modifiers::default(),
                });
            }
            view.hold_text_edit = true;
            view.rebuild_frame(cx);
            view.begin_text_input(window, cx);
            // Capture hook: a still frame cannot show a blinking caret or an
            // empty buffer, so seed the field with sample text. A real edit
            // always starts from the element's own content.
            view.text_input.update(cx, |input, cx| {
                input.set_value("Architecture notes".to_string(), window, cx);
            });
            view.rebuild_frame(cx);
        }
        if view.library_open {
            view.reload_library(cx);
        }
        view
    }

    /// The active surface.
    pub fn mode(&self) -> WhiteboardMode {
        self.mode
    }

    /// The gallery model.
    pub fn gallery(&self) -> &GalleryModel {
        &self.gallery
    }

    /// The open editor session, if any.
    pub fn session(&self) -> Option<&EditorSession> {
        self.session.as_ref()
    }

    /// The last rendered viewport frame, if any.
    pub fn frame(&self) -> Option<&ViewportFrame> {
        self.frame.as_ref()
    }

    /// The library rows loaded for the drawer.
    pub fn library_items(&self) -> &[LibraryItemRow] {
        &self.library_items
    }

    /// Whether the library drawer is open.
    pub fn library_open(&self) -> bool {
        self.library_open
    }

    /// Whether the style panel is open.
    pub fn style_open(&self) -> bool {
        self.style_open
    }

    /// The current inline error, if any.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// The current inline status message, if any.
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    /// The pending rename target.
    pub fn rename_target(&self) -> Option<&str> {
        self.rename_target.as_deref()
    }
    /// The create-name input.
    pub fn create_name_input(&self) -> &Entity<InputState> {
        &self.create_input
    }

    /// The rename input.
    pub fn rename_input(&self) -> &Entity<InputState> {
        &self.rename_input
    }

    /// The library item-name input.
    pub fn library_name_input(&self) -> &Entity<InputState> {
        &self.library_input
    }

    /// The inline canvas text input.
    pub fn text_input(&self) -> &Entity<InputState> {
        &self.text_input
    }

    /// The canvas letterbox geometry.
    pub(crate) fn geometry(&self) -> Rc<Cell<canvas::CanvasGeometry>> {
        Rc::clone(&self.geometry)
    }

    /// The cached canvas frame image.
    pub(crate) fn frame_image(&self) -> Option<Arc<RenderImage>> {
        self.frame_image.clone()
    }

    /// The editor focus handle.
    pub(crate) fn focus(&self) -> &FocusHandle {
        &self.focus
    }
}

impl PanelRequests for CanvasView {
    fn pending_navigation(&self) -> Option<&str> {
        self.pending_navigation.as_deref()
    }

    fn clear_pending_navigation(&mut self) {
        self.pending_navigation = None;
    }

    fn pending_toast(&self) -> Option<&str> {
        self.pending_toast.as_deref()
    }

    fn take_pending_toast(&mut self) -> Option<String> {
        self.pending_toast.take()
    }
}

impl Render for CanvasView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let _ = window;
        let body = match self.mode {
            WhiteboardMode::Gallery => gallery::gallery(self, cx).into_any_element(),
            WhiteboardMode::Editor => editor::editor(self, cx).into_any_element(),
        };
        let theme = cx.theme();
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.colors.background)
            .text_color(theme.colors.foreground)
            .child(body)
    }
}
