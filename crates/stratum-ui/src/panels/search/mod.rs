//! Native search panel.
//!
//! [`SearchView`] ports the shipped desktop `SearchPanel`: a query field with
//! Enter submission, a tag/full-text router, a rebuild-index action and result
//! rows that navigate to the matched page. Provider calls stay behind the
//! [`crate::provider::PanelData`] seam; this module only owns presentation and
//! the pending navigation/toast requests the shell consumes.
//!
//! [`SEARCH_I18N_KEYS`] is the release gate list for every string this surface
//! renders.

use std::sync::Arc;

use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use crate::panels::PanelRequests;
use crate::provider::{SearchHit, ShellData};

pub mod model;
pub mod rows;
pub mod toolbar;

use model::{classify_query, SearchMode, SearchStatus};

/// Every translation key the search surface renders.
///
/// The contract test walks this list against all eight catalogs, so adding a
/// visible string without a catalog entry fails the build instead of leaking a
/// raw key into the UI.
pub const SEARCH_I18N_KEYS: &[&str] = &[
    "common.search",
    "search._score",
    "search.failed",
    "search.indexing",
    "search.no_results_found",
    "search.rebuild_search_index",
    "search.search_blocks",
    "search.search_blocks_prefix_with_to_search_tags",
];

/// Deterministic start state for the search panel.
///
/// `STRATUM_SEARCH=<query>` uses this to submit a query during construction so
/// the capture harness can render real results without synthesising keystrokes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SearchStart {
    /// Query to submit immediately after construction.
    pub query: Option<String>,
}

/// The search panel entity.
pub struct SearchView {
    provider: Arc<dyn ShellData>,
    input: Entity<InputState>,
    _input_subscription: Subscription,
    results: Vec<SearchHit>,
    status: SearchStatus,
    last_query: Option<String>,
    indexing: bool,
    index_message: Option<String>,
    pending_navigation: Option<String>,
    pending_toast: Option<String>,
}

impl SearchView {
    /// Build the search panel and submit `start.query` when present.
    pub fn new(
        provider: Arc<dyn ShellData>,
        start: SearchStart,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let placeholder = t_global("search.search_blocks_prefix_with_to_search_tags");
        let initial = start.query.clone().unwrap_or_default();
        let input = cx.new(move |cx| {
            InputState::new(window, cx)
                .placeholder(placeholder)
                .default_value(initial)
        });
        let subscription = cx.subscribe(&input, |this: &mut Self, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::PressEnter { .. }) {
                this.submit(cx);
            }
        });
        let mut view = Self {
            provider,
            input,
            _input_subscription: subscription,
            results: Vec::new(),
            status: SearchStatus::Idle,
            last_query: None,
            indexing: false,
            index_message: None,
            pending_navigation: None,
            pending_toast: None,
        };
        if let Some(query) = start.query {
            view.run_query(&query, cx);
        }
        view
    }

    /// The current result rows.
    pub fn results(&self) -> &[SearchHit] {
        &self.results
    }

    /// The current status.
    pub fn status(&self) -> &SearchStatus {
        &self.status
    }

    /// The last submitted query, if any.
    pub fn last_query(&self) -> Option<&str> {
        self.last_query.as_deref()
    }

    /// Whether the rebuild action is running.
    pub fn indexing(&self) -> bool {
        self.indexing
    }

    /// The rebuild caption, if any.
    pub fn index_message(&self) -> Option<&str> {
        self.index_message.as_deref()
    }

    /// Submit the current field value.
    pub fn submit(&mut self, cx: &mut Context<Self>) {
        let query = self.input.read(cx).value().to_string();
        self.run_query(&query, cx);
    }

    /// Route and execute a query through the provider.
    fn run_query(&mut self, raw: &str, cx: &mut Context<Self>) {
        self.last_query = Some(raw.trim().to_string());
        match classify_query(raw) {
            SearchMode::Idle => {
                self.results.clear();
                self.status = SearchStatus::Idle;
            }
            SearchMode::Tag(tag) => {
                self.status = SearchStatus::Searching;
                match self.provider.search_by_tag(&tag) {
                    Ok(results) => {
                        self.results = results;
                        self.status = SearchStatus::Done;
                    }
                    Err(error) => {
                        self.results.clear();
                        self.status = SearchStatus::Failed(failure(&error));
                    }
                }
            }
            SearchMode::FullText(query) => {
                self.status = SearchStatus::Searching;
                match self.provider.search_blocks(&query, 20) {
                    Ok(results) => {
                        self.results = results;
                        self.status = SearchStatus::Done;
                    }
                    Err(error) => {
                        self.results.clear();
                        self.status = SearchStatus::Failed(failure(&error));
                    }
                }
            }
        }
        cx.notify();
    }

    /// Rebuild the search index through the provider.
    pub fn rebuild_index(&mut self, cx: &mut Context<Self>) {
        if self.indexing {
            return;
        }
        self.indexing = true;
        self.index_message = match self.provider.rebuild_search_index() {
            Ok(message) => Some(message),
            Err(error) => Some(failure(&error)),
        };
        self.indexing = false;
        cx.notify();
    }

    /// Open the result at `index`.
    pub fn open_result(&mut self, index: usize) {
        if let Some(hit) = self.results.get(index) {
            self.pending_navigation = Some(hit.page_path.clone());
        }
    }
}

impl PanelRequests for SearchView {
    fn pending_navigation(&self) -> Option<&str> {
        self.pending_navigation.as_deref()
    }

    fn clear_pending_navigation(&mut self) {
        self.pending_navigation = None;
    }

    fn pending_toast(&self) -> Option<&str> {
        self.pending_toast.as_deref()
    }

    fn take_pending_toast(&mut self) -> Option<String> {
        self.pending_toast.take()
    }
}

impl Render for SearchView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        toolbar::body(self, cx)
    }
}

/// Prefix a provider error with the localized failure label.
fn failure(error: &str) -> String {
    format!("{} {error}", t_global("search.failed"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::core::prelude::v1::test;

    #[test]
    fn failure_prefixes_the_catalog_label() {
        assert_eq!(failure("boom"), "Failed: boom");
    }
}
