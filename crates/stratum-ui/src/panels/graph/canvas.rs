//! Graph canvas: cached-image painting plus orbit, pan, zoom and node actions.
//!
//! The panel rasterizes the graph once and paints it as a scaled image. This
//! module owns the letterbox geometry shared with the mouse handlers, so a click
//! maps to the same pixel the software renderer produced. Left-clicking a node
//! navigates to its page; right-clicking starts the shipped 500 ms focus tween.
//! Dragging orbits (left), pans (right/middle) and the wheel dollies the camera.
//! The canvas takes focus on click and mirrors the shipped WASD/arrow camera
//! steps, cancelling any running focus tween when a control interaction starts.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Instant;

use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_graph::MoveKey;

use super::scene::{IMAGE_HEIGHT, IMAGE_WIDTH};
use super::GraphView;

/// Fallback canvas height used when the letterbox geometry is not measured yet.
const FALLBACK_CANVAS_HEIGHT: f32 = 600.0;

/// The rendered image bounds inside the canvas, in window pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CanvasGeometry {
    /// Canvas window bounds: `x, y, width, height`.
    pub window: [f32; 4],
    /// Image window bounds: `x, y, width, height`.
    pub image: [f32; 4],
}

impl CanvasGeometry {
    /// Map a window-space point to image pixels, rejecting letterbox hits.
    pub fn to_image(&self, window_x: f32, window_y: f32) -> Option<(f32, f32)> {
        let [x, y, width, height] = self.image;
        if width <= f32::EPSILON || height <= f32::EPSILON {
            return None;
        }
        if window_x < x || window_y < y || window_x > x + width || window_y > y + height {
            return None;
        }
        let image_x = (window_x - x) / width * IMAGE_WIDTH as f32;
        let image_y = (window_y - y) / height * IMAGE_HEIGHT as f32;
        Some((image_x, image_y))
    }
}

/// An active camera drag.
#[derive(Clone, Copy, Debug)]
pub struct DragState {
    /// Pressed button.
    pub button: MouseButton,
    /// Press position in window space.
    pub start: [f32; 2],
    /// Last position in window space.
    pub last: [f32; 2],
    /// Whether the pointer moved past the click threshold.
    pub moved: bool,
}

impl DragState {
    fn new(button: MouseButton, position: [f32; 2]) -> Self {
        Self {
            button,
            start: position,
            last: position,
            moved: false,
        }
    }
}

/// Build the graph canvas element.
pub(super) fn graph_canvas(view: &GraphView, cx: &mut Context<GraphView>) -> impl IntoElement {
    let geometry = view.geometry.clone();
    let image = view.image.clone();
    let background = rgb(0x0b0f14);
    div()
        .id("graph-canvas")
        .debug_selector(|| "graph-canvas".to_string())
        .relative()
        .size_full()
        .bg(background)
        .track_focus(&view.focus)
        .on_key_down(cx.listener(|view, event: &KeyDownEvent, _, cx| {
            if let Some(key) = move_key(event) {
                view.step_camera(key);
                cx.notify();
                cx.stop_propagation();
            }
        }))
        .child(
            canvas(
                move |bounds, _window, _cx| bounds,
                move |bounds, _, window, _cx| {
                    let canvas_size = bounds.size;
                    let width = canvas_size.width.as_f32();
                    let height = canvas_size.height.as_f32();
                    let scale = (width / IMAGE_WIDTH as f32).min(height / IMAGE_HEIGHT as f32);
                    let image_width = IMAGE_WIDTH as f32 * scale;
                    let image_height = IMAGE_HEIGHT as f32 * scale;
                    let image_x = bounds.origin.x.as_f32() + (width - image_width) * 0.5;
                    let image_y = bounds.origin.y.as_f32() + (height - image_height) * 0.5;
                    geometry.set(CanvasGeometry {
                        window: [
                            bounds.origin.x.as_f32(),
                            bounds.origin.y.as_f32(),
                            width,
                            height,
                        ],
                        image: [image_x, image_y, image_width, image_height],
                    });
                    let Some(image) = image else {
                        return;
                    };
                    let image_bounds = Bounds::new(
                        point(px(image_x), px(image_y)),
                        size(px(image_width), px(image_height)),
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
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|view, event: &MouseDownEvent, window, cx| {
                view.mouse_down(MouseButton::Left, event.position, window, cx);
            }),
        )
        .on_mouse_down(
            MouseButton::Right,
            cx.listener(|view, event: &MouseDownEvent, window, cx| {
                view.mouse_down(MouseButton::Right, event.position, window, cx);
            }),
        )
        .on_mouse_down(
            MouseButton::Middle,
            cx.listener(|view, event: &MouseDownEvent, window, cx| {
                view.mouse_down(MouseButton::Middle, event.position, window, cx);
            }),
        )
        .on_mouse_move(cx.listener(|view, event: &MouseMoveEvent, _, cx| {
            view.mouse_move(event.position, event.pressed_button, cx);
        }))
        .on_mouse_up(
            MouseButton::Left,
            cx.listener(|view, event: &MouseUpEvent, _, cx| {
                view.mouse_up(MouseButton::Left, event.position, cx);
            }),
        )
        .on_mouse_up(
            MouseButton::Right,
            cx.listener(|view, event: &MouseUpEvent, _, cx| {
                view.mouse_up(MouseButton::Right, event.position, cx);
            }),
        )
        .on_mouse_up(
            MouseButton::Middle,
            cx.listener(|view, event: &MouseUpEvent, _, cx| {
                view.mouse_up(MouseButton::Middle, event.position, cx);
            }),
        )
        .on_scroll(cx.listener(|view, event: &ScrollEvent, _, cx| {
            view.scroll(event, cx);
        }))
}

/// Map a key-down event to a discrete camera move.
fn move_key(event: &KeyDownEvent) -> Option<MoveKey> {
    match event.keystroke.key.to_ascii_lowercase().as_str() {
        "w" | "up" => Some(MoveKey::Forward),
        "s" | "down" => Some(MoveKey::Back),
        "a" | "left" => Some(MoveKey::Left),
        "d" | "right" => Some(MoveKey::Right),
        _ => None,
    }
}

impl GraphView {
    /// Begin a camera drag or record a click candidate.
    fn mouse_down(
        &mut self,
        button: MouseButton,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus, cx);
        // Any pointer interaction cancels the shipped focus tween.
        self.tween = None;
        self.drag = Some(DragState::new(
            button,
            [position.x.as_f32(), position.y.as_f32()],
        ));
        cx.notify();
    }

    /// Apply a discrete WASD/arrow camera step.
    fn step_camera(&mut self, key: MoveKey) {
        self.camera.step(key);
        self.tween = None;
        self.dirty = true;
    }

    /// Update the drag, orbiting or panning the camera.
    fn mouse_move(
        &mut self,
        position: Point<Pixels>,
        pressed: Option<MouseButton>,
        cx: &mut Context<Self>,
    ) {
        let current = [position.x.as_f32(), position.y.as_f32()];
        if let Some(drag) = self.drag.as_mut() {
            let dx = current[0] - drag.last[0];
            let dy = current[1] - drag.last[1];
            let distance = (current[0] - drag.start[0]).hypot(current[1] - drag.start[1]);
            if distance > 3.0 {
                drag.moved = true;
            }
            drag.last = current;
            let moved = distance > 3.0;
            let button = drag.button;
            self.camera_drag(button, dx, dy, moved);
        } else if pressed.is_none() {
            let hover = self.pick_at(current[0], current[1]);
            self.set_hover(hover, cx);
        }
        cx.notify();
    }

    /// Apply a camera drag step for `button`.
    fn camera_drag(&mut self, button: MouseButton, dx: f32, dy: f32, moved: bool) {
        if !moved {
            return;
        }
        match button {
            MouseButton::Left => self.camera.orbit(dx * 0.01, -dy * 0.01),
            MouseButton::Right | MouseButton::Middle => {
                let height = self.geometry.get().window[3];
                let height = if height > f32::EPSILON {
                    height
                } else {
                    FALLBACK_CANVAS_HEIGHT
                };
                self.camera.pan_pixels(dx, dy, height);
            }
            _ => return,
        }
        self.tween = None;
        self.dirty = true;
    }

    /// Finish a drag; a short press becomes a click action.
    fn mouse_up(&mut self, button: MouseButton, position: Point<Pixels>, cx: &mut Context<Self>) {
        let current = [position.x.as_f32(), position.y.as_f32()];
        let Some(drag) = self.drag.take() else {
            return;
        };
        if drag.moved || drag.button != button {
            cx.notify();
            return;
        }
        let Some(index) = self.pick_at(current[0], current[1]) else {
            cx.notify();
            return;
        };
        match button {
            MouseButton::Left => {
                if let Some(path) = self
                    .filtered
                    .as_ref()
                    .and_then(|filtered| filtered.nodes.get(index))
                    .map(|node| node.path.clone())
                {
                    self.pending_navigation = Some(path);
                }
            }
            MouseButton::Right => self.start_focus(index),
            _ => {}
        }
        cx.notify();
    }

    /// Dolly the camera from a scroll event.
    fn scroll(&mut self, event: &ScrollEvent, cx: &mut Context<Self>) {
        let delta = match event.delta {
            ScrollDelta::Pixels(point) => point.y.as_f32(),
            ScrollDelta::Lines(point) => point.y * 24.0,
        };
        if delta.abs() <= f32::EPSILON {
            return;
        }
        self.camera.zoom_steps(delta / 100.0);
        self.tween = None;
        self.dirty = true;
        cx.notify();
    }

    /// Start the shipped focus tween toward a node.
    fn start_focus(&mut self, index: usize) {
        let Some(scene) = self.scene.as_ref() else {
            return;
        };
        let Some(position) = super::scene::node_position(scene, index) else {
            return;
        };
        self.tween = Some(self.camera.focus_tween(position, Instant::now()));
        self.dirty = true;
    }

    /// Pick a node at a window-space point.
    fn pick_at(&self, window_x: f32, window_y: f32) -> Option<usize> {
        let (image_x, image_y) = self.geometry.get().to_image(window_x, window_y)?;
        let scene = self.scene.as_ref()?;
        scene.pick_cpu(&self.camera, image_x, image_y, IMAGE_WIDTH, IMAGE_HEIGHT)
    }
}

/// Build the shared letterbox geometry cell.
pub(super) fn geometry_cell() -> Rc<Cell<CanvasGeometry>> {
    Rc::new(Cell::new(CanvasGeometry::default()))
}

#[cfg(test)]
mod tests {
    use super::{move_key, CanvasGeometry};
    use ::core::prelude::v1::test;
    use gpui_kit::{KeyDownEvent, Keystroke};
    use stratum_graph::MoveKey;

    fn key_event(key: &str) -> KeyDownEvent {
        KeyDownEvent {
            keystroke: Keystroke {
                key: key.to_string(),
                ..Keystroke::default()
            },
            is_held: false,
            prefer_character_input: false,
        }
    }

    #[test]
    fn move_key_maps_wasd_and_arrows_case_insensitively() {
        for (key, expected) in [
            ("w", MoveKey::Forward),
            ("W", MoveKey::Forward),
            ("up", MoveKey::Forward),
            ("s", MoveKey::Back),
            ("S", MoveKey::Back),
            ("down", MoveKey::Back),
            ("a", MoveKey::Left),
            ("A", MoveKey::Left),
            ("left", MoveKey::Left),
            ("d", MoveKey::Right),
            ("D", MoveKey::Right),
            ("right", MoveKey::Right),
        ] {
            assert_eq!(move_key(&key_event(key)), Some(expected), "key {key}");
        }
    }

    #[test]
    fn move_key_rejects_unrelated_keys() {
        for key in ["q", "e", "escape", "space", "1", "tab"] {
            assert_eq!(move_key(&key_event(key)), None, "key {key}");
        }
    }

    #[test]
    fn letterbox_maps_only_inside_the_image() {
        let geometry = CanvasGeometry {
            window: [0.0, 0.0, 800.0, 600.0],
            image: [100.0, 0.0, 600.0, 600.0],
        };
        assert_eq!(geometry.to_image(400.0, 300.0), Some((480.0, 300.0)));
        assert_eq!(geometry.to_image(50.0, 300.0), None);
        assert_eq!(geometry.to_image(400.0, 700.0), None);
    }

    #[test]
    fn empty_geometry_rejects_every_point() {
        let geometry = CanvasGeometry::default();
        assert_eq!(geometry.to_image(0.0, 0.0), None);
    }
}
