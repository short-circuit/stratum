//! Integration coverage for the AI slash actions and the quote block action:
//! request mapping, page replacement with frontmatter preservation, Mermaid
//! sibling insertion and byte-stable quote round-trips.

use pkm_block::Block;
use stratum_editor::menus::{
    ai_call_for, slash_items, slash_row_count, AiCallScope, SlashAction, SLASH_MAX_ROWS,
};
use stratum_editor::{EditorDocument, EditorState};
use uuid::Uuid;

fn block(id: Uuid, content: &str) -> Block {
    Block::new(id, content.to_string())
}

#[test]
fn slash_catalogue_row_bound_matches_every_context() {
    for items in [
        slash_items(false, false),
        slash_items(false, true),
        slash_items(true, false),
        slash_items(true, true),
    ] {
        assert_eq!(slash_row_count(items.len()), items.len());
        assert!(items.len() <= SLASH_MAX_ROWS);
        // Every action id in the catalogue resolves to a SlashAction, so the
        // view's `apply_slash` lookup can never miss a drawn row.
        for item in &items {
            assert!(
                SlashAction::ALL.iter().any(|action| action.id() == item.id),
                "unresolvable id {}",
                item.id
            );
        }
    }
}

#[test]
fn every_ai_action_produces_a_serviceable_call() {
    let page_text = "alpha\nbeta";
    let page_markdown = "- alpha\n- beta";
    let path = "journals/2026-11-09.md";
    for action in [
        SlashAction::AiRewrite,
        SlashAction::AiFormatSelection,
        SlashAction::AiSummarizeSelection,
        SlashAction::AiStructureJournal,
        SlashAction::AiStructurePage,
        SlashAction::AiFormatJournalNotes,
        SlashAction::AiSummarizePage,
        SlashAction::AiResearch,
        SlashAction::AiInterlink,
        SlashAction::AiGenerateMermaid,
    ] {
        let call = ai_call_for(
            action,
            Some("selected text"),
            page_text,
            page_markdown,
            path,
        )
        .unwrap_or_else(|| panic!("no call for {action:?}"));
        assert!(!call.text.trim().is_empty(), "{action:?} empty prompt");
        if matches!(
            action,
            SlashAction::AiRewrite
                | SlashAction::AiFormatSelection
                | SlashAction::AiSummarizeSelection
                | SlashAction::AiStructureJournal
                | SlashAction::AiStructurePage
                | SlashAction::AiFormatJournalNotes
                | SlashAction::AiSummarizePage
                | SlashAction::AiInterlink
        ) {
            assert_eq!(call.page_path.as_deref(), Some(path), "{action:?}");
        }
        if action != SlashAction::AiGenerateMermaid {
            assert_ne!(call.scope, AiCallScope::InsertBlockAfterCaret, "{action:?}");
        }
    }
    // Empty page + no selection: page actions must not fire.
    assert!(ai_call_for(SlashAction::AiSummarizePage, None, "   ", "", path).is_none());
    assert!(ai_call_for(SlashAction::AiRewrite, None, page_text, page_markdown, path).is_none());
}

#[test]
fn quote_content_round_trips_byte_stable() {
    let id = Uuid::new_v4();
    let doc = EditorDocument::from_blocks("pages/x.md", vec![block(id, "> quoted line")]).unwrap();
    let markdown = doc.markdown();
    let reopened = EditorDocument::open("pages/x.md", &markdown).unwrap();
    assert_eq!(reopened.markdown(), markdown);
    assert_eq!(reopened.blocks()[0].content, "> quoted line");
}

#[test]
fn page_replacement_preserves_frontmatter_and_undoes() {
    let raw = "---\ntitle: Notes\n---\n\n- old line\n";
    let mut state = EditorState::new(EditorDocument::open("pages/x.md", raw).unwrap());
    let original = state.document().markdown();
    let ai = "- ## Summary\n- first point\n- ```mermaid\n  graph TD\n  A-->B\n  ```\n";
    state.replace_from_markdown(ai).unwrap();
    let replaced = state.document().markdown();
    assert!(replaced.starts_with("---\ntitle: Notes\n---"), "{replaced}");
    let reopened = EditorDocument::open("pages/x.md", &replaced).unwrap();
    assert_eq!(
        reopened.markdown(),
        replaced,
        "AI page body must round-trip"
    );
    assert!(state.undo().unwrap());
    assert_eq!(state.document().markdown(), original);
}

#[test]
fn mermaid_insert_keeps_the_tree_consistent() {
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let mut tail = block(b, "tail");
    tail.left_id = Some(a);
    let mut state = EditorState::new(
        EditorDocument::from_blocks("pages/x.md", vec![block(a, "head"), tail]).unwrap(),
    );
    state.set_caret(a, 4).unwrap();
    let before = state.document().markdown();
    state
        .insert_markdown_after_caret("```mermaid\ngraph TD\nA-->B\n```")
        .unwrap();
    for block in state.document().blocks() {
        if let Some(parent) = block.parent_id {
            assert!(state.document().block(parent).is_some(), "dangling parent");
        }
        if let Some(left) = block.left_id {
            assert!(state.document().block(left).is_some(), "dangling left");
        }
    }
    let markdown = state.document().markdown();
    let reopened = EditorDocument::open("pages/x.md", &markdown).unwrap();
    assert_eq!(reopened.markdown(), markdown);
    assert!(state.undo().unwrap());
    assert_eq!(state.document().markdown(), before);
}
