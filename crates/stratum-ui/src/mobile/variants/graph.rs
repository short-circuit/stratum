//! Mobile Graph variant.
//!
//! Wraps the live [`GraphView`](crate::panels::graph::GraphView). The shell
//! creates the graph with `GraphStart::use_3d = Some(false)` on mobile, so the
//! variant opens on the 2D orthographic camera; the settings panel can still
//! switch to 3D.

use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::mobile::insets::MobileFrame;
use crate::router::Panel;
use crate::shell::Shell;

/// Render the Graph surface for the mobile shell.
pub fn render(shell: &Shell, _frame: &MobileFrame, cx: &mut Context<Shell>) -> AnyElement {
    match shell.graph_view() {
        Some(graph) => div().size_full().child(graph.clone()).into_any_element(),
        None => super::placeholder(Panel::Graph, cx),
    }
}
