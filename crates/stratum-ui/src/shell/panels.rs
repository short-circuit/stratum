//! Search and query panel lifecycle for the shell.
//!
//! Creating the panel entities, applying deterministic start states and
//! draining their navigation/toast requests live here so [`super::mod`] stays
//! under the repository file-length limit. The panels own their inputs and
//! provider calls; the shell only wires routes and overlays.

use std::sync::Arc;

use gpui_kit::prelude::*;
use gpui_kit::*;

use super::Shell;
use crate::panels::query::{QueryStart, QueryTab, QueryView};
use crate::panels::search::{SearchStart, SearchView};
use crate::panels::{PanelRequest, PanelRequests};
use crate::router::Panel;

/// Deterministic start states for the search and query panels.
///
/// The capture harness fills this from `STRATUM_SEARCH`, `STRATUM_QUERY*` and
/// tag navigation. The shell consumes the matching start when it creates the
/// panel entity, so a later navigation falls back to the default state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PanelStarts {
    /// Search panel start state.
    pub search: SearchStart,
    /// Query panel start state.
    pub query: QueryStart,
}

impl PanelStarts {
    /// Build a start state that submits a search query.
    pub fn search_query(query: impl Into<String>) -> Self {
        Self {
            search: SearchStart {
                query: Some(query.into()),
            },
            query: QueryStart::default(),
        }
    }

    /// Build a start state that opens a query tab and optionally runs it.
    pub fn query(tab: QueryTab, run: bool) -> Self {
        Self {
            search: SearchStart::default(),
            query: QueryStart {
                datalog: None,
                tab: Some(tab),
                run,
            },
        }
    }
}

impl Shell {
    /// Set the search/query start states and recreate their entities.
    ///
    /// Called by the capture harness before the first frame. Recreating the
    /// entities makes a later call take effect deterministically.
    pub fn set_panel_start(&mut self, starts: PanelStarts) {
        self.panel_starts = starts;
        self.search = None;
        self.query = None;
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

    /// Create the active panel entity and drain its pending requests.
    pub(super) fn sync_panels(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.active {
            Panel::Search => self.sync_search(window, cx),
            Panel::Query => self.sync_query(window, cx),
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
}
