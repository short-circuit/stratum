//! Mobile Plugins variant.
//!
//! Wraps the live [`PluginsView`](crate::panels::plugins::PluginsView); the
//! panel owns the installed/directory/themes tabs and the enable/uninstall
//! actions.

use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::mobile::insets::MobileFrame;
use crate::router::Panel;
use crate::shell::Shell;

/// Render the Plugins surface for the mobile shell.
pub fn render(shell: &Shell, _frame: &MobileFrame, cx: &mut Context<Shell>) -> AnyElement {
    match shell.plugins_view() {
        Some(plugins) => div().size_full().child(plugins.clone()).into_any_element(),
        None => super::placeholder(Panel::Plugins, cx),
    }
}
