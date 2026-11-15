//! Mobile Reminders variant.
//!
//! Wraps the live [`RemindersView`](crate::panels::reminders::RemindersView);
//! the panel owns the overdue/today/upcoming groups, snooze/complete actions and
//! the due dialog.

use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::mobile::insets::MobileFrame;
use crate::router::Panel;
use crate::shell::Shell;

/// Render the Reminders surface for the mobile shell.
pub fn render(shell: &Shell, _frame: &MobileFrame, cx: &mut Context<Shell>) -> AnyElement {
    match shell.reminders_view() {
        Some(reminders) => div()
            .size_full()
            .child(reminders.clone())
            .into_any_element(),
        None => super::placeholder(Panel::Reminders, cx),
    }
}
