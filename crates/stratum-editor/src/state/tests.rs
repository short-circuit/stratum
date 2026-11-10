use pkm_block::{Block, Priority, TaskMarker};
use uuid::Uuid;

use super::*;
use crate::document::EditorDocument;
use crate::inline::SpanKind;
use crate::menus::{AiCall, AiCallKind, AiCallScope, AiTransformAction};

fn doc_with(blocks: Vec<Block>) -> EditorDocument {
    EditorDocument::from_blocks("pages/x.md", blocks).unwrap()
}

fn block(id: Uuid, content: &str) -> Block {
    Block::new(id, content.to_string())
}

fn caret(block_id: Uuid, byte_offset: usize) -> Caret {
    Caret {
        block_id,
        byte_offset,
    }
}

fn dto(id: &str, content: &str) -> crate::dto::BlockDto {
    crate::dto::BlockDto {
        id: id.into(),
        content: content.into(),
        parent_id: None,
        left_id: None,
        properties: Vec::new(),
        marker: None,
        priority: None,
        collapsed: false,
        heading_level: None,
    }
}

#[test]
fn typing_coalesces_and_undo_removes_the_run() {
    let id = Uuid::new_v4();
    let mut state = EditorState::new(doc_with(vec![block(id, "")]));
    state.set_caret(id, 0).unwrap();
    state.insert_text("h").unwrap();
    state.insert_text("i").unwrap();
    assert_eq!(state.history_depth(), 1);
    assert_eq!(state.document().block(id).unwrap().content, "hi");

    assert!(state.undo().unwrap());
    assert_eq!(state.document().block(id).unwrap().content, "");
    assert_eq!(state.history_depth(), 0);

    assert!(state.redo().unwrap());
    assert_eq!(state.document().block(id).unwrap().content, "hi");
}

#[test]
fn non_contiguous_typing_is_not_coalesced() {
    let id = Uuid::new_v4();
    let mut state = EditorState::new(doc_with(vec![block(id, "ab")]));
    state.set_caret(id, 0).unwrap();
    state.insert_text("x").unwrap();
    state.set_caret(id, 3).unwrap();
    state.insert_text("y").unwrap();
    assert_eq!(state.history_depth(), 2);
}

#[test]
fn indent_undo_restores_parent_and_left_links() {
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let mut second = block(b, "b");
    second.left_id = Some(a);
    let mut state = EditorState::new(doc_with(vec![block(a, "a"), second]));
    state.set_caret(b, 0).unwrap();
    state.indent().unwrap();
    assert_eq!(state.document().block(b).unwrap().parent_id, Some(a));

    assert!(state.undo().unwrap());
    let restored = state.document().block(b).unwrap();
    assert_eq!(restored.parent_id, None);
    assert_eq!(restored.left_id, Some(a));
}

#[test]
fn outdent_undo_restores_parent_and_left_links() {
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let c = Uuid::new_v4();
    let mut child_b = block(b, "b");
    child_b.parent_id = Some(a);
    let mut child_c = block(c, "c");
    child_c.parent_id = Some(a);
    child_c.left_id = Some(b);
    let mut state = EditorState::new(doc_with(vec![block(a, "a"), child_b, child_c]));
    state.set_caret(b, 0).unwrap();
    state.outdent().unwrap();
    assert_eq!(state.document().block(b).unwrap().parent_id, None);
    assert_eq!(state.document().block(b).unwrap().left_id, Some(a));

    assert!(state.undo().unwrap());
    let restored = state.document().block(b).unwrap();
    assert_eq!(restored.parent_id, Some(a));
    assert_eq!(restored.left_id, None);
}

#[test]
fn split_undo_merges_the_new_block_back() {
    let id = Uuid::new_v4();
    let mut state = EditorState::new(doc_with(vec![block(id, "hello world")]));
    state.set_caret(id, 5).unwrap();
    state.insert_newline().unwrap();
    assert_eq!(state.document().blocks().len(), 2);
    assert_eq!(state.document().block(id).unwrap().content, "hello");

    assert!(state.undo().unwrap());
    assert_eq!(state.document().blocks().len(), 1);
    assert_eq!(state.document().block(id).unwrap().content, "hello world");
}

#[test]
fn delete_undo_restores_the_deleted_subtree() {
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let d = Uuid::new_v4();
    let mut second = block(b, "y");
    second.left_id = Some(a);
    let mut child = block(d, "child");
    child.parent_id = Some(b);
    let mut state = EditorState::new(doc_with(vec![block(a, "x"), second, child]));
    state.set_caret(a, 1).unwrap();
    state.delete_forward().unwrap();
    assert!(state.document().block(b).is_none());
    assert_eq!(state.document().block(d).unwrap().parent_id, Some(a));
    assert_eq!(state.document().block(a).unwrap().content, "xy");

    assert!(state.undo().unwrap());
    let restored_b = state.document().block(b).unwrap();
    assert_eq!(restored_b.content, "y");
    let restored_child = state.document().block(d).unwrap();
    assert_eq!(restored_child.parent_id, Some(b));
}

#[test]
fn merge_undo_restores_the_merged_block() {
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let mut second = block(b, "tail");
    second.left_id = Some(a);
    let mut state = EditorState::new(doc_with(vec![block(a, "head"), second]));
    state.set_caret(b, 0).unwrap();
    state.backspace().unwrap();
    assert!(state.document().block(b).is_none());
    assert_eq!(state.document().block(a).unwrap().content, "headtail");

    assert!(state.undo().unwrap());
    assert_eq!(state.document().block(a).unwrap().content, "head");
    assert_eq!(state.document().block(b).unwrap().content, "tail");
    assert_eq!(state.document().block(b).unwrap().left_id, Some(a));
}

#[test]
fn markers_and_priorities_undo() {
    let id = Uuid::new_v4();
    let mut state = EditorState::new(doc_with(vec![block(id, "task")]));
    state.set_caret(id, 0).unwrap();
    state.set_marker(Some(TaskMarker::Todo)).unwrap();
    state.set_priority(Some(Priority::A)).unwrap();
    assert_eq!(
        state.document().block(id).unwrap().marker,
        Some(TaskMarker::Todo)
    );
    assert_eq!(
        state.document().block(id).unwrap().priority,
        Some(Priority::A)
    );

    assert!(state.undo().unwrap());
    assert_eq!(state.document().block(id).unwrap().priority, None);
    assert!(state.undo().unwrap());
    assert_eq!(state.document().block(id).unwrap().marker, None);
}

#[test]
fn apply_span_wraps_and_unwraps() {
    let id = Uuid::new_v4();
    let mut state = EditorState::new(doc_with(vec![block(id, "bold")]));
    state.set_caret(id, 0).unwrap();
    state.apply_span(id, 0..4, SpanKind::Bold).unwrap();
    assert_eq!(state.document().block(id).unwrap().content, "**bold**");
    state.apply_span(id, 2..6, SpanKind::Bold).unwrap();
    assert_eq!(state.document().block(id).unwrap().content, "bold");
}

#[test]
fn copy_cut_and_paste_selection() {
    let id = Uuid::new_v4();
    let mut state = EditorState::new(doc_with(vec![block(id, "hello world")]));
    let anchor = Caret {
        block_id: id,
        byte_offset: 0,
    };
    let head = Caret {
        block_id: id,
        byte_offset: 5,
    };
    state.set_selection(anchor, head).unwrap();
    let payload = state.copy_selection();
    assert_eq!(payload.plain_text, "hello");

    let payload = state.cut_selection().unwrap();
    assert_eq!(payload.plain_text, "hello");
    assert_eq!(state.document().block(id).unwrap().content, " world");

    state.paste_payload(&payload).unwrap();
    assert_eq!(state.document().block(id).unwrap().content, "hello world");
}

#[test]
fn paste_multiple_blocks_preserves_markers_and_properties() {
    let id = Uuid::new_v4();
    let mut state = EditorState::new(doc_with(vec![block(id, "start")]));
    state.set_caret(id, 5).unwrap();
    let payload = ClipboardPayload {
        plain_text: "one\ntwo".into(),
        blocks: vec![
            crate::dto::BlockDto {
                id: "11111111-1111-4111-8111-111111111111".into(),
                content: "one".into(),
                parent_id: None,
                left_id: None,
                properties: vec![("alpha".into(), "1".into())],
                marker: Some("TODO".into()),
                priority: Some("A".into()),
                collapsed: false,
                heading_level: Some(2),
            },
            crate::dto::BlockDto {
                id: "22222222-2222-4222-8222-222222222222".into(),
                content: "two".into(),
                parent_id: None,
                left_id: None,
                properties: Vec::new(),
                marker: None,
                priority: None,
                collapsed: false,
                heading_level: None,
            },
        ],
    };
    state.paste_payload(&payload).unwrap();
    let blocks = state.document().blocks();
    let one = blocks
        .iter()
        .find(|b| b.content == "one")
        .expect("pasted block one");
    assert_eq!(one.marker, Some(TaskMarker::Todo));
    assert_eq!(one.priority, Some(Priority::A));
    assert_eq!(one.properties.get("alpha").map(String::as_str), Some("1"));
    assert_eq!(one.meta.heading_level, Some(2));
    assert!(blocks.iter().any(|b| b.content == "two"));
}

#[test]
fn ime_composition_commits_one_history_entry() {
    let id = Uuid::new_v4();
    let mut state = EditorState::new(doc_with(vec![block(id, "ab")]));
    state.set_caret(id, 0).unwrap();
    state.ime_replace(0..0, "日本").unwrap();
    assert_eq!(state.document().block(id).unwrap().content, "日本ab");
    assert_eq!(state.marked_range_utf16(), Some((0, 2)));
    state.ime_replace(0..2, "日本語").unwrap();
    assert_eq!(state.document().block(id).unwrap().content, "日本語ab");
    assert_eq!(state.marked_range_utf16(), Some((0, 3)));
    state.ime_commit().unwrap();
    assert_eq!(state.history_depth(), 1);
    assert!(state.undo().unwrap());
    assert_eq!(state.document().block(id).unwrap().content, "ab");
}

#[test]
fn ime_cancel_restores_original_content_without_history() {
    let id = Uuid::new_v4();
    let mut state = EditorState::new(doc_with(vec![block(id, "ab")]));
    state.set_caret(id, 1).unwrap();
    state.ime_replace(1..1, "😀").unwrap();
    assert_eq!(state.document().block(id).unwrap().content, "a😀b");
    state.ime_cancel().unwrap();
    assert_eq!(state.document().block(id).unwrap().content, "ab");
    assert_eq!(state.history_depth(), 0);
}

#[test]
fn utf16_offsets_reject_surrogate_interiors() {
    let text = "a😀b";
    assert_eq!(utf16_to_byte(text, 0), Some(0));
    assert_eq!(utf16_to_byte(text, 1), Some(1));
    // UTF-16 units: 'a' = 0, emoji = 1..3, 'b' = 3.
    assert_eq!(utf16_to_byte(text, 2), None);
    assert_eq!(utf16_to_byte(text, 3), Some(5));
    assert_eq!(utf16_to_byte(text, 4), Some(6));
    assert_eq!(utf16_to_byte(text, 5), None);
    assert_eq!(utf16_to_byte("😀", 1), None);

    assert_eq!(byte_to_utf16(text, 0), Some(0));
    assert_eq!(byte_to_utf16(text, 1), Some(1));
    assert_eq!(byte_to_utf16(text, 5), Some(3));
    assert_eq!(byte_to_utf16(text, 6), Some(4));
    assert_eq!(byte_to_utf16(text, 2), None);
}

#[test]
fn normalize_paste_table() {
    assert_eq!(normalize_paste("a\r\nb"), "a\nb");
    assert_eq!(normalize_paste("a\rb"), "a\nb");
    assert_eq!(normalize_paste("a\0b"), "ab");
    assert_eq!(normalize_paste("\tx"), "    x");
    assert_eq!(normalize_paste("a   "), "a");
    assert_eq!(normalize_paste("a\n\n\n\nb"), "a\n\n\nb");
    assert_eq!(normalize_paste("```\n\ta\n```"), "```\n\ta\n```");
    assert_eq!(normalize_paste("x\n\n\n").trim_end(), "x");
    assert_eq!(
        normalize_paste(&"x".repeat(100_001)).chars().count(),
        100_000
    );
}

#[test]
fn paste_markdown_builds_indented_blocks() {
    let blocks = paste_markdown("- a\n  - b\n- c");
    assert_eq!(blocks.len(), 3);
    assert_eq!(blocks[0].content, "a");
    assert_eq!(blocks[1].content, "b");
    assert_eq!(blocks[1].parent_id.as_deref(), Some(blocks[0].id.as_str()));
    assert_eq!(blocks[2].content, "c");
    assert_eq!(blocks[2].parent_id, None);
    assert_eq!(blocks[2].left_id.as_deref(), Some(blocks[0].id.as_str()));
}

#[test]
fn cross_block_selection_deletes_the_range() {
    let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
    let mut second = block(b, "second");
    second.left_id = Some(a);
    let mut state = EditorState::new(doc_with(vec![block(a, "first"), second]));
    state.set_selection(caret(a, 0), caret(b, 6)).unwrap();
    let payload = state.cut_selection().unwrap();
    assert_eq!(payload.blocks.len(), 2);
    assert_eq!(state.document().block(a).unwrap().content, "");
    assert!(state.document().block(b).is_none());
    assert!(state.undo().unwrap());
    assert_eq!(state.document().block(a).unwrap().content, "first");
    assert_eq!(state.document().block(b).unwrap().content, "second");
    assert_eq!(state.document().block(b).unwrap().left_id, Some(a));
}

#[test]
fn cross_block_delete_keeps_a_usable_caret_and_is_undoable() {
    let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
    let mut second = block(b, "second");
    second.left_id = Some(a);
    let mut state = EditorState::new(doc_with(vec![block(a, "first"), second]));
    state.set_selection(caret(a, 0), caret(b, 6)).unwrap();
    state.insert_text("x").unwrap();
    assert!(state.document().block(b).is_none());
    assert_eq!(state.document().block(a).unwrap().content, "x");
    assert!(state.undo().unwrap());
    assert_eq!(state.document().block(a).unwrap().content, "");
    assert!(state.undo().unwrap());
    assert_eq!(state.document().block(a).unwrap().content, "first");
    let restored = state.document().block(b).unwrap();
    assert_eq!(restored.content, "second");
    assert_eq!(restored.left_id, Some(a));
    assert!(state.redo().unwrap());
    assert!(state.redo().unwrap());
    assert_eq!(state.document().block(a).unwrap().content, "x");
    assert!(state.document().block(b).is_none());
}

#[test]
fn cross_block_delete_preserves_unselected_text() {
    let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
    let mut second = block(b, "tail");
    second.left_id = Some(a);
    let mut state = EditorState::new(doc_with(vec![block(a, "hello world"), second]));
    state.set_selection(caret(a, 6), caret(b, 0)).unwrap();
    state.cut_selection().unwrap();
    assert!(state.document().block(b).is_none());
    assert_eq!(state.document().block(a).unwrap().content, "hello tail");
}

#[test]
fn cross_block_delete_rehomes_tail_children_and_undo_restores_them() {
    let (a, b, c) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    let mut second = block(b, "bb");
    second.left_id = Some(a);
    let mut child = block(c, "cc");
    child.parent_id = Some(b);
    let mut state = EditorState::new(doc_with(vec![block(a, "aa"), second, child]));
    state.set_selection(caret(a, 0), caret(b, 2)).unwrap();
    state.cut_selection().unwrap();
    assert!(state.document().block(b).is_none());
    assert_eq!(state.document().block(c).unwrap().parent_id, Some(a));
    assert_eq!(state.document().block(a).unwrap().content, "");
    assert!(state.undo().unwrap());
    assert_eq!(state.document().block(b).unwrap().left_id, Some(a));
    let restored_c = state.document().block(c).unwrap();
    assert_eq!(restored_c.parent_id, Some(b));
    assert_eq!(restored_c.left_id, None);
    assert_eq!(state.document().block(a).unwrap().content, "aa");
}

#[test]
fn multi_block_paste_replaces_the_selection_and_is_undoable() {
    let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
    let mut second = block(b, "tail");
    second.left_id = Some(a);
    let mut state = EditorState::new(doc_with(vec![block(a, "head"), second]));
    state.set_selection(caret(a, 0), caret(b, 4)).unwrap();
    let payload = ClipboardPayload {
        plain_text: "one\ntwo".into(),
        blocks: vec![
            dto("11111111-1111-4111-8111-111111111111", "one"),
            dto("22222222-2222-4222-8222-222222222222", "two"),
        ],
    };
    state.paste_payload(&payload).unwrap();
    assert!(state.document().block(b).is_none());
    assert_eq!(state.document().block(a).unwrap().content, "");
    let blocks = state.document().blocks();
    let one = blocks
        .iter()
        .find(|candidate| candidate.content == "one")
        .unwrap();
    let two = blocks
        .iter()
        .find(|candidate| candidate.content == "two")
        .unwrap();
    assert_eq!(one.left_id, Some(a));
    assert_eq!(two.left_id, Some(one.id));
    let (one_id, two_id) = (one.id, two.id);
    let caret = state.caret().unwrap();
    assert_eq!(caret.block_id, two_id);
    assert!(state.document().block(caret.block_id).is_some());
    assert!(state.undo().unwrap());
    assert!(state.document().block(one_id).is_none());
    assert!(state.document().block(two_id).is_none());
}

#[test]
fn history_is_capped_at_200_entries() {
    let mut history = History::new();
    for i in 0..250 {
        let op = EditOp::InsertText {
            block_id: Uuid::new_v4(),
            offset: i * 10,
            text: "x".repeat(10),
        };
        history.push(op.clone(), op.inverse(), chrono::Utc::now());
    }
    assert_eq!(history.depth(), 200);
}

#[test]
fn selected_text_is_read_only_and_matches_the_selection() {
    let id = Uuid::new_v4();
    let mut state = EditorState::new(doc_with(vec![block(id, "hello world")]));
    assert_eq!(state.selected_text(), None);
    state.set_selection(caret(id, 0), caret(id, 5)).unwrap();
    assert_eq!(state.selected_text(), Some("hello".to_string()));
    assert!(state.clipboard().is_none());
}

const AI_REPLACEMENT: &str = "- new one\n  .id: 11111111-1111-4111-8111-111111111111\n- new two\n  .id: 22222222-2222-4222-8222-222222222222\n";

#[test]
fn replace_from_markdown_round_trips_and_undoes_in_one_entry() {
    let id = Uuid::new_v4();
    let mut state = EditorState::new(doc_with(vec![block(id, "old text")]));
    state.set_caret(id, 0).unwrap();
    let original = state.document().markdown();

    state.replace_from_markdown(AI_REPLACEMENT).unwrap();
    assert_eq!(state.document().markdown(), AI_REPLACEMENT);
    assert_eq!(state.history_depth(), 1);
    let caret = state.caret().expect("caret");
    assert_eq!(
        state.document().block(caret.block_id).unwrap().content,
        "new one"
    );

    assert!(state.undo().unwrap());
    assert_eq!(state.document().markdown(), original);
}

#[test]
fn replace_from_markdown_keeps_a_single_empty_block_for_an_empty_body() {
    let id = Uuid::new_v4();
    let mut state = EditorState::new(doc_with(vec![block(id, "old")]));
    state.replace_from_markdown("").unwrap();
    let blocks = state.document().blocks();
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].content, "");
    assert_eq!(state.history_depth(), 1);
}

#[test]
fn insert_markdown_after_caret_links_the_chain_and_undoes() {
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let mut second = block(b, "tail");
    second.left_id = Some(a);
    let mut state = EditorState::new(doc_with(vec![block(a, "head"), second]));
    state.set_caret(a, 4).unwrap();
    let before = state.document().markdown();
    let snippet = "- inserted\n  .id: 11111111-1111-4111-8111-111111111111\n";

    state.insert_markdown_after_caret(snippet).unwrap();
    for block in state.document().blocks() {
        if let Some(parent) = block.parent_id {
            assert!(
                state.document().block(parent).is_some(),
                "dangling parent_id"
            );
        }
        if let Some(left) = block.left_id {
            assert!(state.document().block(left).is_some(), "dangling left_id");
        }
    }
    let inserted = state
        .document()
        .blocks()
        .into_iter()
        .find(|block| block.content == "inserted")
        .expect("inserted block");
    assert_eq!(inserted.parent_id, None);
    assert_eq!(inserted.left_id, Some(a));
    assert_eq!(
        state.document().block(b).unwrap().left_id,
        Some(inserted.id)
    );
    assert_eq!(state.caret().unwrap().block_id, inserted.id);

    assert!(state.undo().unwrap());
    assert_eq!(state.document().markdown(), before);
}

#[test]
fn insert_asset_reference_adds_a_sibling_block_and_undoes() {
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let mut second = block(b, "tail");
    second.left_id = Some(a);
    let mut state = EditorState::new(doc_with(vec![block(a, "head"), second]));
    state.set_caret(a, 4).unwrap();
    let before = state.document().markdown();
    let reference = crate::assets::asset_reference("assets/pic.png");

    state.insert_asset_reference("assets/pic.png").unwrap();
    let ordered = state.document().ordered_ids();
    assert_eq!(ordered.len(), 3);
    let inserted = ordered[1];
    assert_eq!(state.document().block(inserted).unwrap().content, reference);
    assert_eq!(state.document().block(inserted).unwrap().left_id, Some(a));
    assert_eq!(state.document().block(b).unwrap().left_id, Some(inserted));
    assert_eq!(state.caret().unwrap().block_id, inserted);
    assert_eq!(state.caret().unwrap().byte_offset, reference.len());
    assert_eq!(state.history_depth(), 1);
    assert!(state.is_dirty());

    // The reference survives the save transform byte-exactly.
    let body = state.document().body();
    let canonical = crate::document::canonicalize_body(&body);
    assert_eq!(canonical, body);
    assert!(canonical.contains("![[assets/pic.png]]"));

    assert!(state.undo().unwrap());
    assert_eq!(state.document().markdown(), before);
}

#[test]
fn insert_asset_reference_replaces_an_empty_caret_block() {
    let id = Uuid::new_v4();
    let mut state = EditorState::new(doc_with(vec![block(id, "")]));
    state.set_caret(id, 0).unwrap();
    let reference = crate::assets::asset_reference("assets/doc.pdf");

    state.insert_asset_reference("assets/doc.pdf").unwrap();
    assert_eq!(state.document().blocks().len(), 1);
    assert_eq!(state.document().block(id).unwrap().content, reference);
    assert_eq!(state.caret().unwrap().block_id, id);
    assert_eq!(state.caret().unwrap().byte_offset, reference.len());
    assert_eq!(state.history_depth(), 1);
    assert!(state.is_dirty());

    assert!(state.undo().unwrap());
    assert_eq!(state.document().block(id).unwrap().content, "");
}

#[test]
fn apply_ai_content_covers_every_scope() {
    // ReplacePage
    let id = Uuid::new_v4();
    let mut state = EditorState::new(doc_with(vec![block(id, "old")]));
    let call = AiCall {
        kind: AiCallKind::Transform(AiTransformAction::Structure),
        scope: AiCallScope::ReplacePage,
        text: "prompt".into(),
        page_path: Some("pages/x.md".into()),
    };
    state.apply_ai_content(&call, AI_REPLACEMENT).unwrap();
    assert_eq!(state.document().markdown(), AI_REPLACEMENT);

    // InsertAtCaret
    let id = Uuid::new_v4();
    let mut state = EditorState::new(doc_with(vec![block(id, "ab")]));
    state.set_caret(id, 0).unwrap();
    let call = AiCall {
        kind: AiCallKind::Transform(AiTransformAction::Rewrite),
        scope: AiCallScope::InsertAtCaret,
        text: "prompt".into(),
        page_path: None,
    };
    state.apply_ai_content(&call, "hello").unwrap();
    assert_eq!(state.document().block(id).unwrap().content, "helloab");

    // InsertBlockAfterCaret
    let id = Uuid::new_v4();
    let mut state = EditorState::new(doc_with(vec![block(id, "source")]));
    state.set_caret(id, 0).unwrap();
    let call = AiCall {
        kind: AiCallKind::Mermaid,
        scope: AiCallScope::InsertBlockAfterCaret,
        text: "prompt".into(),
        page_path: None,
    };
    state.apply_ai_content(&call, "graph TD\n  A-->B").unwrap();
    let mermaid = state
        .document()
        .blocks()
        .into_iter()
        .find(|block| block.content.starts_with("```mermaid"))
        .expect("mermaid block");
    assert_eq!(mermaid.content, "```mermaid\ngraph TD\n  A-->B\n```");
}

#[test]
fn set_block_reminder_writes_and_clears_the_due_property() {
    let id = Uuid::new_v4();
    let mut state = EditorState::new(doc_with(vec![block(id, "task")]));
    let before = state.document().revision();

    state
        .set_block_reminder(id, Some("2026-11-09T12:00:00Z"))
        .unwrap();
    let block = state.document().block(id).unwrap();
    assert_eq!(
        block.properties.get("due").map(String::as_str),
        Some("2026-11-09T12:00:00Z")
    );
    assert!(state.document().revision() > before);
    assert!(state.is_dirty());

    state.set_block_reminder(id, None).unwrap();
    assert!(!state
        .document()
        .block(id)
        .unwrap()
        .properties
        .contains_key("due"));

    assert!(state
        .set_block_reminder(Uuid::new_v4(), Some("2026-11-09T12:00:00Z"))
        .is_err());
}
