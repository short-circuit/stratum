use pkm_block::{Block, Priority, TaskMarker};
use uuid::Uuid;

use super::*;
use crate::document::EditorDocument;
use crate::inline::SpanKind;

fn doc_with(blocks: Vec<Block>) -> EditorDocument {
    EditorDocument::from_blocks("pages/x.md", blocks).unwrap()
}

fn block(id: Uuid, content: &str) -> Block {
    Block::new(id, content.to_string())
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
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let mut second = block(b, "second");
    second.left_id = Some(a);
    let mut state = EditorState::new(doc_with(vec![block(a, "first"), second]));
    let anchor = Caret {
        block_id: a,
        byte_offset: 0,
    };
    let head = Caret {
        block_id: b,
        byte_offset: 6,
    };
    state.set_selection(anchor, head).unwrap();
    let payload = state.cut_selection().unwrap();
    assert_eq!(payload.blocks.len(), 2);
    assert!(state.document().block(a).is_none());
    assert!(state.document().block(b).is_none());
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
