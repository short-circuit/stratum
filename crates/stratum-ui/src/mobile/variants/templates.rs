//! Mobile Templates variant.
//!
//! Wraps the live [`TemplatesView`](crate::panels::templates::TemplatesView);
//! the panel owns the target page field, variable rows and apply action.

use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::mobile::insets::MobileFrame;
use crate::router::Panel;
use crate::shell::Shell;

/// Render the Templates surface for the mobile shell.
pub fn render(shell: &Shell, _frame: &MobileFrame, cx: &mut Context<Shell>) -> AnyElement {
    match shell.templates_view() {
        Some(templates) => div()
            .size_full()
            .child(templates.clone())
            .into_any_element(),
        None => super::placeholder(Panel::Templates, cx),
    }
}
