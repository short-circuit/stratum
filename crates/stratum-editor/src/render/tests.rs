use chrono::{Duration, TimeZone, Utc};
use pkm_block::{Block, TaskMarker};
use uuid::Uuid;

use super::*;
use crate::document::EditorDocument;

fn doc_with(blocks: Vec<Block>) -> EditorDocument {
    EditorDocument::from_blocks("pages/x.md", blocks).unwrap()
}

fn block(id: Uuid, content: &str) -> Block {
    Block::new(id, content.to_string())
}

fn now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 9, 12, 0, 0).unwrap()
}

#[test]
fn stage_a_keeps_syntax_delimiters_visible() {
    let id = Uuid::new_v4();
    let doc = doc_with(vec![block(id, "**bold**")]);
    let rows = render_page(&doc, None, RenderMode::StageA, now());
    assert_eq!(rows.len(), 1);
    let syntax: Vec<&DecoratedSpan> = rows[0].spans.iter().filter(|s| s.syntax).collect();
    assert_eq!(syntax.len(), 2);
    assert_eq!(syntax[0].text, "**");
    assert!(syntax.iter().all(|s| !s.hidden));
    let content: String = rows[0]
        .spans
        .iter()
        .filter(|s| !s.syntax)
        .map(|s| s.text.clone())
        .collect();
    assert_eq!(content, "bold");
}

#[test]
fn focused_line_hides_only_the_focused_blocks_delimiters() {
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let mut second = block(b, "**other**");
    second.left_id = Some(a);
    let doc = doc_with(vec![block(a, "**focused**"), second]);
    let rows = render_page(&doc, Some(a), RenderMode::FocusedLine, now());
    assert!(rows[0].spans.iter().filter(|s| s.syntax).all(|s| s.hidden));
    assert!(rows[1].spans.iter().filter(|s| s.syntax).all(|s| !s.hidden));
}

#[test]
fn marker_keyword_is_stripped_and_represented_by_the_badge() {
    let id = Uuid::new_v4();
    let mut block = block(id, "TODO buy milk");
    block.marker = Some(TaskMarker::Todo);
    let doc = doc_with(vec![block]);
    let rows = render_page(&doc, None, RenderMode::StageA, now());
    let text: String = rows[0].spans.iter().map(|s| s.text.clone()).collect();
    assert_eq!(text, "buy milk");
    assert_eq!(
        rows[0].badge.as_ref().unwrap().marker,
        Some(TaskMarker::Todo)
    );
}

#[test]
fn reminder_chip_is_attached_to_the_block() {
    let id = Uuid::new_v4();
    let due = (now() + Duration::hours(2)).to_rfc3339();
    let block = block(id, "reminder").with_property("due", due);
    let doc = doc_with(vec![block]);
    let rows = render_page(&doc, None, RenderMode::StageA, now());
    let chip = rows[0].reminder.as_ref().expect("reminder chip");
    assert_eq!(chip.label_key, "reminders.humanize.due_in_hours");
    assert!(!chip.overdue);
}

#[test]
fn classifies_block_kinds() {
    let cases: Vec<(Block, BlockKind)> = vec![
        {
            let mut b = block(Uuid::new_v4(), "Title");
            b.meta.heading_level = Some(2);
            (b, BlockKind::Heading(2))
        },
        (
            block(Uuid::new_v4(), "```rust\nfn main() {}\n```"),
            BlockKind::Code {
                lang: "rust".into(),
            },
        ),
        (
            block(Uuid::new_v4(), "```mermaid\ngraph TD\n```"),
            BlockKind::Mermaid,
        ),
        (
            block(Uuid::new_v4(), "$$x$$"),
            BlockKind::Math { display: true },
        ),
        (
            block(Uuid::new_v4(), "$x$"),
            BlockKind::Math { display: false },
        ),
        (block(Uuid::new_v4(), "> quote"), BlockKind::Quote),
        (block(Uuid::new_v4(), "- item"), BlockKind::ListItem),
        (block(Uuid::new_v4(), "---"), BlockKind::Divider),
        (block(Uuid::new_v4(), "plain"), BlockKind::Paragraph),
    ];
    for (block, expected) in cases {
        let doc = doc_with(vec![block]);
        let rows = render_page(&doc, None, RenderMode::StageA, now());
        assert_eq!(rows[0].kind, expected);
    }
}

#[test]
fn code_bodies_are_opaque_text_spans() {
    let id = Uuid::new_v4();
    let doc = doc_with(vec![block(id, "```rust\nlet x = 1;\n```")]);
    let rows = render_page(&doc, Some(id), RenderMode::FocusedLine, now());
    assert_eq!(rows[0].spans.len(), 1);
    assert_eq!(rows[0].spans[0].kind, SpanKind::Text);
    assert_eq!(rows[0].spans[0].text, "```rust\nlet x = 1;\n```");
}

#[test]
fn asset_embeds_render_as_asset_spans() {
    let id = Uuid::new_v4();
    let doc = doc_with(vec![block(id, "![[assets/foo.png]]")]);
    let rows = render_page(&doc, None, RenderMode::StageA, now());
    let content: Vec<&DecoratedSpan> = rows[0].spans.iter().filter(|s| !s.syntax).collect();
    assert_eq!(content.len(), 1);
    assert_eq!(
        content[0].kind,
        SpanKind::Asset {
            path: "assets/foo.png".into()
        }
    );
    assert_eq!(content[0].text, "assets/foo.png");
    assert!(!rows[0]
        .spans
        .iter()
        .any(|s| matches!(s.kind, SpanKind::WikiLink { embed: true, .. })));
}

#[test]
fn row_order_matches_depth_first_order() {
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let c = Uuid::new_v4();
    let mut child = block(b, "child");
    child.parent_id = Some(a);
    let mut third = block(c, "third");
    third.left_id = Some(a);
    let doc = doc_with(vec![block(a, "root"), child, third]);
    let rows = render_page(&doc, None, RenderMode::StageA, now());
    let ids: Vec<Uuid> = rows.iter().map(|r| r.block_id).collect();
    let expected: Vec<Uuid> = doc.ordered_ids();
    assert_eq!(ids, expected);
    assert_eq!(rows[0].depth, 0);
    assert_eq!(rows[1].depth, 1);
    assert_eq!(rows[2].depth, 0);
}

#[test]
fn collapsed_state_is_exposed() {
    let id = Uuid::new_v4();
    let mut block = block(id, "parent");
    block.meta.collapsed = true;
    let doc = doc_with(vec![block]);
    let rows = render_page(&doc, None, RenderMode::StageA, now());
    assert!(rows[0].collapsed);
}
