//! Mobile Kanban variant.
//!
//! Wraps the live [`KanbanView`](crate::panels::kanban::KanbanView) — the board
//! is the same three-column surface, and the mobile shell adds the safe-area
//! frame around it. Tapping a card opens the edit dialog, which is how a mobile
//! user moves a card between columns without dragging; the context menu still
//! exposes edit and delete.

use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::mobile::insets::MobileFrame;
use crate::router::Panel;
use crate::shell::Shell;

/// Render the Kanban surface for the mobile shell.
pub fn render(shell: &Shell, _frame: &MobileFrame, cx: &mut Context<Shell>) -> AnyElement {
    match shell.kanban_view() {
        Some(kanban) => div().size_full().child(kanban.clone()).into_any_element(),
        None => super::placeholder(Panel::Kanban, cx),
    }
}
