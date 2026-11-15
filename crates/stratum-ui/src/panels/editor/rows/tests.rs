//! Unit tests for the pure row geometry and offset helpers.

use super::*;
use ::core::prelude::v1::test;
use chrono::Utc;
use stratum_editor::{dto_to_blocks, render_page, EditorDocument, RenderMode};

fn row_for(content: &str, focus: bool) -> DecoratedRow {
    let blocks = dto_to_blocks(vec![stratum_editor::BlockDto {
        id: uuid::Uuid::new_v4().to_string(),
        content: content.to_string(),
        parent_id: None,
        left_id: None,
        properties: Vec::new(),
        marker: None,
        priority: None,
        collapsed: false,
        heading_level: None,
    }]);
    let doc = EditorDocument::from_blocks("pages/Test.md", blocks).expect("document");
    let id = doc.ordered_ids()[0];
    let mode = if focus {
        RenderMode::FocusedLine
    } else {
        RenderMode::StageA
    };
    render_page(&doc, focus.then_some(id), mode, Utc::now())
        .into_iter()
        .next()
        .expect("row")
}

#[test]
fn visible_offsets_round_trip_for_stage_a() {
    let row = row_for("hello **bold** world", false);
    let text = visible_text(&row);
    assert_eq!(text, "hello **bold** world");
    for offset in 0..=text.len() {
        let source = source_offset_for_visible(&row, offset);
        assert_eq!(visible_offset_for_source(&row, source), offset);
    }
}

#[test]
fn hidden_delimiters_collapse_visible_offsets() {
    let row = row_for("hello **bold** world", true);
    let text = visible_text(&row);
    assert_eq!(text, "hello bold world");
    // The source offset after the opening delimiter maps to the same
    // visible position as the offset before it.
    let before = visible_offset_for_source(&row, 6);
    let after = visible_offset_for_source(&row, 8);
    assert_eq!(before, after);
}

#[test]
fn marker_prefix_is_measured_like_the_renderer() {
    assert_eq!(marker_prefix_len("  TODO write", Some("TODO")), 7);
    assert_eq!(marker_prefix_len("TODOs write", Some("TODO")), 0);
    assert_eq!(marker_prefix_len("write", None), 0);
}
