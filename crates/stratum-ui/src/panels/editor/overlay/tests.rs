//! Overlay detection and selection tests.

use ::core::prelude::v1::test;

use super::*;

#[test]
fn slash_detection_is_word_anchored() {
    assert!(matches!(
        detect_overlay("/", 1, false, false),
        OverlayState::Slash { .. }
    ));
    assert!(matches!(
        detect_overlay("hello /head", 11, false, false),
        OverlayState::Slash { .. }
    ));
    assert_eq!(detect_overlay("a/b", 3, false, false), OverlayState::None);
}

#[test]
fn wiki_detection_returns_the_open_query() {
    assert_eq!(
        detect_overlay("see [[Pro", 9, false, false),
        OverlayState::Wiki {
            query: "Pro".to_string(),
            selected: 0
        }
    );
}

#[test]
fn marker_detection_needs_a_non_empty_token() {
    assert_eq!(
        detect_overlay("task :", 6, false, false),
        OverlayState::Marker { selected: 0 }
    );
    assert_eq!(detect_overlay("task:", 5, false, false), OverlayState::None);
}

#[test]
fn selection_wraps_at_both_ends() {
    let mut overlay = OverlayState::Slash {
        query: String::new(),
        selected: 0,
    };
    overlay.move_selection(-1, 3);
    assert_eq!(overlay.selected(), Some(2));
    overlay.move_selection(1, 3);
    assert_eq!(overlay.selected(), Some(0));
}

#[test]
fn wiki_rows_cover_every_selectable_entry() {
    assert_eq!(wiki_row_count(0), 1);
    assert_eq!(wiki_row_count(3), 3);
    assert_eq!(wiki_row_count(20), 8);
}
