//! Search panel header, query bar and status area.
//!
//! Split out of [`super::SearchView::render`] to keep the render function small.
//! The result rows live in [`super::rows`]; this module owns the static chrome
//! around them.

use gpui_kit::component::input::Input;
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::{ActiveTheme, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use super::model::SearchStatus;
use super::rows;
use super::SearchView;

/// Render the full search surface.
pub fn body(view: &SearchView, cx: &mut Context<SearchView>) -> impl IntoElement {
    let theme = cx.theme();
    div()
        .flex()
        .flex_col()
        .size_full()
        .bg(theme.colors.background)
        .text_color(theme.colors.foreground)
        .child(header(cx))
        .child(content(view, cx))
}

/// The panel title bar.
fn header(cx: &mut Context<SearchView>) -> impl IntoElement {
    let theme = cx.theme();
    div()
        .flex()
        .items_center()
        .px(px(16.0))
        .py(px(10.0))
        .border_b_1()
        .border_color(theme.colors.border)
        .child(
            div()
                .text_size(px(16.0))
                .font_weight(FontWeight::BOLD)
                .child(t_global("search.search_blocks")),
        )
}

/// The scrollable search body: query bar, rebuild action, status and rows.
fn content(view: &SearchView, cx: &mut Context<SearchView>) -> impl IntoElement {
    div()
        .id("search-body")
        .flex()
        .flex_col()
        .flex_1()
        .overflow_y_scroll()
        .p(px(24.0))
        .gap(px(8.0))
        .child(query_bar(view, cx))
        .child(rebuild_row(view, cx))
        .child(status(view, cx))
        .children(rows::result_rows(view, cx))
        .when(show_empty(view), |element| element.child(empty_result(cx)))
}

/// The query field and submit button.
fn query_bar(view: &SearchView, cx: &mut Context<SearchView>) -> impl IntoElement {
    let theme = cx.theme();
    let submit_label = if matches!(view.status(), SearchStatus::Searching) {
        "...".to_string()
    } else {
        t_global("common.search")
    };
    div()
        .flex()
        .items_center()
        .gap(px(8.0))
        .child(div().flex_1().child(Input::new(&view.input)))
        .child(
            div()
                .id("search-submit")
                .debug_selector(|| "search-submit".to_string())
                .px(px(12.0))
                .py(px(5.0))
                .rounded(px(6.0))
                .bg(theme.colors.accent)
                .text_color(theme.colors.accent_foreground)
                .text_size(px(12.0))
                .cursor_pointer()
                .child(submit_label)
                .on_click(cx.listener(|view, _, _, cx| view.submit(cx))),
        )
}

/// The rebuild-index action with its caption.
fn rebuild_row(view: &SearchView, cx: &mut Context<SearchView>) -> impl IntoElement {
    let theme = cx.theme();
    let muted = theme.colors.muted_foreground;
    let rebuild_label = if view.indexing() {
        t_global("search.indexing")
    } else {
        t_global("search.rebuild_search_index")
    };
    div()
        .flex()
        .items_center()
        .gap(px(10.0))
        .child(
            div()
                .id("search-rebuild")
                .debug_selector(|| "search-rebuild".to_string())
                .text_size(px(11.0))
                .text_color(muted)
                .cursor_pointer()
                .child(rebuild_label)
                .on_click(cx.listener(|view, _, _, cx| view.rebuild_index(cx))),
        )
        .when_some(
            view.index_message().map(str::to_string),
            |element, message| {
                element.child(div().text_size(px(11.0)).text_color(muted).child(message))
            },
        )
}

/// The searching spinner and failure alert.
fn status(view: &SearchView, cx: &mut Context<SearchView>) -> impl IntoElement {
    let theme = cx.theme();
    let muted = theme.colors.muted_foreground;
    let danger = theme.colors.danger;
    let failure = match view.status() {
        SearchStatus::Failed(message) => Some(message.clone()),
        _ => None,
    };
    div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .when(
            matches!(view.status(), SearchStatus::Searching),
            |element| {
                element.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .child(Spinner::new().with_size(px(14.0)))
                        .child(
                            div()
                                .text_size(px(11.0))
                                .text_color(muted)
                                .child(t_global("search.search_blocks")),
                        ),
                )
            },
        )
        .when_some(failure, |element, message| {
            element.child(div().text_size(px(12.0)).text_color(danger).child(message))
        })
}

/// Whether the empty-result hint should be shown.
fn show_empty(view: &SearchView) -> bool {
    matches!(view.status(), SearchStatus::Done)
        && view.results().is_empty()
        && view.last_query().is_some_and(|query| !query.is_empty())
}

/// The localized empty-result hint.
fn empty_result(cx: &mut Context<SearchView>) -> impl IntoElement {
    let theme = cx.theme();
    div()
        .text_size(px(12.0))
        .text_color(theme.colors.muted_foreground)
        .child(t_global("search.no_results_found"))
}
