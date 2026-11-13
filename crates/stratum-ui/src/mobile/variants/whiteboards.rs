//! Mobile Whiteboards variant.
//!
//! Wraps the live [`CanvasView`](crate::panels::canvas::CanvasView); the panel
//! owns the gallery/editor mode, the rasterized board surface, tools and the
//! library drawer.

use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::mobile::insets::MobileFrame;
use crate::router::Panel;
use crate::shell::Shell;

/// Render the Whiteboards surface for the mobile shell.
pub fn render(shell: &Shell, _frame: &MobileFrame, cx: &mut Context<Shell>) -> AnyElement {
    match shell.canvas_view() {
        Some(canvas) => div().size_full().child(canvas.clone()).into_any_element(),
        None => super::placeholder(Panel::Whiteboards, cx),
    }
}
