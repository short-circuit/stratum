//! Window-free state machine for the native query panel.
//!
//! [`PanelState`] owns the tab, the Datalog editor buffer, the visual builder
//! rows, the last result and the validation/build errors. Every builder
//! transition calls the shared `stratum_app::query::builder` helpers, so the
//! native panel and the shipped React panel emit byte-identical queries.

use stratum_app::query::builder::{
    build_datalog, parse_datalog_to_rows, BuildError, BuildOutcome, Combiner, FieldId, FilterSpec,
    OpId,
};
use stratum_app::query::display::format_updated_at;
use stratum_i18n::t_global;

use crate::provider::{QueryTable, SavedQueryRow};

/// The query panel's editor tab.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum QueryTab {
    /// Visual filter builder.
    #[default]
    Builder,
    /// Raw Datalog editor.
    Datalog,
}

/// The initial editor buffer used by the shipped query panel.
pub const INITIAL_QUERY: &str = concat!(
    "{:query [:find ?b ?content :where ",
    "[?b :block/marker \"TODO\"] [?b :block/content ?content]]}"
);

/// The reset buffer produced by the shipped panel's Reset action.
pub const DEFAULT_QUERY: &str = "{:query [:find ?b :where [?b :block/marker \"TODO\"]]}";

/// Every queryable field, in the shipped display order.
const ALL_FIELDS: [FieldId; 10] = [
    FieldId::Content,
    FieldId::Marker,
    FieldId::Priority,
    FieldId::PageTitle,
    FieldId::PagePath,
    FieldId::PageBlockCount,
    FieldId::BlockCreated,
    FieldId::BlockModified,
    FieldId::Tags,
    FieldId::HasWikilink,
];

/// The builder/editor state shared by the query view and its contract tests.
pub struct PanelState {
    tab: QueryTab,
    datalog: String,
    rows: Vec<FilterSpec>,
    result: Option<QueryTable>,
    error: Option<String>,
    running: bool,
    build_error: Option<BuildError>,
    parse_notice: bool,
    row_counter: usize,
}

impl Default for PanelState {
    fn default() -> Self {
        Self::new(INITIAL_QUERY)
    }
}

impl PanelState {
    /// Build state from a Datalog buffer.
    ///
    /// The initial rows are parsed from the buffer and the generated query is
    /// rebuilt so the builder preview matches the shipped panel's effect
    /// ordering.
    pub fn new(datalog: &str) -> Self {
        let mut state = Self {
            tab: QueryTab::Builder,
            datalog: datalog.to_string(),
            rows: parse_datalog_to_rows(datalog).unwrap_or_default(),
            result: None,
            error: None,
            running: false,
            build_error: None,
            parse_notice: false,
            row_counter: 0,
        };
        state.rebuild();
        state
    }

    /// The active tab.
    pub fn tab(&self) -> QueryTab {
        self.tab
    }

    /// The current Datalog buffer.
    pub fn datalog(&self) -> &str {
        &self.datalog
    }

    /// The visual builder rows.
    pub fn rows(&self) -> &[FilterSpec] {
        &self.rows
    }

    /// The last query result, if any.
    pub fn result(&self) -> Option<&QueryTable> {
        self.result.as_ref()
    }

    /// The last run error, if any.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Whether a query is running.
    pub fn running(&self) -> bool {
        self.running
    }

    /// The current builder validation error, if any.
    pub fn build_error(&self) -> Option<BuildError> {
        self.build_error
    }

    /// The localized key for the current builder validation error.
    pub fn build_error_key(&self) -> Option<&'static str> {
        self.build_error.map(BuildError::key)
    }

    /// Whether the last switch to the builder could not parse the buffer.
    pub fn parse_notice(&self) -> bool {
        self.parse_notice
    }

    /// The generated builder outcome.
    pub fn generated(&self) -> BuildOutcome {
        build_datalog(&self.rows)
    }

    /// Replace the Datalog buffer (raw editor input).
    pub fn set_datalog(&mut self, datalog: impl Into<String>) {
        self.datalog = datalog.into();
    }

    /// Switch tabs. Switching to the builder parses the current buffer.
    pub fn switch_tab(&mut self, tab: QueryTab) {
        if tab == QueryTab::Builder {
            match parse_datalog_to_rows(&self.datalog) {
                Some(rows) => {
                    self.rows = rows;
                    self.parse_notice = false;
                }
                None => self.parse_notice = true,
            }
        }
        self.tab = tab;
    }

    /// Restore the shipped reset query.
    pub fn reset(&mut self) {
        self.datalog = DEFAULT_QUERY.to_string();
        self.rows = parse_datalog_to_rows(DEFAULT_QUERY).unwrap_or_default();
        self.result = None;
        self.error = None;
        self.build_error = None;
        self.parse_notice = false;
        self.running = false;
    }

    /// Load a saved query into the editor and switch to the raw tab.
    pub fn load_saved(&mut self, query: &str) {
        self.datalog = query.to_string();
        self.tab = QueryTab::Datalog;
        self.result = None;
        self.error = None;
        self.build_error = None;
        self.parse_notice = false;
    }

    /// Copy the generated builder query into the raw editor.
    ///
    /// Returns `false` when the builder is invalid.
    pub fn copy_to_editor(&mut self) -> bool {
        let generated = self.generated();
        if generated.error.is_some() {
            return false;
        }
        self.datalog = generated.datalog;
        self.tab = QueryTab::Datalog;
        self.parse_notice = false;
        true
    }

    /// Append a default condition row.
    pub fn add_row(&mut self) {
        let id = format!("r{}", self.row_counter);
        self.row_counter += 1;
        self.rows.push(FilterSpec {
            id,
            field: FieldId::Content,
            op: OpId::Contains,
            value: String::new(),
            combiner: Combiner::And,
        });
        self.rebuild();
    }

    /// Remove the condition at `index`.
    pub fn remove_row(&mut self, index: usize) {
        if index < self.rows.len() {
            self.rows.remove(index);
            self.rebuild();
        }
    }

    /// Move a condition up (`dir = -1`) or down (`dir = 1`).
    pub fn move_row(&mut self, index: usize, dir: i32) {
        let target = index as i32 + dir;
        if index >= self.rows.len() || target < 0 || target as usize >= self.rows.len() {
            return;
        }
        let target = target as usize;
        let row = self.rows.remove(index);
        self.rows.insert(target, row);
        self.rebuild();
    }

    /// Cycle the field of a condition to the next whitelisted field.
    pub fn cycle_field(&mut self, index: usize) {
        let Some(row) = self.rows.get(index) else {
            return;
        };
        let position = ALL_FIELDS
            .iter()
            .position(|field| *field == row.field)
            .unwrap_or(0);
        let field = ALL_FIELDS[(position + 1) % ALL_FIELDS.len()];
        let op = if field.ops().contains(&row.op) {
            row.op
        } else {
            field.ops()[0]
        };
        if let Some(row) = self.rows.get_mut(index) {
            row.field = field;
            row.op = op;
        }
        self.rebuild();
    }

    /// Cycle the operator of a condition to the next operator for its field.
    pub fn cycle_op(&mut self, index: usize) {
        let Some(row) = self.rows.get(index) else {
            return;
        };
        let ops = row.field.ops();
        let position = ops.iter().position(|op| *op == row.op).unwrap_or(0);
        let op = ops[(position + 1) % ops.len()];
        if let Some(row) = self.rows.get_mut(index) {
            row.op = op;
        }
        self.rebuild();
    }

    /// Toggle the combiner of a non-first condition.
    pub fn toggle_combiner(&mut self, index: usize) {
        if index == 0 {
            return;
        }
        if let Some(row) = self.rows.get_mut(index) {
            row.combiner = match row.combiner {
                Combiner::And => Combiner::Or,
                Combiner::Or => Combiner::And,
            };
        }
        self.rebuild();
    }

    /// Update a condition value by row id.
    pub fn set_row_value(&mut self, id: &str, value: String) {
        if let Some(row) = self.rows.iter_mut().find(|row| row.id == id) {
            row.value = value;
            self.rebuild();
        }
    }

    /// Record a successful result.
    pub fn set_result(&mut self, result: QueryTable) {
        self.result = Some(result);
        self.error = None;
        self.running = false;
    }

    /// Record a run failure.
    pub fn set_run_error(&mut self, message: impl Into<String>) {
        self.error = Some(message.into());
        self.running = false;
    }

    /// Mark the query as running.
    pub fn set_running(&mut self, running: bool) {
        self.running = running;
    }

    /// Rebuild the Datalog buffer from the rows.
    ///
    /// A valid build replaces the buffer; an invalid build keeps the last valid
    /// buffer and surfaces the localized error key.
    fn rebuild(&mut self) {
        let outcome = build_datalog(&self.rows);
        match outcome.error {
            Some(error) => self.build_error = Some(error),
            None => {
                self.datalog = outcome.datalog;
                self.build_error = None;
            }
        }
    }

    /// The secondary label for a saved query row.
    pub fn saved_query_subtitle(row: &SavedQueryRow) -> String {
        format_updated_at(&row.updated_at)
    }

    /// The localized builder error, if any.
    pub fn build_error_message(&self) -> Option<String> {
        self.build_error_key().map(t_global)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::core::prelude::v1::test;

    fn shape(rows: &[FilterSpec]) -> Vec<(FieldId, OpId, String, Combiner)> {
        rows.iter()
            .map(|row| (row.field, row.op, row.value.clone(), row.combiner))
            .collect()
    }

    #[test]
    fn builder_edits_match_the_shared_build_function() {
        let mut state = PanelState::default();
        state.add_row();
        state.set_row_value("r0", "offline".to_string());
        let generated = build_datalog(state.rows());
        assert_eq!(state.build_error(), None);
        assert_eq!(state.datalog(), generated.datalog);
    }

    #[test]
    fn generated_queries_parse_back_to_the_same_rows() {
        let mut state = PanelState::default();
        state.add_row();
        state.set_row_value("r0", "offline".to_string());
        let parsed = parse_datalog_to_rows(state.datalog()).expect("generated query parses");
        assert_eq!(shape(&parsed), shape(state.rows()));
    }

    #[test]
    fn invalid_specs_surface_the_build_error_key() {
        let mut state = PanelState::default();
        state.add_row();
        assert_eq!(
            state.build_error_key(),
            Some("query.builder_requires_value")
        );
    }

    #[test]
    fn reset_restores_the_shipped_reset_query() {
        let mut state = PanelState::default();
        state.add_row();
        state.reset();
        assert_eq!(state.datalog(), DEFAULT_QUERY);
        assert_eq!(state.build_error(), None);
        assert_eq!(state.result(), None);
    }

    #[test]
    fn switching_to_the_builder_parses_or_notices() {
        let mut state = PanelState::default();
        state.set_datalog("{:query [:find ?b :where [?b :block/marker \"TODO\"]]}");
        state.switch_tab(QueryTab::Builder);
        assert!(!state.parse_notice());
        assert_eq!(state.rows().len(), 1);

        state.set_datalog("not a query");
        state.switch_tab(QueryTab::Datalog);
        state.switch_tab(QueryTab::Builder);
        assert!(state.parse_notice());
    }

    #[test]
    fn copy_to_editor_moves_the_generated_query_into_the_buffer() {
        let mut state = PanelState::default();
        state.add_row();
        state.set_row_value("r0", "offline".to_string());
        assert!(state.copy_to_editor());
        assert_eq!(state.tab(), QueryTab::Datalog);
        assert_eq!(state.datalog(), build_datalog(state.rows()).datalog);
    }

    #[test]
    fn saved_query_subtitles_use_the_shared_formatter() {
        let row = SavedQueryRow {
            name: "Open".to_string(),
            query: DEFAULT_QUERY.to_string(),
            updated_at: "not-a-date".to_string(),
        };
        assert_eq!(PanelState::saved_query_subtitle(&row), "not-a-date");
    }
}
