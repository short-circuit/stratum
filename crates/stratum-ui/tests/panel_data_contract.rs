//! Window-free contract tests for the panel data seam.
//!
//! These prove the [`PanelData`] trait is non-breaking (every default is
//! documented), that the deterministic fixture returns the shapes the seven
//! native panels need, and that the visual Datalog builder round-trips through
//! the provider. No GPUI window is opened.

use chrono::Local;
use stratum_app::query::builder::{build_datalog, Combiner, FieldId, FilterSpec, OpId};
use stratum_app::reminders::due::{bucket, parse_due};
use stratum_ui::provider::{FixtureProvider, PanelData, SavedQueryRow, PANEL_DATA_NOT_AVAILABLE};

/// A provider that only implements the trait defaults.
struct DefaultProvider;

impl PanelData for DefaultProvider {}

#[test]
fn default_provider_returns_the_documented_defaults() {
    let provider = DefaultProvider;

    assert!(provider.search_blocks("anything", 10).unwrap().is_empty());
    assert!(provider.search_by_tag("anything").unwrap().is_empty());
    assert_eq!(provider.rebuild_search_index(), Ok(String::new()));
    assert_eq!(
        provider.run_query("{:query [:find ?b :where [?b :block/content ?c]]}"),
        Err(PANEL_DATA_NOT_AVAILABLE.to_string())
    );
    assert!(provider.saved_queries().unwrap().is_empty());
    assert_eq!(
        provider.save_query("Q", "{:query [:find ?b]}"),
        Ok(SavedQueryRow::new("Q", "{:query [:find ?b]}"))
    );
    assert_eq!(
        provider.rename_query("Q", "R"),
        Ok(SavedQueryRow::new("R", ""))
    );
    assert_eq!(provider.delete_query("Q"), Ok(()));
    assert_eq!(
        provider.ask_notes("why?"),
        Err(PANEL_DATA_NOT_AVAILABLE.to_string())
    );
    assert!(provider.templates().unwrap().is_empty());
    assert_eq!(
        provider.apply_template("T", "pages/x.md", Vec::new()),
        Ok(String::new())
    );
    assert!(provider.install_starter_templates().unwrap().is_empty());
    assert!(provider.flashcards().unwrap().is_empty());
    assert_eq!(provider.review_card("c", 4, "pages/x.md"), Ok(()));
    assert!(provider.reminders().unwrap().is_empty());
    assert!(provider.due_reminders().is_empty());
    assert_eq!(provider.snooze_reminder("p", "b", 10), Ok(()));
    assert_eq!(provider.clear_reminder("p", "b"), Ok(()));
    assert_eq!(provider.set_reminder("p", "b", None), Ok(()));
    assert!(provider.plugins().unwrap().is_empty());

    let enabled = provider.set_plugin_enabled("p", true).unwrap();
    assert_eq!(enabled.id, "p");
    assert!(enabled.enabled);
    assert_eq!(enabled.status, "ready");
    assert_eq!(provider.reload_plugin("p").unwrap().id, "p");
    assert_eq!(provider.uninstall_plugin("p"), Ok(()));
    assert_eq!(
        provider.install_plugin_path("plugins/p.wasm").unwrap().id,
        "plugins/p.wasm"
    );
    assert_eq!(
        provider
            .install_plugin_url("https://example.test/p.wasm", None, "p")
            .unwrap()
            .id,
        "p"
    );
    assert_eq!(provider.plugin_note_read("pages/x.md"), Ok(String::new()));
    assert_eq!(
        provider.plugin_http_request("https://example.test", "GET"),
        Ok(String::new())
    );
    assert_eq!(
        provider.plugin_catalog(),
        Err(PANEL_DATA_NOT_AVAILABLE.to_string())
    );
    assert_eq!(
        provider.theme_catalog(),
        Err(PANEL_DATA_NOT_AVAILABLE.to_string())
    );
    assert!(provider.installed_themes().is_empty());
    let installed = provider.install_theme("midnight").unwrap();
    assert_eq!(installed.id, "midnight");
    assert!(installed.active);
    assert_eq!(
        provider.apply_theme("midnight"),
        Err(PANEL_DATA_NOT_AVAILABLE.to_string())
    );
    assert_eq!(provider.uninstall_theme("midnight"), Ok(()));
}

#[test]
fn fixture_search_returns_three_ranked_hits() {
    let provider = FixtureProvider::new();
    let hits = provider.search_blocks("fixture", 10).unwrap();
    assert_eq!(hits.len(), 3);
    assert_eq!(hits[0].page_path, "pages/fixture.md");
    assert_eq!(hits[1].page_path, "pages/Project Notes.md");
    assert_eq!(hits[2].page_path, "pages/Welcome.md");
    assert_eq!(hits[0].score, 1.0);
    assert_eq!(hits[1].score, 0.72);
    assert_eq!(hits[2].score, 0.41);
    assert!(hits.iter().all(|hit| hit.snippet.contains("fixture")));
    assert!(provider.search_blocks("", 10).unwrap().is_empty());

    assert_eq!(provider.search_by_tag("project").unwrap().len(), 2);
    assert_eq!(provider.search_by_tag("reading").unwrap().len(), 2);
    assert!(provider.search_by_tag("unknown").unwrap().is_empty());
    assert_eq!(
        provider.rebuild_search_index(),
        Ok("Indexed 4 pages".to_string())
    );
}

#[test]
fn fixture_query_table_and_saved_query_mutation() {
    let provider = FixtureProvider::new();
    let table = provider
        .run_query("{:query [:find ?b ?content :where [?b :block/content ?content]]}")
        .unwrap();
    assert_eq!(table.columns, ["?b", "?content"]);
    assert_eq!(table.rows.len(), 3);
    assert!(provider.run_query("").is_err());

    let seeded = provider.saved_queries().unwrap();
    assert_eq!(seeded.len(), 2);
    assert!(seeded.iter().any(|row| row.name == "Open TODOs"));
    assert!(seeded.iter().any(|row| row.name == "Project pages"));

    let saved = provider
        .save_query("Weekly review", "{:query [:find ?b]}")
        .unwrap();
    assert_eq!(saved.name, "Weekly review");
    assert_eq!(provider.saved_queries().unwrap().len(), 3);

    let renamed = provider
        .rename_query("Weekly review", "Monthly review")
        .unwrap();
    assert_eq!(renamed.name, "Monthly review");
    assert!(provider
        .saved_queries()
        .unwrap()
        .iter()
        .any(|row| row.name == "Monthly review"));

    provider.delete_query("Monthly review").unwrap();
    assert_eq!(provider.saved_queries().unwrap().len(), 2);

    assert!(provider.save_query("", "{:query [:find ?b]}").is_err());
    assert!(provider.rename_query("Open TODOs", "").is_err());
    assert!(provider.delete_query("missing").is_err());
}

#[test]
fn fixture_ask_answers_with_and_without_sources() {
    let provider = FixtureProvider::new();
    let with_sources = provider.ask_notes("Should we decide now?").unwrap();
    assert!(with_sources.had_sources);
    assert_eq!(with_sources.sources.len(), 2);
    assert_eq!(with_sources.sources[0].path, "pages/fixture.md");

    let without_sources = provider.ask_notes("What is the weather?").unwrap();
    assert!(!without_sources.had_sources);
    assert!(without_sources.sources.is_empty());
    assert!(provider.ask_notes("").is_err());
}

#[test]
fn fixture_templates_and_cards_are_deterministic() {
    let provider = FixtureProvider::new();
    let templates = provider.templates().unwrap();
    assert_eq!(templates.len(), 3);
    assert!(templates
        .iter()
        .all(|template| template.variables.contains(&"date".to_string())
            && template.variables.contains(&"title".to_string())));
    assert_eq!(
        provider.apply_template("Daily note", "journals/2026-11-10.md", Vec::new()),
        Ok("journals/2026-11-10.md".to_string())
    );
    assert_eq!(provider.install_starter_templates().unwrap().len(), 3);

    let cards = provider.flashcards().unwrap();
    assert_eq!(cards.len(), 3);
    provider
        .review_card("card-1", 5, "pages/fixture.md")
        .unwrap();
    provider
        .review_card("card-2", 2, "pages/Reading List.md")
        .unwrap();
    assert_eq!(
        provider.reviewed_qualities(),
        vec![("card-1".to_string(), 5), ("card-2".to_string(), 2)]
    );
    assert!(provider.review_card("missing", 4, "pages/x.md").is_err());
}

#[test]
fn fixture_reminders_bucket_into_two_overdue_one_today_two_upcoming() {
    let provider = FixtureProvider::new();
    let reminders = provider.reminders().unwrap();
    assert_eq!(reminders.len(), 5);

    let dues: Vec<_> = reminders
        .iter()
        .map(|reminder| parse_due(&reminder.due).expect("valid due"))
        .collect();
    let buckets = bucket(&dues, Local::now());
    assert_eq!(buckets.overdue.len(), 2);
    assert_eq!(buckets.today.len(), 1);
    assert_eq!(buckets.upcoming.len(), 2);

    let due_reminders = provider.due_reminders();
    if std::env::var("STRATUM_REMINDER_TOAST").ok().as_deref() == Some("1") {
        assert_eq!(due_reminders.len(), 2);
    } else {
        assert!(due_reminders.is_empty());
    }

    let first = reminders[0].clone();
    provider
        .snooze_reminder(&first.page_path, &first.block_id, 15)
        .unwrap();
    let snoozed = provider
        .reminders()
        .unwrap()
        .into_iter()
        .find(|reminder| reminder.block_id == first.block_id)
        .expect("snoozed reminder");
    assert_ne!(snoozed.due, first.due);

    provider
        .clear_reminder(&first.page_path, &first.block_id)
        .unwrap();
    assert_eq!(provider.reminders().unwrap().len(), 4);

    let second = provider.reminders().unwrap()[0].clone();
    provider
        .set_reminder(
            &second.page_path,
            &second.block_id,
            Some("2026-12-01T09:00:00Z".to_string()),
        )
        .unwrap();
    let updated = provider
        .reminders()
        .unwrap()
        .into_iter()
        .find(|reminder| reminder.block_id == second.block_id)
        .expect("updated reminder");
    assert_eq!(updated.due, "2026-12-01T09:00:00Z");
}

#[test]
fn fixture_plugins_and_catalogs_are_mutable() {
    let provider = FixtureProvider::new();
    let plugins = provider.plugins().unwrap();
    assert_eq!(plugins.len(), 2);
    assert!(plugins.iter().any(|plugin| plugin.status == "ready"));
    assert!(plugins
        .iter()
        .any(|plugin| plugin.status == "disabled" && !plugin.hooks.is_empty()));

    let disabled = provider.set_plugin_enabled("word-count", false).unwrap();
    assert!(!disabled.enabled);
    assert_eq!(disabled.status, "disabled");
    let enabled = provider.set_plugin_enabled("auto-link", true).unwrap();
    assert!(enabled.enabled);
    assert_eq!(provider.reload_plugin("auto-link").unwrap().id, "auto-link");
    provider.uninstall_plugin("word-count").unwrap();
    assert_eq!(provider.plugins().unwrap().len(), 1);

    let catalog = provider.plugin_catalog().unwrap();
    assert_eq!(catalog.entries.len(), 3);
    assert!(!catalog.offline);
    let themes = provider.theme_catalog().unwrap();
    assert_eq!(themes.entries.len(), 2);
    assert!(!themes.offline);
    assert!(provider.installed_themes().is_empty());

    let installed = provider.install_theme("midnight").unwrap();
    assert_eq!(installed.id, "midnight");
    assert!(installed.active);
    assert_eq!(provider.installed_themes().len(), 1);
    let values = provider.apply_theme("midnight").unwrap();
    assert!(values.dark_mode);
    assert_eq!(values.primary_color, "#0f172a");
    provider.uninstall_theme("midnight").unwrap();
    assert!(provider.installed_themes().is_empty());
}

#[test]
fn builder_output_round_trips_through_the_fixture_provider() {
    let specs = vec![
        FilterSpec {
            id: "p0".to_string(),
            field: FieldId::Content,
            op: OpId::Contains,
            value: "fixture".to_string(),
            combiner: Combiner::And,
        },
        FilterSpec {
            id: "p1".to_string(),
            field: FieldId::PageTitle,
            op: OpId::Eq,
            value: "Fixture Note".to_string(),
            combiner: Combiner::And,
        },
    ];
    let outcome = build_datalog(&specs);
    assert_eq!(outcome.error, None);
    let provider = FixtureProvider::new();
    let table = provider.run_query(&outcome.datalog).unwrap();
    assert_eq!(table.columns.len(), 2);
    assert_eq!(table.rows.len(), 3);
}
