//! Scripted editing-session integration tests.
//!
//! Each test drives [`EditorState`] the way the phase-B view will and then
//! asserts the exact markdown the document would save. Random ids for inserted
//! blocks are captured from the document instead of being guessed.

use pkm_block::{Priority, TaskMarker};
use stratum_editor::{paste_markdown, EditorDocument, EditorState};

const A: &str = "11111111-1111-4111-8111-111111111111";

fn open(raw: &str) -> EditorState {
    let doc = EditorDocument::open("pages/session.md", raw).unwrap();
    EditorState::new(doc)
}

fn id_of(state: &EditorState, content: &str) -> String {
    state
        .document()
        .blocks()
        .iter()
        .find(|block| block.content == content)
        .map(|block| block.id.to_string())
        .expect("block with content")
}

#[test]
fn typing_undo_and_redo_ends_at_the_expected_markdown() {
    let raw = format!("- hello\n  .id: {A}\n");
    let mut state = open(&raw);
    let id = state.document().ordered_ids()[0];
    state.set_caret(id, 5).unwrap();
    state.insert_text("!").unwrap();
    state.insert_text("?").unwrap();
    assert!(state.undo().unwrap());
    assert!(state.redo().unwrap());
    assert_eq!(
        state.document().markdown(),
        format!("- hello!?\n  .id: {A}\n")
    );
}

#[test]
fn split_marker_priority_and_indent_session_ends_at_the_expected_markdown() {
    let raw = format!("- hello\n  .id: {A}\n");
    let mut state = open(&raw);
    let first = state.document().ordered_ids()[0];
    state.set_caret(first, 0).unwrap();
    state.insert_text("Hi ").unwrap();
    state.set_caret(first, 3).unwrap();
    state.insert_newline().unwrap();
    state.insert_text("there ").unwrap();
    let second = state.document().ordered_ids()[1];
    state.set_marker(Some(TaskMarker::Todo)).unwrap();
    state.set_priority(Some(Priority::A)).unwrap();
    state.indent().unwrap();
    assert!(state.undo().unwrap());
    assert!(state.undo().unwrap());
    assert!(state.undo().unwrap());
    assert!(state.redo().unwrap());
    assert!(state.redo().unwrap());
    assert!(state.redo().unwrap());
    let new = second.to_string();
    assert_eq!(
        state.document().markdown(),
        format!(
            "- Hi \n  .id: {A}\n  - there hello\n    .marker: TODO\n    .priority: A\n    .id: {new}\n"
        )
    );
}

#[test]
fn paste_markdown_session_ends_at_the_expected_markdown() {
    let raw = format!("- start\n  .id: {A}\n");
    let mut state = open(&raw);
    let first = state.document().ordered_ids()[0];
    state.set_caret(first, 5).unwrap();
    let payload = stratum_editor::ClipboardPayload {
        plain_text: "one\ntwo".into(),
        blocks: paste_markdown("one\ntwo"),
    };
    state.paste_payload(&payload).unwrap();
    let one = id_of(&state, "one");
    let two = id_of(&state, "two");
    assert_eq!(
        state.document().markdown(),
        format!("- start\n  .id: {A}\n- one\n  .id: {one}\n- two\n  .id: {two}\n")
    );
}

#[test]
fn ime_session_ends_at_the_expected_markdown() {
    let raw = format!("- ab\n  .id: {A}\n");
    let mut state = open(&raw);
    let id = state.document().ordered_ids()[0];
    state.set_caret(id, 0).unwrap();
    state.ime_replace(0..0, "日本").unwrap();
    state.ime_replace(0..2, "日本語").unwrap();
    state.ime_commit().unwrap();
    assert_eq!(
        state.document().markdown(),
        format!("- 日本語ab\n  .id: {A}\n")
    );
    assert!(state.undo().unwrap());
    assert_eq!(state.document().markdown(), format!("- ab\n  .id: {A}\n"));
}

#[test]
fn cut_then_paste_restores_the_original_document() {
    let raw = format!("- hello world\n  .id: {A}\n");
    let mut state = open(&raw);
    let id = state.document().ordered_ids()[0];
    state
        .set_selection(
            stratum_editor::Caret {
                block_id: id,
                byte_offset: 0,
            },
            stratum_editor::Caret {
                block_id: id,
                byte_offset: 5,
            },
        )
        .unwrap();
    let payload = state.cut_selection().unwrap();
    state.set_caret(id, 0).unwrap();
    state.paste_payload(&payload).unwrap();
    assert_eq!(
        state.document().markdown(),
        format!("- hello world\n  .id: {A}\n")
    );
}
