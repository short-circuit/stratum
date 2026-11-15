//! Native Datalog query panel.
//!
//! [`QueryView`] ports the shipped desktop `QueryPanel`: a builder/datalog tab
//! pair, the visual filter builder backed by `stratum_app::query::builder`, the
//! raw editor, run/save/reset actions, the result table and the saved-query
//! list with save/rename/delete dialogs.
//!
//! Provider calls stay behind [`crate::provider::PanelData`]; the module owns
//! presentation and the pending navigation/toast requests the shell consumes.
//! The state transitions live in [`flows`], input synchronisation in [`inputs`],
//! and the sub-surfaces in the sibling modules. [`QUERY_I18N_KEYS`] is the
//! release gate list for every string this surface renders.

use std::collections::HashMap;
use std::sync::Arc;

use gpui_kit::component::input::{InputState, TextareaState};
use gpui_kit::component::ActiveTheme;
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::panels::PanelRequests;
use crate::provider::{QueryTable, SavedQueryRow, ShellData};

pub mod builder_rows;
pub mod dialogs;
mod flows;
mod inputs;
pub mod model;
pub mod results;
pub mod saved;
pub mod toolbar;

use dialogs::SavedDialog;
use model::PanelState;
pub use model::QueryTab;

/// Every translation key the query surface renders.
///
/// The contract test walks this list against all eight catalogs, so adding a
/// visible string without a catalog entry fails the build instead of leaking a
/// raw key into the UI.
pub const QUERY_I18N_KEYS: &[&str] = &[
    "common.cancel",
    "common.delete",
    "common.rename",
    "common.reset",
    "common.save",
    "common.this_cannot_be_undone",
    "query.add_condition",
    "query.and",
    "query.builder_empty",
    "query.builder_hint",
    "query.builder_invalid_condition",
    "query.builder_mode",
    "query.builder_not_parseable",
    "query.builder_or_needs_two",
    "query.builder_requires_value",
    "query.cancel",
    "query.combiner",
    "query.copied_to_editor",
    "query.copy_to_editor",
    "query.could_not_load_saved_queries",
    "query.datalog_mode",
    "query.datalog_query",
    "query.delete_label",
    "query.delete_saved_query",
    "query.enter_datalog_query",
    "query.field",
    "query.field.block_created",
    "query.field.block_modified",
    "query.field.content",
    "query.field.has_wikilink",
    "query.field.marker",
    "query.field.page_block_count",
    "query.field.page_path",
    "query.field.page_title",
    "query.field.priority",
    "query.field.tags",
    "query.generated_query",
    "query.move_down",
    "query.move_up",
    "query.name",
    "query.name_must_not_be_empty",
    "query.no_saved_queries_yet_write_a_query_above_and_click_save_quer",
    "query.op",
    "query.op.absent",
    "query.op.contains",
    "query.op.equals",
    "query.op.gt",
    "query.op.gte",
    "query.op.has",
    "query.op.lt",
    "query.op.lte",
    "query.op.not_equals",
    "query.op.not_has",
    "query.op.present",
    "query.op.starts_with",
    "query.or",
    "query.query_returned_no_results",
    "query.refresh_list",
    "query.remove_condition",
    "query.rename",
    "query.rename_saved_query",
    "query.renamed_saved_queries_sync_with_your_vault",
    "query.renaming",
    "query.run",
    "query.run_query",
    "query.running",
    "query.save",
    "query.save_query",
    "query.save_query_label",
    "query.saved_queries",
    "query.saved_queries_are_stored_in_your_vault_and_sync_across_devic",
    "query.value",
];

/// Deterministic start state for the query panel.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct QueryStart {
    /// Datalog buffer to prefill.
    pub datalog: Option<String>,
    /// Initial tab.
    pub tab: Option<QueryTab>,
    /// Run the current query after construction.
    pub run: bool,
}

/// The query panel entity.
pub struct QueryView {
    provider: Arc<dyn ShellData>,
    state: PanelState,
    saved: Vec<SavedQueryRow>,
    saved_error: Option<String>,
    saved_loading: bool,
    pub(super) dialog: Option<SavedDialog>,
    delete_target: Option<String>,
    editor: Option<Entity<TextareaState>>,
    editor_subscription: Option<Subscription>,
    row_inputs: HashMap<String, Entity<InputState>>,
    row_subscriptions: Vec<Subscription>,
    dialog_input: Option<Entity<InputState>>,
    dialog_subscription: Option<Subscription>,
    pending_navigation: Option<String>,
    pending_toast: Option<String>,
}

impl QueryView {
    /// Build the query panel, prefill the requested buffer/tab and run when
    /// `start.run` is set.
    pub fn new(
        provider: Arc<dyn ShellData>,
        start: QueryStart,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut state = match &start.datalog {
            Some(datalog) => {
                let mut state = PanelState::new(datalog);
                // Preserve the caller's exact buffer for the raw editor; the
                // builder rows still come from the shared parser.
                state.set_datalog(datalog.clone());
                state
            }
            None => PanelState::default(),
        };
        if let Some(tab) = start.tab {
            state.switch_tab(tab);
        }
        let saved = provider.saved_queries().unwrap_or_default();
        let mut view = Self {
            provider,
            state,
            saved,
            saved_error: None,
            saved_loading: false,
            dialog: None,
            delete_target: None,
            editor: None,
            editor_subscription: None,
            row_inputs: HashMap::new(),
            row_subscriptions: Vec::new(),
            dialog_input: None,
            dialog_subscription: None,
            pending_navigation: None,
            pending_toast: None,
        };
        if start.run {
            view.run(cx);
        }
        view
    }

    /// The current builder/editor state.
    pub fn state(&self) -> &PanelState {
        &self.state
    }

    /// The current Datalog buffer.
    pub fn datalog(&self) -> &str {
        self.state.datalog()
    }

    /// The active tab.
    pub fn tab(&self) -> QueryTab {
        self.state.tab()
    }

    /// The last query result, if any.
    pub fn result(&self) -> Option<&QueryTable> {
        self.state.result()
    }

    /// The last run error, if any.
    pub fn error(&self) -> Option<&str> {
        self.state.error()
    }

    /// The current builder error key, if any.
    pub fn build_error_key(&self) -> Option<&'static str> {
        self.state.build_error_key()
    }

    /// The saved-query rows.
    pub fn saved(&self) -> &[SavedQueryRow] {
        &self.saved
    }

    /// The saved-query load error, if any.
    pub fn saved_error(&self) -> Option<&str> {
        self.saved_error.as_deref()
    }

    /// Whether the saved-query list is refreshing.
    pub fn saved_loading(&self) -> bool {
        self.saved_loading
    }

    /// Whether a save/rename dialog is open.
    pub fn dialog_open(&self) -> bool {
        self.dialog.is_some()
    }

    /// The dialog name field value.
    pub fn dialog_name(&self) -> Option<&str> {
        self.dialog.as_ref().map(|dialog| dialog.name.as_str())
    }

    /// The pending delete target, if any.
    pub fn delete_target(&self) -> Option<&str> {
        self.delete_target.as_deref()
    }

    /// The builder value input for a row, if it exists.
    pub fn row_input(&self, id: &str) -> Option<&Entity<InputState>> {
        self.row_inputs.get(id)
    }

    /// The raw Datalog editor, if it has been created.
    pub fn editor(&self) -> Option<&Entity<TextareaState>> {
        self.editor.as_ref()
    }

    /// The dialog name input, if it exists.
    pub fn dialog_input(&self) -> Option<&Entity<InputState>> {
        self.dialog_input.as_ref()
    }
}

impl PanelRequests for QueryView {
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

impl Render for QueryView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_inputs(window, cx);
        let theme = cx.theme();
        let background = theme.colors.background;
        let foreground = theme.colors.foreground;

        div()
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .bg(background)
            .text_color(foreground)
            .child(toolbar::header(self, cx))
            .child(
                div()
                    .id("query-body")
                    .flex()
                    .flex_col()
                    .flex_1()
                    .overflow_y_scroll()
                    .p(px(24.0))
                    .gap(px(12.0))
                    .when(self.state.tab() == QueryTab::Builder, |element| {
                        element.child(builder_rows::builder_section(self, cx))
                    })
                    .when(self.state.tab() == QueryTab::Datalog, |element| {
                        element.child(toolbar::editor_section(self, cx))
                    })
                    .child(toolbar::actions(self, cx))
                    .when_some(self.state.error().map(str::to_string), |element, error| {
                        element.child(toolbar::error_alert(error, cx))
                    })
                    .child(results::results_section(self, cx))
                    .child(saved::saved_section(self, cx)),
            )
            .when(self.dialog.is_some(), |element| {
                element.child(dialogs::dialog(self, cx))
            })
            .when(self.delete_target.is_some(), |element| {
                element.child(dialogs::delete_dialog(self, cx))
            })
    }
}
