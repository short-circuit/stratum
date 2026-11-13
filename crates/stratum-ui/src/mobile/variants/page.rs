//! Mobile page variant.
//!
//! Wraps the live [`PageView`](crate::panels::page::PageView); the page header
//! already owns its title, path chip and mode controls.

use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::mobile::insets::MobileFrame;
use crate::router::Panel;
use crate::shell::Shell;

/// Render the page surface for the mobile shell.
pub fn render(shell: &Shell, _frame: &MobileFrame, cx: &mut Context<Shell>) -> AnyElement {
    match shell.page_view() {
        Some(page) => div().size_full().child(page.clone()).into_any_element(),
        None => super::placeholder(Panel::Page, cx),
    }
}
