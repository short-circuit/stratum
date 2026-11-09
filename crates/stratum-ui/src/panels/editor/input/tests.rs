//! Unit tests for the caret and overlay token helpers.

use ::core::prelude::v1::test;

use stratum_editor::menus::WikiPage;

use super::mouse::{wiki_click, WikiClick};
use super::overlay_ops::{marker_token_start, slash_token_start};
use super::{next_char_boundary, prev_char_boundary};

fn wiki_page(path: &str, slug: &str, title: &str) -> WikiPage {
    WikiPage {
        path: path.to_string(),
        slug: slug.to_string(),
        title: title.to_string(),
    }
}

#[test]
fn wiki_click_navigates_to_existing_pages() {
    let pages = vec![
        wiki_page("pages/alpha.md", "alpha", "Alpha"),
        wiki_page("pages/notes/beta.md", "beta", "Beta"),
    ];
    assert_eq!(
        wiki_click("alpha", &pages),
        WikiClick::Navigate("pages/alpha.md".to_string())
    );
    assert_eq!(
        wiki_click("Alpha", &pages),
        WikiClick::Navigate("pages/alpha.md".to_string())
    );
    assert_eq!(
        wiki_click("pages/notes/beta.md", &pages),
        WikiClick::Navigate("pages/notes/beta.md".to_string())
    );
}

#[test]
fn wiki_click_reports_missing_targets_as_dead_links() {
    let pages = vec![wiki_page("pages/alpha.md", "alpha", "Alpha")];
    assert_eq!(
        wiki_click("gamma", &pages),
        WikiClick::DeadLink("gamma".to_string())
    );
    assert_eq!(
        wiki_click("  missing  ", &pages),
        WikiClick::DeadLink("missing".to_string())
    );
}

#[test]
fn slash_token_is_word_anchored() {
    assert_eq!(slash_token_start("hello /head", 11), Some(6));
    assert_eq!(slash_token_start("a/b", 3), None);
}

#[test]
fn marker_token_requires_a_non_empty_body() {
    assert_eq!(marker_token_start("task :todo", 10), Some(5));
    assert_eq!(marker_token_start("task:", 5), None);
}

#[test]
fn char_boundaries_stay_on_char_boundaries() {
    let text = "héllo";
    assert_eq!(prev_char_boundary(text, 2), 1);
    assert_eq!(next_char_boundary(text, 1), 3);
    assert_eq!(next_char_boundary(text, text.len()), text.len());
}

#[test]
fn caret_helpers_clamp_to_the_text_length() {
    assert_eq!(prev_char_boundary("", 0), 0);
    assert_eq!(next_char_boundary("", 0), 0);
}
