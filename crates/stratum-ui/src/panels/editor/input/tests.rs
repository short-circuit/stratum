//! Unit tests for the caret and overlay token helpers.

use ::core::prelude::v1::test;

use stratum_editor::menus::{SlashAction, WikiPage};

use super::mouse::{wiki_click, WikiClick};
use super::overlay_ops::{marker_token_start, slash_caret_offset, slash_token_start};
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
fn slash_fenced_actions_place_the_caret_inside_the_fence() {
    assert_eq!(
        slash_caret_offset(Some(SlashAction::CodeBlock), "```\n\n```", 0),
        4
    );
    assert_eq!(
        slash_caret_offset(Some(SlashAction::Mermaid), "```mermaid\n\n```", 0),
        11
    );
    assert_eq!(
        slash_caret_offset(Some(SlashAction::MathEquation), "$$\n\n$$", 0),
        3
    );
    assert_eq!(slash_caret_offset(Some(SlashAction::Quote), "> body", 0), 2);
    assert_eq!(
        slash_caret_offset(Some(SlashAction::Paragraph), "abc", 5),
        3
    );
}

#[test]
fn table_action_places_the_caret_inside_the_first_header_cell() {
    let skeleton = stratum_editor::menus::table_skeleton();
    assert_eq!(
        slash_caret_offset(Some(SlashAction::Table), &skeleton, 0),
        2
    );
}

#[test]
fn typing_at_the_slash_caret_stays_inside_the_fence() {
    use stratum_editor::{EditorDocument, EditorState};

    fn typed(replacement: &str, action: SlashAction, text: &str) -> String {
        let doc = EditorDocument::open("pages/x.md", "/").expect("doc opens");
        let id = doc.ordered_ids()[0];
        let mut state = EditorState::new(doc);
        state
            .replace_block_content(id, replacement.to_string())
            .unwrap();
        let offset = slash_caret_offset(Some(action), replacement, 0);
        state.set_caret(id, offset).unwrap();
        state.insert_text(text).unwrap();
        state.document().blocks()[0].content.clone()
    }

    assert_eq!(
        typed("```\n\n```", SlashAction::CodeBlock, "graph TD"),
        "```\ngraph TD\n```"
    );
    assert_eq!(
        typed("```mermaid\n\n```", SlashAction::Mermaid, "graph TD"),
        "```mermaid\ngraph TD\n```"
    );
    assert_eq!(
        typed("$$\n\n$$", SlashAction::MathEquation, "x^2"),
        "$$\nx^2\n$$"
    );
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
