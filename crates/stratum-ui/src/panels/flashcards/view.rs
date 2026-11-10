//! Render helpers for the native Flashcards panel.
//!
//! Split out of [`super`] so the entity/action module stays under the
//! repository file-length limit. These functions only read the panel's public
//! accessors and dispatch actions through its methods; all visible strings are
//! catalog keys from [`super::FLASHCARDS_I18N_KEYS`].

use gpui_kit::component::ActiveTheme;
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_app::flashcards::{rating_quality, FlashcardRating};
use stratum_i18n::{t_global, tr_global};

use super::model::SessionState;
use super::FlashcardsView;
use crate::provider::FlashcardRow;

/// Render the flashcards panel body.
pub fn render(view: &FlashcardsView, cx: &mut Context<FlashcardsView>) -> impl IntoElement {
    let theme = cx.theme();
    div()
        .flex()
        .flex_col()
        .size_full()
        .bg(theme.colors.background)
        .text_color(theme.colors.foreground)
        .child(header(cx))
        .child(
            div()
                .id("flashcards-body")
                .flex()
                .flex_col()
                .flex_1()
                .overflow_y_scroll()
                .p(px(24.0))
                .gap(px(10.0))
                .child(body(view, cx)),
        )
}

/// The panel title bar.
fn header(cx: &mut Context<FlashcardsView>) -> impl IntoElement {
    let theme = cx.theme();
    div()
        .flex()
        .items_center()
        .px(px(24.0))
        .py(px(12.0))
        .border_b_1()
        .border_color(theme.colors.border)
        .child(
            div()
                .text_size(px(18.0))
                .font_weight(FontWeight::BOLD)
                .text_color(theme.colors.foreground)
                .child(t_global("flashcards.flashcards")),
        )
}

/// Render the state-specific body.
fn body(view: &FlashcardsView, cx: &mut Context<FlashcardsView>) -> AnyElement {
    match view.session().state() {
        SessionState::Loading => loading(cx),
        SessionState::Empty => empty_state(cx),
        SessionState::Failed(error) => failed_state(error, cx),
        SessionState::Complete => complete_state(view, cx),
        SessionState::Card { .. } => card_state(view, cx),
    }
}

/// The loading placeholder.
fn loading(cx: &mut Context<FlashcardsView>) -> AnyElement {
    let theme = cx.theme();
    div()
        .id("flashcards-loading")
        .debug_selector(|| "flashcards-loading".to_string())
        .text_size(px(12.0))
        .text_color(theme.colors.muted_foreground)
        .child(t_global("flashcards.flashcards"))
        .into_any_element()
}

/// The shipped empty-state hint.
fn empty_state(cx: &mut Context<FlashcardsView>) -> AnyElement {
    let theme = cx.theme();
    let muted = theme.colors.muted_foreground;
    div()
        .id("flashcards-empty")
        .debug_selector(|| "flashcards-empty".to_string())
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(div().text_size(px(12.0)).text_color(muted).child(t_global(
            "flashcards.no_flashcards_found_create_blocks_with",
        )))
        .child(
            div()
                .flex()
                .gap(px(6.0))
                .child(code_chip(t_global("flashcards.question"), cx))
                .child(code_chip(t_global("flashcards.answer"), cx)),
        )
        .child(
            div()
                .text_size(px(12.0))
                .text_color(muted)
                .child(t_global("flashcards.properties_to_generate_cards")),
        )
        .into_any_element()
}

/// A monospace-style inline code chip for the empty state.
fn code_chip(label: String, cx: &mut Context<FlashcardsView>) -> AnyElement {
    let theme = cx.theme();
    div()
        .px(px(4.0))
        .py(px(1.0))
        .rounded(px(3.0))
        .bg(theme.colors.muted_foreground.opacity(0.16))
        .text_size(px(11.0))
        .text_color(theme.colors.foreground)
        .child(label)
        .into_any_element()
}

/// The provider failure line.
fn failed_state(error: &str, cx: &mut Context<FlashcardsView>) -> AnyElement {
    let theme = cx.theme();
    div()
        .id("flashcards-error")
        .debug_selector(|| "flashcards-error".to_string())
        .text_size(px(12.0))
        .text_color(theme.colors.danger)
        .child(format!("{} {error}", t_global("flashcards.error")))
        .into_any_element()
}

/// The session-complete view with the restart action.
fn complete_state(view: &FlashcardsView, cx: &mut Context<FlashcardsView>) -> AnyElement {
    let theme = cx.theme();
    let muted = theme.colors.muted_foreground;
    let accent = theme.colors.accent;
    let accent_foreground = theme.colors.accent_foreground;
    let count = view.cards().len();
    let count_label = count.to_string();
    let reviewed_key = if count == 1 {
        "flashcards.reviewed_cards_one"
    } else {
        "flashcards.reviewed_cards_other"
    };
    let reviewed = tr_global(reviewed_key, &[("count", count_label.as_str())]);
    div()
        .id("flashcards-complete")
        .debug_selector(|| "flashcards-complete".to_string())
        .flex()
        .flex_col()
        .items_center()
        .gap(px(8.0))
        .child(
            div()
                .text_size(px(16.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.colors.foreground)
                .child(t_global("flashcards.session_complete")),
        )
        .child(div().text_size(px(12.0)).text_color(muted).child(reviewed))
        .child(
            div()
                .id("flashcard-start-again")
                .debug_selector(|| "flashcard-start-again".to_string())
                .px(px(14.0))
                .py(px(5.0))
                .rounded(px(6.0))
                .bg(accent)
                .text_color(accent_foreground)
                .text_size(px(12.0))
                .cursor_pointer()
                .child(t_global("flashcards.start_again"))
                .on_click(cx.listener(|view, _, _, cx| view.restart(cx))),
        )
        .into_any_element()
}

/// The active-card view: progress, reveal card, source line, feedback and the
/// rating row when the answer is visible.
fn card_state(view: &FlashcardsView, cx: &mut Context<FlashcardsView>) -> AnyElement {
    let theme = cx.theme();
    let muted = theme.colors.muted_foreground;
    let accent = theme.colors.accent;
    let danger = theme.colors.danger;
    let Some(card) = view.current_card() else {
        return empty_state(cx);
    };
    let revealed = view.revealed();
    let message = view.message();
    let mut progress = view.progress_label();
    if !card.next_review.is_empty() {
        progress.push(' ');
        progress.push_str(&t_global("flashcards._next"));
        progress.push(' ');
        progress.push_str(&card.next_review);
    }
    let source = format!(
        "{} {} {} {:.1} {} {}d",
        t_global("flashcards.source"),
        card.page_path,
        t_global("flashcards._ease"),
        card.ease_factor,
        t_global("flashcards._interval"),
        card.interval_days
    );
    div()
        .id("flashcards-card-state")
        .debug_selector(|| "flashcards-card-state".to_string())
        .flex()
        .flex_col()
        .gap(px(10.0))
        .child(div().text_size(px(11.0)).text_color(muted).child(progress))
        .child(reveal_card(card, revealed, cx))
        .child(div().text_size(px(11.0)).text_color(muted).child(source))
        .when_some(message, |element, message| {
            element.child(
                div()
                    .id("flashcard-message")
                    .debug_selector(|| "flashcard-message".to_string())
                    .text_size(px(12.0))
                    .text_color(accent)
                    .child(message),
            )
        })
        .when_some(view.error().map(str::to_string), |element, error| {
            element.child(
                div()
                    .id("flashcard-review-error")
                    .debug_selector(|| "flashcard-review-error".to_string())
                    .text_size(px(12.0))
                    .text_color(danger)
                    .child(error),
            )
        })
        .when(revealed && view.message().is_none(), |element| {
            element.child(rating_row(cx))
        })
        .into_any_element()
}

/// The clickable question/answer card.
fn reveal_card(
    card: &FlashcardRow,
    revealed: bool,
    cx: &mut Context<FlashcardsView>,
) -> AnyElement {
    let theme = cx.theme();
    let muted = theme.colors.muted_foreground;
    let label = if revealed {
        t_global("flashcards.answer_label")
    } else {
        t_global("flashcards.question_label")
    };
    let text = if revealed {
        card.back.clone()
    } else {
        card.front.clone()
    };
    let hint = if revealed {
        t_global("flashcards.click_to_see_question")
    } else {
        t_global("flashcards.click_to_reveal_answer")
    };
    div()
        .id("flashcard-card")
        .debug_selector(|| "flashcard-card".to_string())
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(6.0))
        .min_h(px(160.0))
        .p(px(16.0))
        .rounded(px(8.0))
        .border_2()
        .border_color(theme.colors.border)
        .cursor_pointer()
        .child(div().text_size(px(10.0)).text_color(muted).child(label))
        .child(
            div()
                .text_size(px(15.0))
                .text_color(theme.colors.foreground)
                .child(text),
        )
        .child(div().text_size(px(10.0)).text_color(muted).child(hint))
        .on_click(cx.listener(|view, _, _, cx| view.toggle_reveal(cx)))
        .into_any_element()
}

/// The SM-2 rating row shown after the answer is revealed.
fn rating_row(cx: &mut Context<FlashcardsView>) -> AnyElement {
    let theme = cx.theme();
    let danger = theme.colors.danger;
    let warning = theme.colors.warning;
    let accent = theme.colors.accent;
    let accent_foreground = theme.colors.accent_foreground;
    div()
        .id("flashcard-rating-row")
        .debug_selector(|| "flashcard-rating-row".to_string())
        .flex()
        .gap(px(6.0))
        .children(FlashcardRating::ALL.into_iter().map(|rating| {
            let quality = rating_quality(rating);
            let key = rating_label_key(rating);
            let color = match rating {
                FlashcardRating::Blackout => danger,
                FlashcardRating::Hard => warning,
                FlashcardRating::Good | FlashcardRating::Easy => accent,
            };
            div()
                .id(format!("flashcard-rating-{quality}"))
                .debug_selector(move || format!("flashcard-rating-{quality}"))
                .px(px(12.0))
                .py(px(4.0))
                .rounded(px(6.0))
                .bg(color)
                .text_color(accent_foreground)
                .text_size(px(11.0))
                .cursor_pointer()
                .child(t_global(key))
                .on_click(cx.listener(move |view, _, _, cx| view.rate(rating, cx)))
        }))
        .into_any_element()
}

/// The catalog key for one rating button.
fn rating_label_key(rating: FlashcardRating) -> &'static str {
    match rating {
        FlashcardRating::Blackout => "flashcards.rating_blackout",
        FlashcardRating::Hard => "flashcards.rating_hard",
        FlashcardRating::Good => "flashcards.rating_good",
        FlashcardRating::Easy => "flashcards.rating_easy",
    }
}
