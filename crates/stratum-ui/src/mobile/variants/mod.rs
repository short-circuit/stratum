//! One mobile variant per shipped panel.
//!
//! Every variant wraps the live entity the shell lifecycle created
//! (`sync_panels`, `sync_journal`, `sync_page`) instead of rewriting the
//! desktop view. The variant only adds the mobile-specific chrome: the journal
//! calendar sheet, the Ask keyboard padding, the Query sticky run action and the
//! Settings tab accordion.

use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::mobile::insets::MobileFrame;
use crate::router::Panel;
use crate::shell::Shell;

pub mod ask;
pub mod flashcards;
pub mod graph;
pub mod journal;
pub mod kanban;
pub mod page;
pub mod plugins;
pub mod query;
pub mod reminders;
pub mod search;
pub mod settings;
pub mod templates;
pub mod whiteboards;

/// Render the mobile variant for `panel`.
pub fn render(
    panel: Panel,
    shell: &Shell,
    window: &mut Window,
    frame: &MobileFrame,
    cx: &mut Context<Shell>,
) -> AnyElement {
    match panel {
        Panel::Journal => journal::render(shell, frame, cx),
        Panel::Page => page::render(shell, frame, cx),
        Panel::Search => search::render(shell, frame, cx),
        Panel::Query => query::render(shell, frame, cx),
        Panel::AskNotes => ask::render(shell, frame, cx),
        Panel::Templates => templates::render(shell, frame, cx),
        Panel::Flashcards => flashcards::render(shell, frame, cx),
        Panel::Kanban => kanban::render(shell, frame, cx),
        Panel::Reminders => reminders::render(shell, frame, cx),
        Panel::Whiteboards => whiteboards::render(shell, frame, cx),
        Panel::Graph => graph::render(shell, frame, cx),
        Panel::Plugins => plugins::render(shell, frame, cx),
        Panel::Settings => settings::render(shell, window, frame, cx),
    }
}

/// The shipped placeholder for a panel without a live entity.
///
/// The window-free shell tests and the capture harness before the first panel
/// sync use this path; it reuses the desktop placeholder so navigation stays
/// legible.
pub(super) fn placeholder(panel: Panel, cx: &mut Context<Shell>) -> AnyElement {
    crate::panels::panel_view(
        panel,
        None,
        crate::panels::PanelEntities {
            page: None,
            journal: None,
            search: None,
            query: None,
            ask: None,
            templates: None,
            flashcards: None,
            reminders: None,
            plugins: None,
            graph: None,
            whiteboards: None,
            settings: None,
            kanban: None,
        },
        cx,
    )
    .into_any_element()
}
