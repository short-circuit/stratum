//! Search, query, Ask Notes, Templates, Flashcards, Reminders and Plugins panel
//! lifecycle for the shell.
//!
//! Creating the panel entities, applying deterministic start states and
//! draining their navigation/toast requests live here so [`super::mod`] stays
//! under the repository file-length limit. The panels own their inputs and
//! provider calls; the shell only wires routes and overlays.

use std::sync::Arc;

use gpui_kit::prelude::*;
use gpui_kit::*;

use super::Shell;
use crate::panels::ask::{AskStart, AskView};
use crate::panels::canvas::{CanvasStart, CanvasView};
use crate::panels::flashcards::{FlashcardsStart, FlashcardsView};
use crate::panels::graph::{GraphStart, GraphView};
use crate::panels::journal::JournalView;
use crate::panels::kanban::{KanbanStart, KanbanView};
use crate::panels::plugins::{PluginsStart, PluginsView};
use crate::panels::query::{QueryStart, QueryTab, QueryView};
use crate::panels::reminders::{RemindersStart, RemindersView};
use crate::panels::search::{SearchStart, SearchView};
use crate::panels::settings::{SettingsStart, SettingsView};
use crate::panels::templates::TemplatesView;
use crate::panels::{PanelRequest, PanelRequests};
use crate::router::Panel;

/// Deterministic start states for the native panels.
///
/// The capture harness fills this from `STRATUM_SEARCH`, `STRATUM_QUERY*`,
/// `STRATUM_ASK` and `STRATUM_FLASHCARD_REVEAL`, plus tag navigation. The shell
/// consumes the matching start when it creates the panel entity, so a later
/// navigation falls back to the default state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PanelStarts {
    /// Search panel start state.
    pub search: SearchStart,
    /// Query panel start state.
    pub query: QueryStart,
    /// Ask Notes panel start state.
    pub ask: AskStart,
    /// Flashcards panel start state.
    pub flashcards: FlashcardsStart,
    /// Reminders panel start state.
    pub reminders: RemindersStart,
    /// Plugins panel start state.
    pub plugins: PluginsStart,
    /// Graph panel start state.
    pub graph: GraphStart,
    /// Whiteboards panel start state.
    pub canvas: CanvasStart,
    /// Settings panel start state.
    pub settings: SettingsStart,
    /// Kanban panel start state.
    pub kanban: KanbanStart,
}

impl PanelStarts {
    /// Build a start state that submits a search query.
    pub fn search_query(query: impl Into<String>) -> Self {
        Self {
            search: SearchStart {
                query: Some(query.into()),
            },
            ..Self::default()
        }
    }

    /// Build a start state that opens a query tab and optionally runs it.
    pub fn query(tab: QueryTab, run: bool) -> Self {
        Self {
            query: QueryStart {
                datalog: None,
                tab: Some(tab),
                run,
            },
            ..Self::default()
        }
    }
}

impl Shell {
    /// Set the panel start states and recreate their entities.
    ///
    /// Called by the capture harness before the first frame. Recreating the
    /// entities makes a later call take effect deterministically.
    pub fn set_panel_start(&mut self, starts: PanelStarts) {
        self.panel_starts = starts;
        self.search = None;
        self.query = None;
        self.ask = None;
        self.flashcards = None;
        self.reminders = None;
        self.plugins = None;
        self.graph = None;
        self.whiteboards = None;
        self.settings = None;
        self.kanban = None;
    }

    /// Set the Whiteboards start state and recreate its entity.
    ///
    /// Used by `/whiteboards?...` deep links and the `STRATUM_WHITEBOARD*`
    /// capture flags.
    pub fn set_canvas_start(&mut self, start: CanvasStart) {
        self.panel_starts.canvas = start;
        self.whiteboards = None;
    }

    /// Set the search panel's start query (used by tag navigation).
    pub fn set_search_start(&mut self, query: Option<String>) {
        self.panel_starts.search = SearchStart { query };
        self.search = None;
    }

    /// The live search entity, if the panel has created one.
    pub fn search_view(&self) -> Option<&Entity<SearchView>> {
        self.search.as_ref()
    }

    /// The live query entity, if the panel has created one.
    pub fn query_view(&self) -> Option<&Entity<QueryView>> {
        self.query.as_ref()
    }

    /// The live Ask Notes entity, if the panel has created one.
    pub fn ask_view(&self) -> Option<&Entity<AskView>> {
        self.ask.as_ref()
    }

    /// The live Templates entity, if the panel has created one.
    pub fn templates_view(&self) -> Option<&Entity<TemplatesView>> {
        self.templates.as_ref()
    }

    /// The live Flashcards entity, if the panel has created one.
    pub fn flashcards_view(&self) -> Option<&Entity<FlashcardsView>> {
        self.flashcards.as_ref()
    }

    /// The live Reminders entity, if the panel has created one.
    pub fn reminders_view(&self) -> Option<&Entity<RemindersView>> {
        self.reminders.as_ref()
    }

    /// The live Plugins entity, if the panel has created one.
    pub fn plugins_view(&self) -> Option<&Entity<PluginsView>> {
        self.plugins.as_ref()
    }

    /// The live graph entity, if the panel has created one.
    pub fn graph_view(&self) -> Option<&Entity<GraphView>> {
        self.graph.as_ref()
    }

    /// The live Whiteboards entity, if the panel has created one.
    pub fn canvas_view(&self) -> Option<&Entity<CanvasView>> {
        self.whiteboards.as_ref()
    }

    /// The live Settings entity, if the panel has created one.
    pub fn settings_view(&self) -> Option<&Entity<SettingsView>> {
        self.settings.as_ref()
    }

    /// The live Kanban entity, if the panel has created one.
    pub fn kanban_view(&self) -> Option<&Entity<KanbanView>> {
        self.kanban.as_ref()
    }

    /// Set the Kanban start state and recreate its entity.
    ///
    /// Used by the `STRATUM_KANBAN*` capture flags.
    pub fn set_kanban_start(&mut self, start: KanbanStart) {
        self.panel_starts.kanban = start;
        self.kanban = None;
    }

    /// Set the Settings tab start state and recreate its entity.
    ///
    /// Used by the `STRATUM_SETTINGS_TAB` capture flag.
    pub fn set_settings_start(&mut self, start: SettingsStart) {
        self.panel_starts.settings = start;
        self.settings = None;
    }

    /// Create the active panel entity and drain its pending requests.
    pub(super) fn sync_panels(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.active {
            Panel::Search => self.sync_search(window, cx),
            Panel::Query => self.sync_query(window, cx),
            Panel::AskNotes => self.sync_ask(window, cx),
            Panel::Templates => self.sync_templates(window, cx),
            Panel::Flashcards => self.sync_flashcards(cx),
            Panel::Reminders => self.sync_reminders(window, cx),
            Panel::Plugins => self.sync_plugins(cx),
            Panel::Graph => self.sync_graph(window, cx),
            Panel::Whiteboards => self.sync_canvas(window, cx),
            Panel::Settings => self.sync_settings(window, cx),
            Panel::Kanban => self.sync_kanban(window, cx),
            _ => {}
        }
    }

    /// Create or reuse the search entity, then drain navigation and toasts.
    fn sync_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.search.is_none() {
            let provider = Arc::clone(&self.provider);
            let start = std::mem::take(&mut self.panel_starts.search);
            self.search = Some(cx.new(move |cx| SearchView::new(provider, start, window, cx)));
        }
        let Some(search) = self.search.clone() else {
            return;
        };
        self.drain_panel(&search, cx);
    }

    /// Create or reuse the query entity, then drain navigation and toasts.
    fn sync_query(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.query.is_none() {
            let provider = Arc::clone(&self.provider);
            let start = std::mem::take(&mut self.panel_starts.query);
            self.query = Some(cx.new(move |cx| QueryView::new(provider, start, window, cx)));
        }
        let Some(query) = self.query.clone() else {
            return;
        };
        self.drain_panel(&query, cx);
    }

    /// Create or reuse the Ask Notes entity, then drain navigation and toasts.
    fn sync_ask(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.ask.is_none() {
            let provider = Arc::clone(&self.provider);
            let start = std::mem::take(&mut self.panel_starts.ask);
            self.ask = Some(cx.new(move |cx| AskView::new(provider, start, window, cx)));
        }
        let Some(ask) = self.ask.clone() else {
            return;
        };
        self.drain_panel(&ask, cx);
    }

    /// Create or reuse the Templates entity, then drain navigation and toasts.
    fn sync_templates(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.templates.is_none() {
            let provider = Arc::clone(&self.provider);
            self.templates = Some(cx.new(move |cx| TemplatesView::new(provider, window, cx)));
        }
        let Some(templates) = self.templates.clone() else {
            return;
        };
        self.drain_panel(&templates, cx);
    }

    /// Create or reuse the Flashcards entity, then drain navigation and toasts.
    fn sync_flashcards(&mut self, cx: &mut Context<Self>) {
        if self.flashcards.is_none() {
            let provider = Arc::clone(&self.provider);
            let start = std::mem::take(&mut self.panel_starts.flashcards);
            self.flashcards = Some(cx.new(move |cx| FlashcardsView::new(provider, start, cx)));
        }
        let Some(flashcards) = self.flashcards.clone() else {
            return;
        };
        self.drain_panel(&flashcards, cx);
    }

    /// Create or reuse the Reminders entity, then drain navigation, block-open
    /// and toast requests.
    fn sync_reminders(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.reminders.is_none() {
            let provider = Arc::clone(&self.provider);
            let start = std::mem::take(&mut self.panel_starts.reminders);
            self.reminders =
                Some(cx.new(move |cx| RemindersView::new(provider, start, window, cx)));
        }
        let Some(reminders) = self.reminders.clone() else {
            return;
        };
        self.drain_panel(&reminders, cx);
    }

    /// Create or reuse the Plugins entity, then drain navigation and toasts.
    fn sync_plugins(&mut self, cx: &mut Context<Self>) {
        if self.plugins.is_none() {
            let provider = Arc::clone(&self.provider);
            let start = std::mem::take(&mut self.panel_starts.plugins);
            self.plugins = Some(cx.new(move |cx| PluginsView::new(provider, start, cx)));
        }
        let Some(plugins) = self.plugins.clone() else {
            return;
        };
        self.drain_panel(&plugins, cx);
    }

    /// Create or reuse the graph entity, then drain navigation requests.
    fn sync_graph(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.graph.is_none() {
            let provider = Arc::clone(&self.provider);
            let mut start = std::mem::take(&mut self.panel_starts.graph);
            // The mobile shell starts on the 2D graph, matching the shipped
            // mobile surface; the settings toggle can still switch to 3D.
            if self.mobile {
                start.use_3d = Some(false);
            }
            self.graph = Some(cx.new(move |cx| GraphView::new(provider, start, window, cx)));
        }
        let Some(graph) = self.graph.clone() else {
            return;
        };
        self.drain_panel(&graph, cx);
    }

    /// Create or reuse the Whiteboards entity, then drain its requests.
    fn sync_canvas(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.whiteboards.is_none() {
            let provider = Arc::clone(&self.provider);
            let start = std::mem::take(&mut self.panel_starts.canvas);
            self.whiteboards = Some(cx.new(move |cx| CanvasView::new(provider, start, window, cx)));
        }
        let Some(canvas) = self.whiteboards.clone() else {
            return;
        };
        self.drain_panel(&canvas, cx);
    }

    /// Create or reuse the Settings entity, then drain its requests.
    fn sync_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.settings.is_none() {
            let provider = Arc::clone(&self.provider);
            let start = std::mem::take(&mut self.panel_starts.settings);
            self.settings = Some(cx.new(move |cx| SettingsView::new(provider, start, window, cx)));
        }
        let Some(settings) = self.settings.clone() else {
            return;
        };
        self.drain_panel(&settings, cx);
    }

    /// Create or reuse the Kanban entity, then drain its requests.
    fn sync_kanban(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.kanban.is_none() {
            let provider = Arc::clone(&self.provider);
            let start = std::mem::take(&mut self.panel_starts.kanban);
            self.kanban = Some(cx.new(move |cx| KanbanView::new(provider, start, window, cx)));
        }
        let Some(kanban) = self.kanban.clone() else {
            return;
        };
        self.drain_panel(&kanban, cx);
    }

    /// Drain a hosted panel's pending navigation, block-open and toast
    /// requests into shell state.
    fn drain_panel<T: PanelRequests + 'static>(
        &mut self,
        entity: &Entity<T>,
        cx: &mut Context<Self>,
    ) {
        let navigation = entity.update(cx, |panel, _| {
            panel.pending_navigation().map(str::to_string)
        });
        if let Some(target) = navigation {
            entity.update(cx, |panel, _| panel.clear_pending_navigation());
            self.drain_panel_requests(PanelRequest::Navigate(target));
        }

        let open = entity.update(cx, |panel, _| panel.take_pending_open_block());
        if let Some((page_path, block_id)) = open {
            self.drain_panel_requests(PanelRequest::OpenBlock {
                page_path,
                block_id,
            });
        }

        let toast = entity.update(cx, |panel, _| panel.take_pending_toast());
        if let Some(message) = toast {
            self.drain_panel_requests(PanelRequest::Toast(message));
        }
    }

    /// Apply one panel request to shell navigation/overlays.
    fn drain_panel_requests(&mut self, request: PanelRequest) {
        match request {
            PanelRequest::Navigate(target) => self.handle_editor_navigation(&target),
            PanelRequest::Toast(message) => self.push_toast(message),
            PanelRequest::OpenBlock {
                page_path,
                block_id,
            } => self.open_page_with_highlight(page_path, block_id),
        }
    }
    /// Create the journal entity lazily, forward shell read-mode changes and
    /// route a child editor's pending navigation.
    ///
    /// The journal header owns its own read-mode toggle, so the shell only
    /// forwards a change when its own read-mode flag differs from the last value
    /// it pushed; otherwise the header toggle would be overwritten every frame.
    pub(super) fn sync_journal(&mut self, cx: &mut Context<Self>) {
        if self.journal.is_none() {
            let provider = Arc::clone(&self.provider);
            let start_date = self.journal_start_date.clone();
            let calendar_open = self.journal_calendar;
            let read_mode = self.read_mode;
            self.journal = Some(cx.new(move |cx| {
                JournalView::new(provider, start_date, calendar_open, read_mode, cx)
            }));
            self.journal_read_mode = read_mode;
        }

        let journal = self.journal.clone();
        let Some(journal) = journal else {
            return;
        };

        if self.read_mode != self.journal_read_mode {
            let read_mode = self.read_mode;
            journal.update(cx, |view, cx| view.set_read_mode(read_mode, cx));
            self.journal_read_mode = read_mode;
        }

        let navigation =
            journal.update(cx, |view, _| view.pending_navigation().map(str::to_string));
        if let Some(target) = navigation {
            journal.update(cx, |view, _| view.clear_pending_navigation());
            self.handle_editor_navigation(&target);
        }
    }
}
