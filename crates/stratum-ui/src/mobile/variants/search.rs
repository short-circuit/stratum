//! Mobile search variant.
//!
//! Wraps the live [`SearchView`](crate::panels::search::SearchView); the panel
//! owns its query field, tag mode and result list.

use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::mobile::insets::MobileFrame;
use crate::router::Panel;
use crate::shell::Shell;

/// Render the search surface for the mobile shell.
pub fn render(shell: &Shell, _frame: &MobileFrame, cx: &mut Context<Shell>) -> AnyElement {
    match shell.search_view() {
        Some(search) => div().size_full().child(search.clone()).into_any_element(),
        None => super::placeholder(Panel::Search, cx),
    }
}
