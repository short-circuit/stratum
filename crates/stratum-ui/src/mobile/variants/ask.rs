//! Mobile Ask Notes variant.
//!
//! Wraps the live [`AskView`](crate::panels::ask::AskView) and reserves the
//! keyboard height below the content so the composer and answer stay visible
//! while the IME is up.

use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::mobile::insets::MobileFrame;
use crate::router::Panel;
use crate::shell::Shell;

/// Render the Ask Notes surface for the mobile shell.
pub fn render(shell: &Shell, frame: &MobileFrame, cx: &mut Context<Shell>) -> AnyElement {
    match shell.ask_view() {
        Some(ask) => div()
            .flex()
            .flex_col()
            .size_full()
            .pb(px(frame.ime))
            .child(div().flex_1().overflow_hidden().child(ask.clone()))
            .into_any_element(),
        None => super::placeholder(Panel::AskNotes, cx),
    }
}
