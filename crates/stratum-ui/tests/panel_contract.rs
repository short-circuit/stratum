//! Window-free contracts for the native Search and Query panels.
//!
//! The i18n gate walks every key the two surfaces render against all eight
//! catalogs. The model tests pin the shared `stratum-app` classification,
//! builder emission/parsing and saved-query formatting. The fixture flows
//! exercise the provider mutations the panels drive.

use stratum_app::query::builder::{build_datalog, parse_datalog_to_rows, Combiner, FieldId, OpId};
use stratum_app::template::template_variables;
use stratum_i18n::{t, Locale};
use stratum_ui::panels::ask::ASK_I18N_KEYS;
use stratum_ui::panels::flashcards::model::{Session, SessionState};
use stratum_ui::panels::flashcards::FLASHCARDS_I18N_KEYS;
use stratum_ui::panels::query::model::{PanelState, DEFAULT_QUERY};
use stratum_ui::panels::query::QUERY_I18N_KEYS;
use stratum_ui::panels::search::model::{classify_query, score_label, SearchMode};
use stratum_ui::panels::search::SEARCH_I18N_KEYS;
use stratum_ui::panels::templates::model::{filter_variables, VariableRow};
use stratum_ui::panels::templates::TEMPLATES_I18N_KEYS;
use stratum_ui::provider::{FixtureProvider, PanelData, SavedQueryRow};

/// Compare rows without the parser-assigned ids.
fn shape(
    rows: &[stratum_app::query::builder::FilterSpec],
) -> Vec<(FieldId, OpId, String, Combiner)> {
    rows.iter()
        .map(|row| (row.field, row.op, row.value.clone(), row.combiner))
        .collect()
}

#[test]
fn every_search_and_query_key_resolves_in_all_catalogs() {
    for locale in Locale::ALL {
        for key in SEARCH_I18N_KEYS.iter().chain(QUERY_I18N_KEYS) {
            assert_ne!(t(locale, key), *key, "{key} missing in {}", locale.code());
        }
    }
}

#[test]
fn every_ask_templates_and_flashcards_key_resolves_in_all_catalogs() {
    for locale in Locale::ALL {
        for key in ASK_I18N_KEYS
            .iter()
            .chain(TEMPLATES_I18N_KEYS)
            .chain(FLASHCARDS_I18N_KEYS)
        {
            assert_ne!(t(locale, key), *key, "{key} missing in {}", locale.code());
        }
    }
}

#[test]
fn template_placeholders_and_variable_filtering_match_the_shipped_flow() {
    let content = "# {{title}}\n\nDate: {{date}}\n\nAgain {{title}} {{ }} {{ attendees }}";
    assert_eq!(
        template_variables(content),
        vec!["title", "date", "attendees"]
    );

    let rows = vec![
        VariableRow {
            key: " title ".to_string(),
            value: "Weekly review".to_string(),
        },
        VariableRow {
            key: "   ".to_string(),
            value: "ignored".to_string(),
        },
        VariableRow {
            key: "date".to_string(),
            value: "2026-11-10".to_string(),
        },
    ];
    assert_eq!(
        filter_variables(&rows),
        vec![
            ("title".to_string(), "Weekly review".to_string()),
            ("date".to_string(), "2026-11-10".to_string()),
        ]
    );
}

#[test]
fn flashcard_session_transitions_reveal_rate_complete_and_restart() {
    use stratum_app::flashcards::FlashcardRating;

    let mut session = Session::new(2);
    assert_eq!(session.state(), &SessionState::Card { index: 0 });
    session.reveal();
    assert!(session.revealed());
    session.rate(FlashcardRating::Good);
    assert_eq!(session.state(), &SessionState::Card { index: 1 });
    assert_eq!(session.message_key(), Some("flashcards.correct"));

    session.reveal();
    session.rate(FlashcardRating::Hard);
    assert_eq!(session.state(), &SessionState::Complete);
    assert_eq!(session.message_key(), Some("flashcards.review_again_soon"));

    session.restart();
    assert_eq!(session.state(), &SessionState::Card { index: 0 });
    assert!(!session.revealed());
    assert_eq!(session.message_key(), None);
}

#[test]
fn fixture_ask_returns_cited_sources_and_the_no_sources_caption_path() {
    let provider = FixtureProvider::new();
    let answer = provider
        .ask_notes("what did I decide about the project?")
        .expect("fixture ask succeeds");
    assert!(answer.had_sources);
    assert_eq!(answer.sources.len(), 2);
    assert_eq!(answer.sources[0].path, "pages/fixture.md");
    assert_eq!(answer.sources[1].path, "pages/Project Notes.md");

    let plain = provider
        .ask_notes("what is the reading list?")
        .expect("fixture ask succeeds");
    assert!(!plain.had_sources);
    assert!(plain.sources.is_empty());
}

#[test]
fn search_classification_matches_the_shipped_routing() {
    assert_eq!(
        classify_query("#project"),
        SearchMode::Tag("project".to_string())
    );
    assert_eq!(
        classify_query("#  spaced tag  "),
        SearchMode::Tag("spaced tag".to_string())
    );
    assert_eq!(
        classify_query("todo"),
        SearchMode::FullText("todo".to_string())
    );
    assert_eq!(
        classify_query("  todo  "),
        SearchMode::FullText("todo".to_string())
    );
    assert_eq!(classify_query(""), SearchMode::Idle);
    assert_eq!(classify_query("   "), SearchMode::Idle);
    assert_eq!(classify_query("#"), SearchMode::Idle);
    assert_eq!(classify_query(" # "), SearchMode::Idle);
}

#[test]
fn search_score_labels_use_two_decimals() {
    assert_eq!(score_label(1.0), "· score: 1.00");
    assert_eq!(score_label(0.72), "· score: 0.72");
    assert_eq!(score_label(0.41), "· score: 0.41");
}

#[test]
fn builder_edits_emit_the_shared_query() {
    let mut state = PanelState::default();
    state.add_row();
    state.set_row_value("r0", "offline".to_string());
    let outcome = build_datalog(state.rows());
    assert_eq!(state.build_error(), None);
    assert_eq!(state.datalog(), outcome.datalog);
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
    assert!(QUERY_I18N_KEYS.contains(&"query.builder_requires_value"));
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
fn saved_query_subtitles_use_the_shared_formatter() {
    let row = SavedQueryRow {
        name: "Open".to_string(),
        query: DEFAULT_QUERY.to_string(),
        updated_at: "not-a-date".to_string(),
    };
    assert_eq!(PanelState::saved_query_subtitle(&row), "not-a-date");
    assert_eq!(
        PanelState::saved_query_subtitle(&SavedQueryRow {
            name: "Empty".to_string(),
            query: DEFAULT_QUERY.to_string(),
            updated_at: String::new(),
        }),
        ""
    );
}

#[test]
fn fixture_saved_query_flows_mutate_the_list() {
    let provider = FixtureProvider::new();
    let before = provider
        .saved_queries()
        .expect("fixture saved queries")
        .len();

    provider
        .save_query("Native query", DEFAULT_QUERY)
        .expect("save succeeds");
    let saved = provider.saved_queries().expect("saved queries");
    assert_eq!(saved.len(), before + 1);
    assert!(saved.iter().any(|row| row.name == "Native query"));

    provider
        .rename_query("Native query", "Renamed native query")
        .expect("rename succeeds");
    assert!(provider
        .saved_queries()
        .expect("saved queries")
        .iter()
        .any(|row| row.name == "Renamed native query"));

    provider
        .delete_query("Renamed native query")
        .expect("delete succeeds");
    assert_eq!(
        provider.saved_queries().expect("saved queries").len(),
        before
    );
}

#[test]
fn fixture_run_query_returns_the_table() {
    let provider = FixtureProvider::new();
    let table = provider
        .run_query("{:query [:find ?b :where [?b :block/marker \"TODO\"]]}")
        .expect("fixture query runs");
    assert_eq!(table.columns, vec!["?b", "?content"]);
    assert_eq!(table.rows.len(), 3);
    assert_eq!(table.rows[0][1], "Fixture TODO row");
}
