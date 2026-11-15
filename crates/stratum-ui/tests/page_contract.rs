//! Window-free contract tests for the native page surface.
//!
//! These tests exercise the pure models the gpui page panel delegates to: the
//! Note menu table, move-path validation, backlink grouping and relevance
//! clamping, the suggestion state machine and the deterministic fixture page
//! seam. They also walk [`PAGE_I18N_KEYS`] against all eight catalogs, so a
//! visible string without a catalog entry fails the build instead of leaking a
//! raw key into the UI.

use stratum_i18n::{t, Locale};
use stratum_ui::panels::page::menu::{NoteAction, NOTE_MENU_ACTIONS};
use stratum_ui::panels::page::move_dialog::{validate_move, MoveError};
use stratum_ui::panels::page::{SuggestionsState, PAGE_I18N_KEYS};
use stratum_ui::provider::{
    clamp_relevance, group_backlinks, normalize_move_path, BacklinkRow, ConnectionRow,
    FixtureProvider, ShellData, FIXTURE_BACKLINK_ANCHOR, FIXTURE_BACKLINK_SOURCE,
    FIXTURE_HIGHLIGHT_BLOCK, FIXTURE_PAGE,
};

#[test]
fn page_keys_are_present_in_every_catalog() {
    for locale in Locale::ALL {
        for &key in PAGE_I18N_KEYS {
            assert_ne!(t(locale, key), key, "{key} missing in {}", locale.code());
        }
    }
}

#[test]
fn page_keys_cover_the_shipped_controls() {
    for key in [
        "asset.attach",
        "backlinks.backlinks",
        "backlinks.linked_references",
        "backlinks.unlinked_mentions",
        "backlinks.no_backlinks_found",
        "backlinks.backlinked_snippet",
        "common.cancel",
        "page.actions",
        "page.delete",
        "page.delete_page",
        "page.edit_mode",
        "page.normalize_file",
        "page.note",
        "page.read_mode",
        "page.reindex_note",
        "page.voice_memo_dictation",
        "sidebar.move",
        "sidebar.move_page_title",
        "sidebar.move_rename",
        "sidebar.new_path",
        "sidebar.page_already_exists",
        "sidebar.path_must_not_be_empty",
        "sidebar.path_must_start_with_pages",
        "suggested_connections.suggested_connections",
        "suggested_connections.find",
        "suggested_connections.see_also",
    ] {
        assert!(
            PAGE_I18N_KEYS.contains(&key),
            "{key} is missing from the release gate list"
        );
    }
}

#[test]
fn note_menu_matches_the_shipped_order_and_styling() {
    assert_eq!(
        NOTE_MENU_ACTIONS,
        [
            NoteAction::Reindex,
            NoteAction::Normalize,
            NoteAction::Move,
            NoteAction::Delete,
        ]
    );
    assert_eq!(NoteAction::Reindex.label_key(), "page.reindex_note");
    assert_eq!(NoteAction::Normalize.label_key(), "page.normalize_file");
    assert_eq!(NoteAction::Move.label_key(), "sidebar.move_rename");
    assert_eq!(NoteAction::Delete.label_key(), "page.delete_page");
    for action in NOTE_MENU_ACTIONS {
        assert_eq!(
            action.is_destructive(),
            action == NoteAction::Delete,
            "{action:?}"
        );
        for locale in Locale::ALL {
            let key = action.label_key();
            assert_ne!(t(locale, key), key, "{key} missing in {}", locale.code());
        }
    }
}

#[test]
fn move_validation_matches_the_shipped_dialog() {
    assert_eq!(
        validate_move("  pages/New  ", "pages/Old.md", &[]),
        Ok("pages/New.md".to_string())
    );
    assert_eq!(
        validate_move("pages/Old.md", "pages/Old.md", &[]),
        Ok("pages/Old.md".to_string())
    );
    assert_eq!(
        validate_move("   ", "pages/Old.md", &[]),
        Err(MoveError::Empty)
    );
    assert_eq!(
        validate_move("notes/New.md", "pages/Old.md", &[]),
        Err(MoveError::NotInPages)
    );
    let existing = vec!["pages/New.md".to_string()];
    assert_eq!(
        validate_move("pages/New.md", "pages/Old.md", &existing),
        Err(MoveError::Exists)
    );
    assert_eq!(normalize_move_path("pages/New"), "pages/New.md");
}

#[test]
fn backlink_partition_and_relevance_are_stable() {
    let rows = vec![
        BacklinkRow {
            source_id: "a".to_string(),
            source_page: "pages/A.md".to_string(),
            context: "links [[Fixture Note]]".to_string(),
            is_linked: true,
        },
        BacklinkRow {
            source_id: "b".to_string(),
            source_page: "pages/B.md".to_string(),
            context: "mentions Fixture Note".to_string(),
            is_linked: false,
        },
    ];
    let (linked, unlinked) = group_backlinks(rows);
    assert_eq!(linked.len(), 1);
    assert_eq!(unlinked.len(), 1);
    assert_eq!(linked[0].source_id, "a");
    assert_eq!(unlinked[0].source_id, "b");
    assert_eq!(clamp_relevance(82.4), 82);
    assert_eq!(clamp_relevance(150.0), 100);
    assert_eq!(clamp_relevance(f64::NAN), 0);
}

#[test]
fn suggestion_state_transitions_are_explicit() {
    assert!(SuggestionsState::Idle.rows().is_none());
    assert!(SuggestionsState::Loading.is_loading());
    let rows = vec![ConnectionRow {
        title: "Reading List".to_string(),
        page_path: "pages/Reading List.md".to_string(),
        score: 82.0,
        snippet: "Shared reading queue".to_string(),
    }];
    let loaded = SuggestionsState::Loaded(rows);
    assert_eq!(loaded.rows().map(|rows| rows.len()), Some(1));
    let error = SuggestionsState::Error("failed".to_string());
    assert_eq!(error.error(), Some("failed"));
}

#[test]
fn fixture_page_seam_is_deterministic() {
    let provider = FixtureProvider::new();
    let info = provider.page_info(FIXTURE_PAGE).expect("fixture page");
    assert_eq!(info.path, FIXTURE_PAGE);
    assert_eq!(info.title.as_deref(), Some("Fixture Note"));
    assert_eq!(info.block_count, 12);

    let backlinks = provider.backlinks(FIXTURE_PAGE);
    let (linked, unlinked) = group_backlinks(backlinks);
    assert_eq!(linked.len(), 2);
    assert_eq!(unlinked.len(), 1);
    assert!(linked
        .iter()
        .any(|row| row.source_page == FIXTURE_BACKLINK_SOURCE));

    let snippet = provider
        .backlink_snippet(FIXTURE_BACKLINK_SOURCE, FIXTURE_BACKLINK_ANCHOR)
        .expect("fixture snippet");
    assert!(!snippet.anchor_content.is_empty());
    assert!(!snippet.context.is_empty());

    let suggestions = provider
        .suggest_connections(FIXTURE_PAGE)
        .expect("fixture suggestions");
    assert_eq!(suggestions.len(), 2);
    assert!(suggestions
        .iter()
        .all(|row| clamp_relevance(row.score) <= 100));

    provider
        .insert_block(FIXTURE_PAGE, "See also: [[Reading List]]")
        .expect("insert block");
    let blocks = provider.load_blocks(FIXTURE_PAGE).expect("fixture blocks");
    assert_eq!(blocks.len(), 13);
    assert!(blocks
        .iter()
        .any(|block| block.content == "See also: [[Reading List]]"));
}

#[test]
fn fixture_highlight_block_is_a_real_page_block() {
    let provider = FixtureProvider::new();
    let blocks = provider.load_blocks(FIXTURE_PAGE).expect("fixture page");
    assert!(
        blocks
            .iter()
            .any(|block| block.id == FIXTURE_HIGHLIGHT_BLOCK),
        "STRATUM_HIGHLIGHT target must exist on the fixture page"
    );
}

#[test]
fn fixture_move_and_delete_update_the_page_set() {
    let provider = FixtureProvider::new();
    let moved = provider
        .move_page(FIXTURE_PAGE, "pages/Renamed.md")
        .expect("fixture move");
    assert_eq!(moved, "pages/Renamed.md");
    assert!(provider.page_info(FIXTURE_PAGE).is_none());
    assert!(provider.page_info("pages/Renamed.md").is_some());
    provider.delete_page("pages/Renamed.md").expect("delete");
    assert!(provider.page_info("pages/Renamed.md").is_none());
}
