//! Related-note suggestions card for the page surface.
//!
//! Mirrors the shipped desktop `SuggestedConnectionsPanel`: a collapsible card
//! with a Find button, the current suggestion list (title, snippet and clamped
//! relevance), per-row navigation and an add-link action that appends
//! `See also: [[title]]` to the page and refreshes the list.
//!
//! [`SuggestionsState`] is a pure state model so the window-free contract tests
//! can pin the transitions; the renderer only dispatches to [`super::PageView`].

use std::sync::Arc;

use gpui_kit::assets::IconName;
use gpui_kit::component::{ActiveTheme, Icon, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use super::PageView;
use crate::provider::{clamp_relevance, ConnectionRow};

/// The related-note suggestion state machine.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum SuggestionsState {
    /// No search has run yet.
    #[default]
    Idle,
    /// A search is in flight.
    Loading,
    /// The search returned these rows (possibly empty).
    Loaded(Vec<ConnectionRow>),
    /// The search or the add-link action failed.
    Error(String),
}

impl SuggestionsState {
    /// The rows when the search completed successfully.
    pub fn rows(&self) -> Option<&[ConnectionRow]> {
        match self {
            SuggestionsState::Loaded(rows) => Some(rows),
            _ => None,
        }
    }

    /// Whether a search is in flight.
    pub fn is_loading(&self) -> bool {
        matches!(self, SuggestionsState::Loading)
    }

    /// The error text, when the last operation failed.
    pub fn error(&self) -> Option<&str> {
        match self {
            SuggestionsState::Error(message) => Some(message),
            _ => None,
        }
    }
}

impl PageView {
    /// Run a related-note search and store the result.
    ///
    /// A second request while one is in flight is ignored so the button cannot
    /// stack searches.
    pub fn find_suggestions(&mut self, cx: &mut Context<Self>) {
        if self.suggestions.is_loading() {
            return;
        }
        self.suggestions = SuggestionsState::Loading;
        cx.notify();

        let provider = Arc::clone(&self.provider);
        let path = self.path.clone();
        cx.spawn(async move |this: WeakEntity<PageView>, cx: &mut AsyncApp| {
            let result = cx
                .background_executor()
                .spawn(async move { provider.suggest_connections(&path) })
                .await;
            let _ = this.update(cx, |view, cx| {
                view.suggestions = match result {
                    Ok(rows) => SuggestionsState::Loaded(rows),
                    Err(message) => SuggestionsState::Error(format!(
                        "{} {message}",
                        t_global("suggested_connections.failed_to_find_connections")
                    )),
                };
                cx.notify();
            });
        })
        .detach();
    }

    /// Append a `See also: [[title]]` block, reload the editor and refresh the
    /// suggestions.
    pub fn add_suggested_link(&mut self, title: String, cx: &mut Context<Self>) {
        let provider = Arc::clone(&self.provider);
        let path = self.path.clone();
        let markdown = format!("{}{}]]", t_global("suggested_connections.see_also"), title);
        cx.spawn(async move |this: WeakEntity<PageView>, cx: &mut AsyncApp| {
            let result = cx
                .background_executor()
                .spawn(async move { provider.insert_block(&path, &markdown) })
                .await;
            let _ = this.update(cx, |view, cx| match result {
                Ok(()) => {
                    view.reload_editor(cx);
                    view.find_suggestions(cx);
                }
                Err(message) => {
                    view.suggestions = SuggestionsState::Error(format!(
                        "{} {message}",
                        t_global("suggested_connections.failed_to_add_link")
                    ));
                    cx.notify();
                }
            });
        })
        .detach();
    }
}

/// Render the suggestions card.
pub fn suggestions_section(view: &PageView, cx: &mut Context<PageView>) -> impl IntoElement {
    let theme = cx.theme();
    let border = theme.colors.border;
    let muted = theme.colors.muted_foreground;
    let foreground = theme.colors.foreground;
    let link = theme.colors.link;
    let panel = theme.colors.secondary;
    let danger = theme.colors.danger;
    let accent = theme.colors.accent;
    let accent_foreground = theme.colors.accent_foreground;

    let mut section = div()
        .border_t_1()
        .border_color(border)
        .bg(panel)
        .flex()
        .flex_col()
        .child(
            div()
                .id("page-suggestions-toggle")
                .flex()
                .items_center()
                .justify_between()
                .px(px(12.0))
                .py(px(5.0))
                .cursor_pointer()
                .child(
                    div()
                        .text_size(px(11.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(muted)
                        .child(t_global("suggested_connections.suggested_connections")),
                )
                .child(
                    Icon::new(if view.suggestions_open() {
                        IconName::ChevronDown
                    } else {
                        IconName::ChevronRight
                    })
                    .with_size(px(12.0))
                    .text_color(muted),
                )
                .on_click(cx.listener(|view, _, _, cx| view.toggle_suggestions(cx))),
        );

    if !view.suggestions_open() {
        return section;
    }

    let loading = view.suggestions().is_loading();
    section = section.child(
        div().px(px(12.0)).pb(px(4.0)).child(
            div()
                .id("page-suggestions-find")
                .px(px(10.0))
                .py(px(3.0))
                .rounded(px(6.0))
                .bg(accent)
                .text_color(accent_foreground)
                .text_size(px(11.0))
                .cursor_pointer()
                .child(if loading {
                    t_global("suggested_connections.searching")
                } else {
                    t_global("suggested_connections.find")
                })
                .on_click(cx.listener(|view, _, _, cx| view.find_suggestions(cx))),
        ),
    );

    if let Some(message) = view.suggestions().error() {
        section = section.child(
            div()
                .px(px(12.0))
                .pb(px(6.0))
                .text_size(px(11.0))
                .text_color(danger)
                .child(message.to_string()),
        );
        return section;
    }

    match view.suggestions().rows() {
        Some([]) => section.child(
            div()
                .px(px(12.0))
                .pb(px(6.0))
                .text_size(px(11.0))
                .text_color(muted)
                .child(t_global("suggested_connections.no_connections_found")),
        ),
        Some(rows) => {
            let mut list = div().flex().flex_col().px(px(12.0)).pb(px(4.0));
            for row in rows {
                list = list.child(suggestion_row(row, foreground, link, muted, cx));
            }
            section.child(list)
        }
        None => section.child(
            div()
                .px(px(12.0))
                .pb(px(6.0))
                .text_size(px(11.0))
                .text_color(muted)
                .child(t_global(
                    "suggested_connections.click_find_to_discover_related_notes",
                )),
        ),
    }
}

/// One suggestion row with navigation, add-link and relevance.
fn suggestion_row(
    row: &ConnectionRow,
    foreground: Hsla,
    link: Hsla,
    muted: Hsla,
    cx: &mut Context<PageView>,
) -> impl IntoElement {
    let page_path = row.page_path.clone();
    let title = row.title.clone();
    let add_title = row.title.clone();
    let relevance = clamp_relevance(row.score);

    div()
        .id(format!("page-suggestion-{}", row.page_path))
        .flex()
        .flex_col()
        .gap(px(2.0))
        .px(px(4.0))
        .py(px(3.0))
        .rounded(px(4.0))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .id(format!("page-suggestion-open-{}", row.page_path))
                        .text_size(px(11.0))
                        .text_color(link)
                        .cursor_pointer()
                        .child(title)
                        .on_click(cx.listener(move |view, _, _, _cx| {
                            view.navigate_to(&page_path);
                        })),
                )
                .child(
                    div()
                        .id(format!("page-suggestion-add-{}", row.page_path))
                        .flex()
                        .items_center()
                        .gap(px(3.0))
                        .px(px(4.0))
                        .rounded(px(3.0))
                        .text_size(px(9.0))
                        .text_color(muted)
                        .cursor_pointer()
                        .child(Icon::new(IconName::Link).with_size(px(12.0)))
                        .child(t_global("suggested_connections.add_wiki_link_to_this_page"))
                        .on_click(cx.listener(move |view, _, _, cx| {
                            view.add_suggested_link(add_title.clone(), cx);
                        })),
                ),
        )
        .when(!row.snippet.is_empty(), |element| {
            element.child(
                div()
                    .text_size(px(10.0))
                    .text_color(foreground.opacity(0.8))
                    .child(row.snippet.clone()),
            )
        })
        .child(div().text_size(px(9.0)).text_color(muted).child(format!(
            "{} {relevance}%",
            t_global("suggested_connections.relevance")
        )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::core::prelude::v1::test;
    use stratum_i18n::{t, Locale};

    fn row(title: &str, score: f64) -> ConnectionRow {
        ConnectionRow {
            title: title.to_string(),
            page_path: format!("pages/{title}.md"),
            score,
            snippet: String::new(),
        }
    }

    #[test]
    fn suggestion_state_exposes_only_loaded_rows() {
        let idle = SuggestionsState::Idle;
        assert!(idle.rows().is_none());
        assert!(!idle.is_loading());
        assert!(idle.error().is_none());

        let loading = SuggestionsState::Loading;
        assert!(loading.is_loading());
        assert!(loading.rows().is_none());

        let loaded = SuggestionsState::Loaded(vec![row("Reading List", 82.0)]);
        assert_eq!(loaded.rows().map(|rows| rows.len()), Some(1));
        assert!(!loaded.is_loading());

        let error = SuggestionsState::Error("boom".to_string());
        assert_eq!(error.error(), Some("boom"));
        assert!(error.rows().is_none());
    }

    #[test]
    fn suggestion_keys_are_localized_in_every_catalog() {
        let keys = [
            "suggested_connections.add_wiki_link_to_this_page",
            "suggested_connections.click_find_to_discover_related_notes",
            "suggested_connections.failed_to_add_link",
            "suggested_connections.failed_to_find_connections",
            "suggested_connections.find",
            "suggested_connections.no_connections_found",
            "suggested_connections.relevance",
            "suggested_connections.searching",
            "suggested_connections.see_also",
            "suggested_connections.suggested_connections",
        ];
        for locale in Locale::ALL {
            for key in keys {
                assert_ne!(t(locale, key), key, "{key} missing in {}", locale.code());
                assert!(super::super::PAGE_I18N_KEYS.contains(&key));
            }
        }
    }

    #[test]
    fn see_also_prefix_composes_the_shipped_wiki_link() {
        let prefix = t_global("suggested_connections.see_also");
        assert_eq!(
            format!("{prefix}Reading List]]"),
            "See also: [[Reading List]]"
        );
    }
}
