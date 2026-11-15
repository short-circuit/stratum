//! Mobile Flashcards variant.
//!
//! Wraps the live [`FlashcardsView`](crate::panels::flashcards::FlashcardsView);
//! the panel owns the card, reveal control and SM-2 rating actions.

use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::mobile::insets::MobileFrame;
use crate::router::Panel;
use crate::shell::Shell;

/// Render the Flashcards surface for the mobile shell.
pub fn render(shell: &Shell, _frame: &MobileFrame, cx: &mut Context<Shell>) -> AnyElement {
    match shell.flashcards_view() {
        Some(flashcards) => div()
            .size_full()
            .child(flashcards.clone())
            .into_any_element(),
        None => super::placeholder(Panel::Flashcards, cx),
    }
}
