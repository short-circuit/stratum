//! Panel bodies hosted by the shell.
//!
//! The shell milestone owns the chrome. Non-page panels are rendered as titled
//! placeholders so navigation can be exercised end to end; the page panel
//! renders the real [`crate::panels::page::PageView`] when the shell has created
//! one. Without a page entity (for example the window-free shell tests) the page
//! panel falls back to the shipped empty-state hint.

use gpui_kit::assets::IconName;
use gpui_kit::component::{ActiveTheme, Icon, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

pub mod editor;
pub mod journal;
pub mod page;
pub mod query;
pub mod search;
pub mod voice;

use crate::panels::journal::JournalView;
use crate::panels::page::PageView;
use crate::panels::query::QueryView;
use crate::panels::search::SearchView;
use crate::router::Panel;
use crate::shell::Shell;

/// A navigation or toast request a hosted panel wants the shell to consume.
///
/// Panels never touch the router or toaster directly; they record a pending
/// request and the shell drains it after the frame. `OpenBlock` is reserved for
/// panels that navigate to a block anchor (the reminder "open" action).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PanelRequest {
    /// Navigate to a page path.
    Navigate(String),
    /// Show a success toast.
    Toast(String),
    /// Open a page and reveal a block.
    OpenBlock {
        /// Vault-relative page path.
        page_path: String,
        /// Block id to reveal.
        block_id: String,
    },
}

/// Requests a hosted panel exposes to the shell.
///
/// Implementations are infallible accessors; the shell drains the pending
/// value once per frame and clears it. Panels without block anchors use the
/// default [`PanelRequests::take_pending_open_block`].
pub trait PanelRequests {
    /// The page path the panel wants to open, if any.
    fn pending_navigation(&self) -> Option<&str>;
    /// Clear the pending navigation.
    fn clear_pending_navigation(&mut self);
    /// The toast message the panel wants to show, if any.
    fn pending_toast(&self) -> Option<&str>;
    /// Take the pending toast message.
    fn take_pending_toast(&mut self) -> Option<String>;
    /// Take a pending page+block navigation, if any.
    fn take_pending_open_block(&mut self) -> Option<(String, String)> {
        None
    }
}

/// Whether a panel type renders its own header instead of the shell chrome
/// header.
///
/// Page, Journal, Search and Query own the full content column (title and
/// controls), matching the shipped routed panels. The shell additionally gates
/// on the live entity existing, so the placeholder fallback keeps a title.
pub fn panel_owns_header(panel: Panel) -> bool {
    matches!(
        panel,
        Panel::Page | Panel::Journal | Panel::Search | Panel::Query
    )
}

/// Icon used by a panel placeholder.
pub fn panel_icon(panel: Panel) -> IconName {
    match panel {
        Panel::Journal => IconName::Calendar,
        Panel::Page => IconName::FileText,
        Panel::Search => IconName::Search,
        Panel::Query => IconName::Code,
        Panel::AskNotes => IconName::Sparkles,
        Panel::Templates => IconName::FileText,
        Panel::Flashcards => IconName::CircleQuestionMark,
        Panel::Kanban => IconName::ListChecks,
        Panel::Reminders => IconName::AlarmClock,
        Panel::Whiteboards => IconName::PenTool,
        Panel::Graph => IconName::Network,
        Panel::Plugins => IconName::Puzzle,
        Panel::Settings => IconName::Settings,
    }
}

/// Render the active panel body.
///
/// `page`, `journal`, `search` and `query` are the live entities for the open
/// panel. All are `None` in the window-free shell tests, which keeps the panels
/// on their shipped stubs.
pub fn panel_view(
    panel: Panel,
    page_path: Option<&str>,
    page: Option<Entity<PageView>>,
    journal: Option<Entity<JournalView>>,
    search: Option<Entity<SearchView>>,
    query: Option<Entity<QueryView>>,
    cx: &mut Context<Shell>,
) -> impl IntoElement {
    if panel == Panel::Page {
        if let Some(page) = page {
            return div().size_full().child(page);
        }
    }
    if panel == Panel::Journal {
        if let Some(journal) = journal {
            return div().size_full().child(journal);
        }
    }
    if panel == Panel::Search {
        if let Some(search) = search {
            return div().size_full().child(search);
        }
    }
    if panel == Panel::Query {
        if let Some(query) = query {
            return div().size_full().child(query);
        }
    }
    let theme = cx.theme();
    let foreground = theme.colors.foreground;
    let muted = theme.colors.muted_foreground;

    let title = t_global(panel.title_key());
    let detail = if panel == Panel::Page {
        page_path.map(str::to_string)
    } else {
        None
    };

    div()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .size_full()
        .gap(px(12.0))
        .px(px(24.0))
        .child(
            Icon::new(panel_icon(panel))
                .with_size(px(48.0))
                .text_color(muted),
        )
        .child(
            div()
                .text_size(px(18.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(foreground)
                .child(title),
        )
        .when(panel == Panel::Page, |element| {
            element.child(
                div()
                    .text_size(px(13.0))
                    .text_color(muted)
                    .child(detail.unwrap_or_else(|| {
                        t_global("page.select_a_page_from_the_sidebar_or_create_a_new_one")
                    })),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    // `gpui_kit::*` re-exports gpui's `test` attribute macro, which shadows
    // the built-in attribute. Import the built-in explicitly so these plain
    // unit tests stay cheap (no GPUI app context is created).
    use ::core::prelude::v1::test;

    #[test]
    fn every_panel_has_an_icon() {
        for panel in Panel::ALL {
            let icon = panel_icon(panel);
            assert!(!icon.path().is_empty(), "{}", panel.id());
        }
    }
}
