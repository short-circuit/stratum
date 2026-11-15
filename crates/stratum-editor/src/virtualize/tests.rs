use uuid::Uuid;

use super::*;
use crate::inline::SpanKind;
use crate::render::{BlockKind, DecoratedRow, DecoratedSpan};

fn row(kind: BlockKind) -> DecoratedRow {
    DecoratedRow {
        block_id: Uuid::new_v4(),
        depth: 0,
        kind,
        marker: None,
        priority: None,
        spans: vec![DecoratedSpan {
            text: "x".into(),
            kind: SpanKind::Text,
            syntax: false,
            hidden: false,
        }],
        badge: None,
        reminder: None,
        collapsed: false,
        math: None,
        mermaid: None,
        asset: None,
        thumbnail: None,
    }
}

fn rows(count: usize) -> Vec<DecoratedRow> {
    (0..count).map(|_| row(BlockKind::Paragraph)).collect()
}

fn row_with_text(kind: BlockKind, text: &str) -> DecoratedRow {
    let mut decorated = row(kind);
    decorated.spans[0].text = text.to_string();
    decorated
}

#[test]
fn visible_window_is_bounded_by_viewport_and_overscan() {
    let layout = VirtualLayout::from_rows(&rows(10_000));
    let window = layout.visible_window(0.0, 600.0, 2);
    let max_rows = (600.0 / MIN_ROW_HEIGHT).ceil() as usize + 2 * 2 + 2;
    assert!(
        window.len() <= max_rows,
        "window {} > {max_rows}",
        window.len()
    );
    assert!(window.end <= 10_000);
}

#[test]
fn empty_layout_has_empty_window() {
    let layout = VirtualLayout::from_rows(&[]);
    assert_eq!(layout.visible_window(0.0, 600.0, 2), 0..0);
    assert_eq!(layout.total_height(), 0.0);
    assert_eq!(layout.block_at_offset(0.0), None);
    assert_eq!(layout.scroll_top_for_block(0), None);
}

#[test]
fn offsets_are_monotonic_and_match_heights() {
    let layout = VirtualLayout::from_rows(&rows(100));
    let offsets = layout.offsets();
    let heights = layout.heights();
    assert_eq!(offsets.len(), heights.len());
    for i in 1..offsets.len() {
        assert!(offsets[i] > offsets[i - 1]);
        assert!((offsets[i] - (offsets[i - 1] + heights[i - 1])).abs() < 0.001);
    }
    assert!((layout.total_height() - (offsets[99] + heights[99])).abs() < 0.001);
}

#[test]
fn block_at_offset_is_the_inverse_of_scroll_top() {
    let layout = VirtualLayout::from_rows(&rows(50));
    for index in 0..50 {
        let top = layout.scroll_top_for_block(index).unwrap();
        assert_eq!(layout.block_at_offset(top), Some(index));
    }
    assert_eq!(layout.scroll_top_for_block(50), None);
}

#[test]
fn block_at_offset_clamps_past_the_end() {
    let layout = VirtualLayout::from_rows(&rows(5));
    let total = layout.total_height();
    assert_eq!(layout.block_at_offset(total + 1000.0), Some(4));
    assert_eq!(layout.block_at_offset(-1.0), None);
}

#[test]
fn anchor_offset_sits_inside_the_row() {
    let layout = VirtualLayout::from_rows(&rows(3));
    let anchor = layout.anchor_offset(1).unwrap();
    let top = layout.scroll_top_for_block(1).unwrap();
    let height = layout.heights()[1];
    assert!(anchor > top);
    assert!(anchor < top + height);
    assert_eq!(layout.anchor_offset(3), None);
}

#[test]
fn multi_line_code_reserves_one_line_per_break() {
    let code = row_with_text(
        BlockKind::Code {
            lang: "rust".into(),
        },
        "```rust\nfn main() {\n    println!(\"x\");\n}\n```",
    );
    let layout = VirtualLayout::from_rows(&[code]);
    let height = layout.heights()[0];
    // Five visual lines: the 22 px code base plus four 18 px breaks.
    assert_eq!(height, 22.0 + 4.0 * 18.0);
}

#[test]
fn line_breaks_do_not_grow_non_text_rows() {
    let heading = row_with_text(BlockKind::Heading(1), "a\nb\nc");
    let layout = VirtualLayout::from_rows(&[heading]);
    assert_eq!(
        layout.heights()[0],
        estimate_height(&BlockKind::Heading(1), 5)
    );
}

#[test]
fn estimate_height_matches_the_documented_formula() {
    assert_eq!(estimate_height(&BlockKind::Paragraph, 0), 24.0);
    assert_eq!(estimate_height(&BlockKind::Paragraph, 160), 42.0);
    assert_eq!(estimate_height(&BlockKind::Heading(1), 0), 32.0);
    assert_eq!(
        estimate_height(
            &BlockKind::Code {
                lang: "rust".into()
            },
            0
        ),
        22.0
    );
    assert_eq!(estimate_height(&BlockKind::Mermaid, 0), 180.0);
    assert_eq!(estimate_height(&BlockKind::Math { display: true }, 0), 48.0);
    assert_eq!(
        estimate_height(&BlockKind::Math { display: false }, 0),
        24.0
    );
    assert_eq!(estimate_height(&BlockKind::Divider, 0), 12.0);
    assert_eq!(estimate_height(&BlockKind::Table, 0), 24.0);
    assert_eq!(
        estimate_height(&BlockKind::Paragraph, 0),
        estimate_height(&BlockKind::Paragraph, 0)
    );
}

#[test]
fn set_height_clamps_to_the_minimum() {
    let mut layout = VirtualLayout::from_rows(&rows(2));
    assert!(layout.set_height(0, 1.0));
    assert_eq!(layout.heights()[0], MIN_ROW_HEIGHT);
    assert!(layout.set_height(1, 100.0));
    assert_eq!(layout.heights()[1], 100.0);
    assert!(!layout.set_height(5, 10.0));
}

#[test]
fn asset_rows_reserve_their_decoded_heights() {
    use crate::assets::{ASSET_FILE_ROW_HEIGHT, ASSET_IMAGE_ROW_HEIGHT};
    use crate::document::EditorDocument;
    use crate::render::{render_page, RenderMode};
    use pkm_block::Block;

    let blocks = vec![
        Block::new(Uuid::new_v4(), "![[assets/pic.png]]".to_string()),
        Block::new(Uuid::new_v4(), "![[assets/doc.pdf]]".to_string()),
    ];
    let doc = EditorDocument::from_blocks("pages/x.md", blocks).unwrap();
    let rows = render_page(&doc, None, RenderMode::StageA, chrono::Utc::now());
    assert_eq!(rows.len(), 2);
    assert!(rows[0].asset.is_some());
    assert!(rows[1].asset.is_some());

    let layout = VirtualLayout::from_rows(&rows);
    let heights = layout.heights();
    assert!(
        heights[0] >= ASSET_IMAGE_ROW_HEIGHT,
        "image row {} < {ASSET_IMAGE_ROW_HEIGHT}",
        heights[0]
    );
    assert!(
        heights[1] >= ASSET_FILE_ROW_HEIGHT,
        "file row {} < {ASSET_FILE_ROW_HEIGHT}",
        heights[1]
    );
    let offsets = layout.offsets();
    assert!(offsets[1] > offsets[0]);
    assert!((offsets[1] - heights[0]).abs() < 0.001);

    let window = layout.visible_window(0.0, 600.0, 2);
    let max_rows = (600.0 / MIN_ROW_HEIGHT).ceil() as usize + 2 * 2 + 2;
    assert!(window.len() <= max_rows);
}

#[test]
fn visible_window_tracks_scroll_position() {
    let layout = VirtualLayout::from_rows(&rows(1000));
    let top = layout.scroll_top_for_block(500).unwrap();
    let window = layout.visible_window(top, 300.0, 1);
    assert!(window.start <= 500);
    assert!(window.end > 500);
    assert!(window.len() <= (300.0 / MIN_ROW_HEIGHT).ceil() as usize + 4);
}
