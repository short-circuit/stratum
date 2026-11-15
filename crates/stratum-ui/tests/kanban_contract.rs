//! Window-free contracts for the native Kanban surface.
//!
//! The i18n gate walks every key the board renders against all eight catalogs,
//! the model tests pin marker→column mapping and grouping, and the fixture flows
//! exercise the provider mutations the panel drives (create, move, edit, delete)
//! without a window or a vault.

use stratum_i18n::{t, Locale};
use stratum_ui::panels::kanban::model::{column_for_marker, group, page_label, ColumnId};
use stratum_ui::panels::kanban::KANBAN_I18N_KEYS;
use stratum_ui::provider::panels::KanbanCardRow;
use stratum_ui::provider::{FixtureProvider, PanelData};

#[test]
fn every_kanban_key_resolves_in_all_catalogs() {
    for locale in Locale::ALL {
        for key in KANBAN_I18N_KEYS {
            assert_ne!(t(locale, key), *key, "{key} missing in {}", locale.code());
        }
    }
}

#[test]
fn fixture_board_places_every_seeded_card_in_its_column() {
    let provider = FixtureProvider::new();
    let cards = provider.kanban_cards().expect("fixture board loads");
    assert_eq!(cards.len(), 5);
    let grouped = group(&cards);
    let counts: Vec<(ColumnId, usize)> = grouped
        .into_iter()
        .map(|(column, rows)| (column, rows.len()))
        .collect();
    assert_eq!(
        counts,
        vec![
            (ColumnId::ToDo, 2),
            (ColumnId::InProgress, 1),
            (ColumnId::Done, 2),
        ]
    );
    // The seeded journal card exercises the journal page label and icon.
    let journal = cards
        .iter()
        .find(|card| card.page_path.starts_with("journals/"))
        .expect("a journal card is seeded");
    assert_eq!(page_label(journal), journal.page_title.clone().unwrap());
}

#[test]
fn create_move_and_delete_round_trip_through_the_provider() {
    let provider = FixtureProvider::new();
    let created = provider
        .create_kanban_card("Write the parity report", "TODO")
        .expect("create succeeds");
    assert_eq!(created.marker.as_deref(), Some("TODO"));
    assert!(created.page_path.starts_with("journals/"));
    let cards = provider.kanban_cards().expect("board reloads");
    assert!(cards.iter().any(|card| card.id == created.id));
    assert_eq!(cards.len(), 6);

    // Dropping a card into In Progress rewrites its marker to the column's
    // canonical marker, which is what moves it between columns.
    let mut moved = created.clone();
    moved.marker = Some(ColumnId::InProgress.target_marker().to_string());
    provider.update_kanban_card(moved).expect("move succeeds");
    let cards = provider.kanban_cards().expect("board reloads");
    let moved = cards
        .iter()
        .find(|card| card.id == created.id)
        .expect("moved card is still on the board");
    assert_eq!(
        column_for_marker(moved.marker.as_deref().unwrap()),
        Some(ColumnId::InProgress)
    );

    provider
        .delete_kanban_card(&created.id, &created.page_path)
        .expect("delete succeeds");
    let cards = provider.kanban_cards().expect("board reloads");
    assert_eq!(cards.len(), 5);
    assert!(!cards.iter().any(|card| card.id == created.id));
}

#[test]
fn editing_a_card_keeps_its_page_and_priority() {
    let provider = FixtureProvider::new();
    let mut card: KanbanCardRow = provider
        .kanban_cards()
        .expect("board loads")
        .into_iter()
        .next()
        .expect("a card is seeded");
    let page_path = card.page_path.clone();
    card.content = "Renamed card".to_string();
    card.priority = Some("C".to_string());
    card.marker = Some("DONE".to_string());
    provider.update_kanban_card(card).expect("edit succeeds");
    let cards = provider.kanban_cards().expect("board reloads");
    let edited = cards
        .iter()
        .find(|row| row.page_path == page_path && row.content == "Renamed card")
        .expect("edited card is on the board");
    assert_eq!(edited.priority.as_deref(), Some("C"));
    assert_eq!(
        column_for_marker(edited.marker.as_deref().unwrap()),
        Some(ColumnId::Done)
    );
}

#[test]
fn unknown_cards_report_an_error_instead_of_panicking() {
    let provider = FixtureProvider::new();
    let card = KanbanCardRow::new("missing", "ghost", "TODO", "pages/x.md");
    assert!(provider.update_kanban_card(card).is_err());
    assert!(provider
        .delete_kanban_card("missing", "pages/x.md")
        .is_err());
}
