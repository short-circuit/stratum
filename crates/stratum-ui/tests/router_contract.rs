//! Route, navigation and shortcut contract tests.
//!
//! These assertions pin the gpui shell to the shipped React router and
//! sidebar order. They live in an integration test so the source modules stay
//! under the repository's 500-line limit.

use stratum_ui::router::{
    keystroke_string, parse_path, parse_target, path_for, percent_decode, percent_encode, Modifier,
    Panel, ShortcutAction, NAV_ITEMS, ROUTES, SHORTCUTS,
};

#[test]
fn thirteen_unique_panels_and_paths() {
    assert_eq!(Panel::ALL.len(), 13);
    assert_eq!(ROUTES.len(), 13);
    let mut panels = std::collections::BTreeSet::new();
    let mut paths = std::collections::BTreeSet::new();
    for panel in Panel::ALL {
        assert!(
            panels.insert(panel.id()),
            "duplicate panel id {}",
            panel.id()
        );
    }
    for route in ROUTES {
        assert!(paths.insert(route.path), "duplicate route {}", route.path);
    }
}

#[test]
fn nav_items_match_the_shipped_order() {
    assert_eq!(NAV_ITEMS.len(), 12);
    let expected = [
        "journal",
        "graph",
        "kanban",
        "reminders",
        "search",
        "query",
        "asknotes",
        "templates",
        "flashcards",
        "whiteboards",
        "plugins",
        "settings",
    ];
    let actual: Vec<_> = NAV_ITEMS.iter().map(|item| item.id).collect();
    assert_eq!(actual, expected);
    let expected_labels = [
        "mobile_nav.journal",
        "mobile_nav.graph",
        "mobile_nav.kanban",
        "mobile_nav.reminders",
        "mobile_nav.search",
        "mobile_nav.query",
        "mobile_nav.ask_notes",
        "mobile_nav.templates",
        "mobile_nav.flashcards",
        "mobile_nav.whiteboards",
        "mobile_nav.plugins",
        "mobile_nav.settings",
    ];
    let actual_labels: Vec<_> = NAV_ITEMS.iter().map(|item| item.label_key).collect();
    assert_eq!(actual_labels, expected_labels);
}

#[test]
fn every_nav_panel_is_a_route() {
    for item in NAV_ITEMS {
        assert!(ROUTES.iter().any(|route| route.panel == item.panel));
    }
}

#[test]
fn parse_path_reaches_every_route() {
    assert_eq!(parse_path("/"), Some(Panel::Journal));
    assert_eq!(parse_path("/ask-notes"), Some(Panel::AskNotes));
    assert_eq!(parse_path("/page/pages/My%20Note.md"), Some(Panel::Page));
    for route in ROUTES {
        assert_eq!(parse_path(route.path), Some(route.panel), "{}", route.path);
    }
    assert_eq!(parse_path("/unknown"), None);
}

#[test]
fn path_for_round_trips() {
    for route in ROUTES {
        if route.panel == Panel::Page {
            let page = "pages/My Note.md";
            let path = path_for(route.panel, Some(page));
            assert_eq!(parse_path(&path), Some(Panel::Page));
            assert_eq!(percent_decode(path.strip_prefix("/page/").unwrap()), page);
        } else {
            let path = path_for(route.panel, None);
            assert_eq!(parse_path(&path), Some(route.panel));
        }
    }
}

#[test]
fn percent_codec_round_trips() {
    for value in ["plain", "pages/My Note.md", "héllo wörld", "a+b&c"] {
        assert_eq!(percent_decode(&percent_encode(value)), value);
    }
    assert_eq!(percent_decode("bad%zz"), "bad%zz");
}

#[test]
fn parse_target_keeps_page_queries() {
    let target = parse_target("/page/foo.md?x=1").expect("page target");
    assert_eq!(target.panel, Panel::Page);
    assert_eq!(target.param("x"), Some("1"));
    // `parse_path` strips the query for callers that only need the panel.
    assert_eq!(parse_path("/page/foo.md?x=1"), Some(Panel::Page));
}

#[test]
fn parse_target_reads_whiteboard_deep_links() {
    let target = parse_target(
        "/whiteboards?board=Architecture%20sketch&tool=frame&library=1&export=svg\
         &element=fixture-flow-left",
    )
    .expect("whiteboards target");
    assert_eq!(target.panel, Panel::Whiteboards);
    assert_eq!(target.param("board"), Some("Architecture sketch"));
    assert_eq!(target.param("tool"), Some("frame"));
    assert_eq!(target.param("library"), Some("1"));
    assert_eq!(target.param("export"), Some("svg"));
    assert_eq!(target.param("element"), Some("fixture-flow-left"));
}

#[test]
fn parse_target_decodes_percent_encoded_spaces() {
    let target = parse_target("/whiteboards?board=My+Board&tool=frame").expect("target");
    assert_eq!(target.param("board"), Some("My Board"));
    let target = parse_target("/page/pages/My%20Note.md").expect("page target");
    assert_eq!(target.panel, Panel::Page);
    assert!(target.query.is_empty());
}

#[test]
fn parse_target_rejects_unknown_paths() {
    assert!(parse_target("/unknown").is_none());
    assert!(parse_target("/unknown?x=1").is_none());
}

#[test]
fn shortcut_table_contains_read_mode_and_sidebar() {
    let read = SHORTCUTS
        .iter()
        .find(|shortcut| shortcut.action == ShortcutAction::ToggleReadMode)
        .expect("read-mode shortcut exists");
    assert_eq!(read.key, "r");
    assert!(read.modifiers.contains(&Modifier::Ctrl));
    assert!(read.modifiers.contains(&Modifier::Shift));
    assert!(SHORTCUTS
        .iter()
        .any(|shortcut| shortcut.action == ShortcutAction::ToggleSidebar));
    assert!(SHORTCUTS
        .iter()
        .any(|shortcut| shortcut.action == ShortcutAction::DismissOverlay));
}

#[test]
fn keystrokes_use_the_secondary_modifier() {
    let read = SHORTCUTS
        .iter()
        .find(|shortcut| shortcut.action == ShortcutAction::ToggleReadMode)
        .expect("read-mode shortcut exists");
    assert_eq!(keystroke_string(read), "secondary-shift-r");
    let sidebar = SHORTCUTS
        .iter()
        .find(|shortcut| shortcut.action == ShortcutAction::ToggleSidebar)
        .expect("sidebar shortcut exists");
    assert_eq!(keystroke_string(sidebar), "secondary-b");
    let dismiss = SHORTCUTS
        .iter()
        .find(|shortcut| shortcut.action == ShortcutAction::DismissOverlay)
        .expect("dismiss shortcut exists");
    assert_eq!(keystroke_string(dismiss), "escape");
}
