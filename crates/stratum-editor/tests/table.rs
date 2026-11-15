//! Integration tests for GFM pipe-table blocks and the `table` slash action.
//!
//! The on-disk representation must stay plain Markdown: a table block survives
//! open → markdown → open byte-for-byte, and the renderer decorates its pipes as
//! structural (never hidden) syntax spans.

use chrono::Utc;
use pkm_block::Block;
use stratum_editor::{menus, render_page, BlockKind, EditorDocument, RenderMode};
use uuid::Uuid;

const TABLE_FIXTURE: &str = include_str!("fixtures/canonical_table.md");
const TABLE_CONTENT: &str = "| Task | Owner |\n| --- | --- |\n| ship | anon |";

#[test]
fn table_block_survives_open_markdown_open() {
    let doc = EditorDocument::open("pages/table.md", TABLE_FIXTURE).expect("fixture opens");
    assert_eq!(doc.body(), TABLE_FIXTURE, "canonical fixture changed");

    let first_markdown = doc.markdown();
    let reopened = EditorDocument::open("pages/table.md", &first_markdown).expect("re-open");
    assert_eq!(reopened.markdown(), first_markdown);

    let content = doc.blocks()[0].content.clone();
    assert_eq!(content, TABLE_CONTENT);

    let table = pkm_markdown::parse_table(&content).expect("block content is a table");
    assert_eq!(table.column_count(), 2);
    assert_eq!(table.row_count(), 1);
    assert_eq!(pkm_markdown::serialize_table(&table), content);
}

#[test]
fn table_action_inserts_a_classifiable_skeleton() {
    let skeleton = menus::table_skeleton();
    let id = Uuid::new_v4();
    let doc = EditorDocument::from_blocks("pages/x.md", vec![Block::new(id, skeleton.clone())])
        .expect("document builds");
    let rows = render_page(&doc, None, RenderMode::StageA, Utc::now());
    assert_eq!(rows[0].kind, BlockKind::Table);

    let body = doc.body();
    let reopened = EditorDocument::open("pages/x.md", &body).expect("body re-opens");
    assert_eq!(reopened.body(), body, "skeleton block must round-trip");
    assert_eq!(reopened.blocks()[0].content, skeleton);

    let table = menus::slash_items(false, false)
        .into_iter()
        .find(|item| item.id == "table")
        .expect("table action is offered");
    assert_eq!(table.title_key, "editor.table");
}

#[test]
fn table_spans_preserve_source_and_stay_visible_when_focused() {
    let id = Uuid::new_v4();
    let doc = EditorDocument::from_blocks(
        "pages/x.md",
        vec![Block::new(id, TABLE_CONTENT.to_string())],
    )
    .expect("document builds");
    let rows = render_page(&doc, Some(id), RenderMode::FocusedLine, Utc::now());
    let row = &rows[0];
    assert_eq!(row.kind, BlockKind::Table);

    let concatenated: String = row.spans.iter().map(|span| span.text.clone()).collect();
    assert_eq!(concatenated, TABLE_CONTENT);
    assert!(
        row.spans.iter().any(|span| span.text == "|" && span.syntax),
        "pipe spans must be structural syntax"
    );
    assert!(
        row.spans
            .iter()
            .filter(|span| span.text.contains("---"))
            .all(|span| span.syntax),
        "delimiter cells must be muted"
    );
    assert!(
        row.spans.iter().all(|span| !span.hidden),
        "table pipes stay visible on the focused line"
    );
}
