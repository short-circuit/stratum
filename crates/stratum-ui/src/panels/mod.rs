//! Panel bodies hosted by the shell.
//!
//! The shell milestone owns the chrome. Non-editor panels are rendered as
//! titled placeholders so navigation can be exercised end to end; the editor
//! panel renders the real [`crate::panels::editor::EditorView`] when the shell
//! has created one. Without an editor entity (for example the window-free shell
//! tests) the page panel falls back to the shipped empty-state hint.

use gpui_kit::assets::IconName;
use gpui_kit::component::{ActiveTheme, Icon, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

pub mod editor;
pub mod journal;
pub mod voice;

use crate::panels::editor::EditorView;
use crate::panels::journal::JournalView;
use crate::router::Panel;
use crate::shell::Shell;

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
/// `editor` is the live editor entity for the open page and `journal` is the
/// live journal entity. Both are `None` in the window-free shell tests, which
/// keeps the panels on their shipped stubs.
pub fn panel_view(
    panel: Panel,
    page_path: Option<&str>,
    editor: Option<Entity<EditorView>>,
    journal: Option<Entity<JournalView>>,
    cx: &mut Context<Shell>,
) -> impl IntoElement {
    if panel == Panel::Page {
        if let Some(editor) = editor {
            return div().size_full().child(editor);
        }
    }
    if panel == Panel::Journal {
        if let Some(journal) = journal {
            return div().size_full().child(journal);
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
