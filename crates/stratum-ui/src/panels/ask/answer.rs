//! Ask Notes answer, read-aloud control and citation rows.
//!
//! Split out of [`super::AskView::render`] to keep the render function small.
//! The shipped panel renders one of five states: searching, a provider error, a
//! read-aloud error, the answer card (with sources or the no-sources caption),
//! or nothing. Citations never navigate; they only display the page path, the
//! snippet and the two-decimal score.

use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::{ActiveTheme, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use super::AskView;
use crate::provider::AskSource;

/// Render the answer area for the current view state.
pub fn body(view: &AskView, cx: &mut Context<AskView>) -> AnyElement {
    if view.loading() {
        return loading_row(cx).into_any_element();
    }
    if let Some(error) = view.error().map(str::to_string) {
        return error_alert(error, cx).into_any_element();
    }
    if let Some(error) = view.speak_error().map(str::to_string) {
        return speak_error_alert(error, cx).into_any_element();
    }
    let Some(answer) = view.answer() else {
        return div().into_any_element();
    };
    answer_card(view, answer, cx).into_any_element()
}

/// The searching row shown while the provider query is in flight.
fn loading_row(cx: &mut Context<AskView>) -> impl IntoElement {
    let theme = cx.theme();
    let muted = theme.colors.muted_foreground;
    div()
        .id("ask-loading")
        .debug_selector(|| "ask-loading".to_string())
        .flex()
        .items_center()
        .gap(px(8.0))
        .py(px(12.0))
        .child(Spinner::new().with_size(px(16.0)))
        .child(
            div()
                .text_size(px(12.0))
                .text_color(muted)
                .child(t_global("ask_notes.searching_your_notes")),
        )
}

/// The provider failure alert.
fn error_alert(error: String, cx: &mut Context<AskView>) -> impl IntoElement {
    let theme = cx.theme();
    let danger = theme.colors.danger;
    let _ = cx;
    div()
        .id("ask-error")
        .debug_selector(|| "ask-error".to_string())
        .px(px(10.0))
        .py(px(8.0))
        .rounded(px(6.0))
        .text_size(px(12.0))
        .text_color(danger)
        .child(error)
}

/// The read-aloud failure alert.
fn speak_error_alert(error: String, cx: &mut Context<AskView>) -> impl IntoElement {
    let theme = cx.theme();
    let warning = theme.colors.warning;
    let _ = cx;
    div()
        .id("ask-speak-error")
        .debug_selector(|| "ask-speak-error".to_string())
        .px(px(10.0))
        .py(px(8.0))
        .rounded(px(6.0))
        .text_size(px(12.0))
        .text_color(warning)
        .child(format!(
            "{} {error}",
            t_global("ask_notes.read_aloud_failed")
        ))
}

/// The answer card with the read-aloud control and sources.
fn answer_card(
    view: &AskView,
    answer: &crate::provider::AskAnswer,
    cx: &mut Context<AskView>,
) -> impl IntoElement {
    let theme = cx.theme();
    let foreground = theme.colors.foreground;
    let muted = theme.colors.muted_foreground;
    let border = theme.colors.border;
    let sources = answer.sources.clone();
    let show_no_sources = !answer.had_sources && sources.is_empty();
    let can_speak = !answer.answer.trim().is_empty();
    let answer_text = answer.answer.clone();
    let speaking = view.speaking();

    div()
        .id("ask-answer")
        .debug_selector(|| "ask-answer".to_string())
        .flex()
        .flex_col()
        .gap(px(8.0))
        .p(px(14.0))
        .rounded(px(8.0))
        .border_1()
        .border_color(border)
        .child(
            div()
                .flex()
                .items_start()
                .gap(px(10.0))
                .child(
                    div()
                        .flex_1()
                        .text_size(px(13.0))
                        .text_color(foreground)
                        .child(answer_text),
                )
                .when(can_speak, |element| {
                    element.child(read_aloud_button(view, speaking, cx))
                }),
        )
        .when(!sources.is_empty(), |element| {
            element
                .child(
                    div()
                        .text_size(px(11.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(muted)
                        .child(t_global("ask_notes.sources")),
                )
                .children(source_rows(sources, cx))
        })
        .when(show_no_sources, |element| {
            element.child(div().text_size(px(11.0)).text_color(muted).child(t_global(
                "ask_notes.no_matching_notes_found_the_answer_is_based_on_general_knowl",
            )))
        })
}

/// The read-aloud control; the label switches while speaking.
fn read_aloud_button(
    view: &AskView,
    speaking: bool,
    cx: &mut Context<AskView>,
) -> impl IntoElement {
    let theme = cx.theme();
    let muted = theme.colors.muted_foreground;
    let label = if speaking {
        t_global("ask_notes.playing")
    } else {
        t_global("editor.read_aloud")
    };
    div()
        .id("ask-read-aloud")
        .debug_selector(|| "ask-read-aloud".to_string())
        .px(px(8.0))
        .py(px(3.0))
        .rounded(px(6.0))
        .text_size(px(11.0))
        .text_color(muted)
        .cursor_pointer()
        .child(label)
        .when(!view.speaking(), |element| {
            element.on_click(cx.listener(|view, _, _, cx| view.speak_answer(cx)))
        })
}

/// Render one citation row per source.
fn source_rows(sources: Vec<AskSource>, cx: &mut Context<AskView>) -> Vec<AnyElement> {
    let theme = cx.theme();
    let foreground = theme.colors.foreground;
    let muted = theme.colors.muted_foreground;
    let border = theme.colors.border;
    let _ = cx;
    sources
        .into_iter()
        .enumerate()
        .map(|(index, source)| {
            div()
                .id(format!("ask-source-{index}"))
                .debug_selector(move || format!("ask-source-{index}"))
                .flex()
                .flex_col()
                .gap(px(2.0))
                .p(px(8.0))
                .rounded(px(6.0))
                .border_1()
                .border_color(border)
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(foreground)
                        .child(source.path.clone()),
                )
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(muted)
                        .child(format!("{} ({:.2})", source.snippet, source.score)),
                )
                .into_any_element()
        })
        .collect()
}
