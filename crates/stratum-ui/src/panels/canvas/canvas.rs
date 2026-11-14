//! Whiteboard canvas: frame painting, pointer/wheel routing and keyboard edits.
//!
//! The panel rasterizes the visible scene into a cached PNG frame (see
//! [`super::model::EditorSession::viewport_frame`]) and paints it through the
//! same letterbox geometry the mouse handlers use, so a click maps to the world
//! point the [`stratum_canvas::CanvasState`] expects. Pointer and wheel events
//! are forwarded to the canvas state machine; document-changing actions mark an
//! edit and schedule the 800 ms autosave, while pan/zoom only rebuilds the
//! cached frame.

use std::sync::Arc;

use gpui_kit::component::input::Input;
use gpui_kit::component::ActiveTheme;
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_canvas::{
    CanvasAction, Modifiers as CanvasModifiers, Point as CanvasPoint, PointerEvent, Tool,
};

use super::model::{ViewportFrame, FRAME_HEIGHT, FRAME_WIDTH};
use super::CanvasView;

/// The rendered frame bounds inside the canvas, in window pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CanvasGeometry {
    /// Canvas window bounds: `x, y, width, height`.
    pub window: [f32; 4],
    /// Frame window bounds: `x, y, width, height`.
    pub image: [f32; 4],
}

impl CanvasGeometry {
    /// Map a window-space point to frame coordinates, rejecting letterbox hits.
    pub fn to_frame(&self, window_x: f32, window_y: f32) -> Option<(f64, f64)> {
        let [x, y, width, height] = self.image;
        if width <= f32::EPSILON || height <= f32::EPSILON {
            return None;
        }
        if window_x < x || window_y < y || window_x > x + width || window_y > y + height {
            return None;
        }
        Some((
            f64::from(window_x - x) / f64::from(width) * FRAME_WIDTH,
            f64::from(window_y - y) / f64::from(height) * FRAME_HEIGHT,
        ))
    }
}

/// Convert a rendered frame into the BGRA [`RenderImage`] gpui paints.
pub(super) fn render_image(frame: &ViewportFrame) -> Option<Arc<RenderImage>> {
    let decoded = image::load_from_memory_with_format(&frame.png, image::ImageFormat::Png).ok()?;
    let rgba = decoded.to_rgba8();
    let (width, height) = rgba.dimensions();
    let mut raw = rgba.into_raw();
    // gpui's RenderImage stores BGRA bytes; the rasterizer produces RGBA.
    for pixel in raw.as_chunks_mut::<4>().0 {
        pixel.swap(0, 2);
    }
    let buffer = image::RgbaImage::from_raw(width, height, raw)?;
    Some(Arc::new(RenderImage::new(vec![image::Frame::new(buffer)])))
}

/// Build the canvas element.
pub(super) fn canvas_element(view: &CanvasView, cx: &mut Context<CanvasView>) -> impl IntoElement {
    let geometry = view.geometry();
    let image = view.frame_image();
    let entity_id = cx.entity_id();
    let background = rgb(0xf8f9fa);
    div()
        .id("whiteboard-canvas")
        .debug_selector(|| "whiteboard-canvas".to_string())
        .relative()
        .flex_1()
        .overflow_hidden()
        .bg(background)
        .track_focus(view.focus())
        .on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
            if view.key_down(event, window, cx) {
                cx.stop_propagation();
            }
        }))
        .child(
            canvas(
                move |bounds, _, _| bounds,
                move |bounds, _, window, cx| {
                    let width = bounds.size.width.as_f32();
                    let height = bounds.size.height.as_f32();
                    let scale = (width / FRAME_WIDTH as f32).min(height / FRAME_HEIGHT as f32);
                    let frame_width = FRAME_WIDTH as f32 * scale;
                    let frame_height = FRAME_HEIGHT as f32 * scale;
                    let frame_x = bounds.origin.x.as_f32() + (width - frame_width) * 0.5;
                    let frame_y = bounds.origin.y.as_f32() + (height - frame_height) * 0.5;
                    let previous = geometry.get();
                    geometry.set(CanvasGeometry {
                        window: [
                            bounds.origin.x.as_f32(),
                            bounds.origin.y.as_f32(),
                            width,
                            height,
                        ],
                        image: [frame_x, frame_y, frame_width, frame_height],
                    });
                    // The first paint populates the letterbox geometry the inline
                    // text overlay measures against; request one follow-up frame so
                    // a start-state text edit becomes visible without a pointer
                    // interaction.
                    if previous.image[2] <= f32::EPSILON && frame_width > f32::EPSILON {
                        cx.notify(entity_id);
                    }
                    let Some(image) = image else {
                        return;
                    };
                    let image_bounds = Bounds::new(
                        point(px(frame_x), px(frame_y)),
                        size(px(frame_width), px(frame_height)),
                    );
                    let _ = window.paint_image(
                        bounds,
                        image_bounds,
                        Corners::default(),
                        image,
                        0,
                        false,
                    );
                },
            )
            .absolute()
            .inset_0(),
        )
        .when_some(text_overlay_bounds(view), |element, bounds| {
            element.child(text_overlay(view, bounds, cx))
        })
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|view, event: &MouseDownEvent, window, cx| {
                view.mouse_down(event, window, cx);
            }),
        )
        .on_mouse_move(cx.listener(|view, event: &MouseMoveEvent, _, cx| {
            view.mouse_move(event, cx);
        }))
        .on_mouse_up(
            MouseButton::Left,
            cx.listener(|view, event: &MouseUpEvent, _, cx| {
                view.mouse_up(event, cx);
            }),
        )
        .on_scroll(cx.listener(|view, event: &ScrollEvent, _, cx| {
            view.scroll(event, cx);
        }))
}

/// Compute the inline text input rect over the edited element, relative to the
/// canvas element.
///
/// The world element rect is transformed through the same viewport transform
/// the pointer handler uses, then mapped into window space through the
/// letterbox geometry. The overlay is clamped to a usable minimum size.
fn text_overlay_bounds(view: &CanvasView) -> Option<[f32; 4]> {
    let session = view.session()?;
    let state = session.state();
    let id = state.text_editing_id()?;
    let element = state.document().element(id)?;
    let geometry = view.geometry().get();
    let [_, _, image_width, image_height] = geometry.image;
    if image_width <= f32::EPSILON || image_height <= f32::EPSILON {
        return None;
    }
    let viewport = state.viewport();
    let zoom = viewport.zoom as f32;
    let scale_x = image_width / FRAME_WIDTH as f32;
    let scale_y = image_height / FRAME_HEIGHT as f32;
    let frame_x = element.x() as f32 * zoom + viewport.x as f32;
    let frame_y = element.y() as f32 * zoom + viewport.y as f32;
    let frame_width = element.width() as f32 * zoom;
    let frame_height = element.height() as f32 * zoom;
    let window_x = geometry.image[0] + frame_x * scale_x;
    let window_y = geometry.image[1] + frame_y * scale_y;
    Some([
        window_x - geometry.window[0],
        window_y - geometry.window[1],
        (frame_width * scale_x).max(120.0),
        (frame_height * scale_y).max(28.0),
    ])
}

/// Build the absolutely positioned inline text input over the edited element.
fn text_overlay(
    view: &CanvasView,
    bounds: [f32; 4],
    cx: &mut Context<CanvasView>,
) -> impl IntoElement {
    // The overlay follows the shell theme instead of the board palette: the
    // inline field's text colour comes from the active theme, so the chip has
    // to carry the matching surface (the canvas itself stays light in both
    // modes, which would otherwise leave the text invisible in dark mode).
    let theme = cx.theme();
    div()
        .id("whiteboard-text-input")
        .debug_selector(|| "whiteboard-text-input".to_string())
        .absolute()
        .left(px(bounds[0]))
        .top(px(bounds[1]))
        .w(px(bounds[2]))
        .h(px(bounds[3]))
        .bg(theme.colors.background)
        .border_1()
        .border_color(theme.colors.border)
        .rounded(px(4.0))
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(Input::new(view.text_input()).appearance(false))
}

impl CanvasView {
    /// Handle a canvas press.
    fn mouse_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.hold_text_edit = false;
        // A canvas press ends any active inline edit before the canvas state
        // machine runs, so a new text edit cannot replace the previous one
        // before its blur commit arrives.
        if self
            .session
            .as_ref()
            .is_some_and(|session| session.state().text_editing().is_some())
        {
            self.commit_text_edit(Some(&mut *window), cx);
        }
        window.focus(&self.focus, cx);
        let Some(pointer) = self.pointer_event(event.position, &event.modifiers) else {
            return;
        };
        let Some(session) = self.session.as_mut() else {
            return;
        };
        let action = if event.click_count >= 2 {
            session.state_mut().double_click(pointer)
        } else {
            session.state_mut().pointer_down(pointer)
        };
        let editing = matches!(action, CanvasAction::TextEditing(_));
        self.finish_pointer(action, cx);
        if editing {
            self.begin_text_input(window, cx);
        }
    }

    /// Handle a canvas drag.
    fn mouse_move(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) {
        if event.pressed_button.is_none() {
            return;
        }
        let Some(pointer) = self.pointer_event(event.position, &event.modifiers) else {
            return;
        };
        let Some(session) = self.session.as_mut() else {
            return;
        };
        let action = session.state_mut().pointer_move(pointer);
        self.finish_pointer(action, cx);
    }

    /// Handle a canvas release.
    fn mouse_up(&mut self, event: &MouseUpEvent, cx: &mut Context<Self>) {
        let Some(pointer) = self.pointer_event(event.position, &event.modifiers) else {
            return;
        };
        let Some(session) = self.session.as_mut() else {
            return;
        };
        let action = session.state_mut().pointer_up(pointer);
        self.finish_pointer(action, cx);
    }

    /// Zoom the viewport at the pointer.
    fn scroll(&mut self, event: &ScrollEvent, cx: &mut Context<Self>) {
        let delta = match event.delta {
            ScrollDelta::Pixels(point) => point.y.as_f32(),
            ScrollDelta::Lines(point) => point.y * 24.0,
        };
        if delta.abs() <= f32::EPSILON {
            return;
        }
        let Some((frame_x, frame_y)) = self
            .geometry
            .get()
            .to_frame(event.position.x.as_f32(), event.position.y.as_f32())
        else {
            return;
        };
        let factor = if delta > 0.0 { 0.9 } else { 1.1 };
        if let Some(session) = self.session.as_mut() {
            session
                .state_mut()
                .zoom_at(CanvasPoint::new(frame_x, frame_y), factor);
        }
        self.rebuild_frame(cx);
    }

    /// Forward a key press to the canvas state machine.
    ///
    /// Returns whether the key was consumed so the shell shortcut handler does
    /// not see it.
    fn key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        self.hold_text_edit = false;
        let key = event.keystroke.key.to_ascii_lowercase();
        let control = event.keystroke.modifiers.control || event.keystroke.modifiers.platform;
        if control {
            match key.as_str() {
                "z" => {
                    if event.keystroke.modifiers.shift {
                        self.redo(cx);
                    } else {
                        self.undo(cx);
                    }
                    return true;
                }
                "y" => {
                    self.redo(cx);
                    return true;
                }
                "c" => {
                    self.copy_selection(cx);
                    return true;
                }
                "v" => {
                    self.paste(cx);
                    return true;
                }
                "a" => {
                    if let Some(session) = self.session.as_mut() {
                        session.state_mut().select_all();
                    }
                    self.rebuild_frame(cx);
                    return true;
                }
                "d" => {
                    self.duplicate_selection(cx);
                    return true;
                }
                "g" => {
                    if event.keystroke.modifiers.shift {
                        self.ungroup_selection(cx);
                    } else {
                        self.group_selection(cx);
                    }
                    return true;
                }
                "l" => {
                    // Shift+`l` toggles the lock; plain `l` falls through to the
                    // Line tool shortcut below.
                    if event.keystroke.modifiers.shift {
                        self.toggle_lock_selection(cx);
                        return true;
                    }
                }
                "]" | "}" => {
                    // A shifted bracket arrives as `}` on layouts that remap
                    // the glyph, so treat the brace as the shift variant too.
                    let shift = event.keystroke.modifiers.shift || key == "}";
                    if shift {
                        self.bring_to_front(cx);
                    } else {
                        self.bring_forward(cx);
                    }
                    return true;
                }
                "[" | "{" => {
                    let shift = event.keystroke.modifiers.shift || key == "{";
                    if shift {
                        self.send_to_back(cx);
                    } else {
                        self.send_backward(cx);
                    }
                    return true;
                }
                _ => {}
            }
        }
        match key.as_str() {
            "arrowup" | "arrowdown" | "arrowleft" | "arrowright" => {
                let step = if event.keystroke.modifiers.shift {
                    10.0
                } else {
                    1.0
                };
                let (dx, dy) = match key.as_str() {
                    "arrowup" => (0.0, -step),
                    "arrowdown" => (0.0, step),
                    "arrowleft" => (-step, 0.0),
                    _ => (step, 0.0),
                };
                self.nudge_selection(dx, dy, cx);
                true
            }
            "escape" => {
                self.cancel_text_edit(window, cx);
                true
            }
            "delete" | "backspace" => {
                self.delete_selection(cx);
                true
            }
            other => match tool_shortcut(other) {
                Some(tool) => {
                    self.set_tool(tool, cx);
                    true
                }
                None => false,
            },
        }
    }

    /// Finish a pointer action: schedule autosave on document changes and
    /// rebuild the cached frame for every interaction (pan/zoom included).
    fn finish_pointer(&mut self, action: CanvasAction, cx: &mut Context<Self>) {
        if action == CanvasAction::DocumentChanged {
            if let Some(session) = self.session.as_mut() {
                session.note_edit(std::time::Instant::now());
            }
            self.rebuild_frame(cx);
            self.schedule_save(cx);
        } else {
            self.rebuild_frame(cx);
        }
    }

    /// Build a canvas pointer event from a window position and modifiers.
    fn pointer_event(
        &self,
        position: Point<Pixels>,
        modifiers: &Modifiers,
    ) -> Option<PointerEvent> {
        let (frame_x, frame_y) = self
            .geometry
            .get()
            .to_frame(position.x.as_f32(), position.y.as_f32())?;
        let viewport = self.session.as_ref()?.state().viewport();
        Some(PointerEvent {
            point: CanvasPoint::new(
                (frame_x - viewport.x) / viewport.zoom,
                (frame_y - viewport.y) / viewport.zoom,
            ),
            screen: CanvasPoint::new(frame_x, frame_y),
            modifiers: CanvasModifiers {
                shift: modifiers.shift,
                alt: modifiers.alt,
                ctrl: modifiers.control,
            },
        })
    }
}

/// Map a single-key tool shortcut.
fn tool_shortcut(key: &str) -> Option<Tool> {
    match key {
        "v" => Some(Tool::Select),
        "h" => Some(Tool::Hand),
        "r" => Some(Tool::Rectangle),
        "d" => Some(Tool::Diamond),
        "o" => Some(Tool::Ellipse),
        "a" => Some(Tool::Arrow),
        "l" => Some(Tool::Line),
        "p" => Some(Tool::Freedraw),
        "t" => Some(Tool::Text),
        "e" => Some(Tool::Eraser),
        "f" => Some(Tool::Frame),
        _ => None,
    }
}
