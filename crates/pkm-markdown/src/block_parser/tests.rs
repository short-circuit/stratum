//! Tests for the block parser module.

use super::raw::{block_content, count_indent, is_block_line, parse_property};
use super::*;
use pkm_block::Block;
use std::str::FromStr;
use uuid::Uuid;

#[test]
fn test_count_indent() {
    assert_eq!(count_indent(""), 0);
    assert_eq!(count_indent("hello"), 0);
    assert_eq!(count_indent("  hello"), 2);
    assert_eq!(count_indent("    hello"), 4);
}

#[test]
fn test_is_block_line() {
    assert!(is_block_line("- hello"));
    assert!(is_block_line("  - hello"));
    assert!(!is_block_line("hello"));
    assert!(!is_block_line("-- hello"));
}

#[test]
fn test_block_content() {
    assert_eq!(block_content("- hello"), "hello");
    assert_eq!(block_content("  - hello world"), "hello world");
}

#[test]
fn test_parse_property() {
    assert_eq!(
        parse_property(".id: abc-123"),
        Some(("id".into(), "abc-123".into()))
    );
    assert_eq!(
        parse_property("  .priority: A"),
        Some(("priority".into(), "A".into()))
    );
    assert_eq!(
        parse_property(".deadline: 2026-07-01"),
        Some(("deadline".into(), "2026-07-01".into()))
    );
    assert_eq!(parse_property("not a property"), None);
    assert_eq!(parse_property("- block line"), None);
    assert_eq!(parse_property(".nocolon"), None);
}

#[test]
fn test_parse_single_block() {
    let body = "- Hello world\n";
    let raw = parse_raw_blocks(body);
    assert_eq!(raw.len(), 1);
    assert_eq!(raw[0].content_lines, vec!["Hello world"]);
}

#[test]
fn test_parse_block_with_properties() {
    let body = "- A task\n  .marker: TODO\n  .priority: A\n  .deadline: tomorrow\n";
    let raw = parse_raw_blocks(body);
    assert_eq!(raw.len(), 1);
    assert_eq!(raw[0].marker, Some("TODO".into()));
    assert_eq!(raw[0].priority, Some("A".into()));
    assert_eq!(raw[0].properties.get("deadline").unwrap(), "tomorrow");
}

#[test]
fn test_parse_block_with_id() {
    let id = "65f8a1e2-3a4b-4c5d-6e7f-8a9b0c1d2e3f";
    let body = format!("- Test block\n  .id: {}\n", id);
    let raw = parse_raw_blocks(&body);
    assert_eq!(raw.len(), 1);
    assert_eq!(raw[0].block_id, Uuid::from_str(id).ok());
}

#[test]
fn test_parse_nested_blocks() {
    let body = "- Parent\n  - Child\n    - Grandchild\n  - Sibling\n";
    let raw = parse_raw_blocks(body);
    assert_eq!(raw.len(), 4);
    assert_eq!(raw[0].indent, 0);
    assert_eq!(raw[1].indent, 2);
    assert_eq!(raw[2].indent, 4);
    assert_eq!(raw[3].indent, 2);
}

#[test]
fn test_parse_multiple_roots() {
    let body = "- First\n- Second\n- Third\n";
    let raw = parse_raw_blocks(body);
    assert_eq!(raw.len(), 3);
    assert_eq!(raw[0].indent, 0);
    assert_eq!(raw[1].indent, 0);
    assert_eq!(raw[2].indent, 0);
}

#[test]
fn test_parse_block_with_continuation() {
    let body = "- First line\n  continuation\n- Next block\n";
    let raw = parse_raw_blocks(body);
    assert_eq!(raw.len(), 2);
    assert_eq!(raw[0].content_lines, vec!["First line", "continuation"]);
    assert_eq!(raw[1].content_lines, vec!["Next block"]);
}

#[test]
fn test_parse_empty_body() {
    let raw = parse_raw_blocks("");
    assert!(raw.is_empty());
}

#[test]
fn test_build_block_tree_simple() {
    let raw = parse_raw_blocks("- First\n- Second\n");
    let blocks = build_block_tree(&raw);
    assert_eq!(blocks.len(), 2);
    assert!(blocks[0].parent_id.is_none());
    assert!(blocks[1].parent_id.is_none());
    assert_eq!(blocks[1].left_id, Some(blocks[0].id));
}

#[test]
fn test_build_block_tree_nested() {
    let raw = parse_raw_blocks("- Parent\n  - Child\n");
    let blocks = build_block_tree(&raw);
    assert_eq!(blocks.len(), 2);
    assert!(blocks[0].parent_id.is_none());
    assert_eq!(blocks[1].parent_id, Some(blocks[0].id));
}

#[test]
fn test_parse_document() {
    let raw = "---\ntitle: Test\ntags: [demo]\n---\n- First block\n  .id: 65f8a1e2-3a4b-4c5d-6e7f-8a9b0c1d2e3f\n- Second block\n";
    let (fm, _body, blocks) = parse_document(raw);
    assert_eq!(fm.title, Some("Test".into()));
    assert_eq!(blocks.len(), 2);
}

#[test]
fn test_convert_body_to_blocks_basic() {
    let body = "## Backend\n\n[[Node.js]] / [[Express]] or [[Fastify]]\n";
    let blocks = convert_body_to_blocks(body);
    assert_eq!(blocks.len(), 2, "should produce 2 blocks");
    assert_eq!(blocks[0].content, "Backend");
    assert_eq!(blocks[0].meta.heading_level, Some(2));
    assert_eq!(
        blocks[1].content,
        "[[Node.js]] / [[Express]] or [[Fastify]]"
    );
    assert!(blocks[1].meta.heading_level.is_none());
}

#[test]
fn test_convert_body_to_blocks_full_sample() {
    let body = "[[React]] with [[TypeScript]] and [[Tailwind CSS]]\n\nResponsive design with CSS Grid and Flexbox\n\n## Backend\n\n[[Node.js]] / [[Express]] or [[Fastify]]\n";
    let blocks = convert_body_to_blocks(body);
    assert_eq!(
        blocks.len(),
        4,
        "should produce 4 blocks: 2 paras, 1 heading, 1 para"
    );
    assert_eq!(
        blocks[0].content,
        "[[React]] with [[TypeScript]] and [[Tailwind CSS]]"
    );
    assert_eq!(
        blocks[1].content,
        "Responsive design with CSS Grid and Flexbox"
    );
    assert_eq!(blocks[2].content, "Backend");
    assert_eq!(blocks[2].meta.heading_level, Some(2));
    assert_eq!(
        blocks[3].content,
        "[[Node.js]] / [[Express]] or [[Fastify]]"
    );
}

#[test]
fn test_parse_document_fallback() {
    let raw = "## Test Heading\n\nSome paragraph with **bold** text.\n";
    let (fm, _body, blocks) = parse_document(raw);
    assert!(fm.title.is_none());
    assert_eq!(
        blocks.len(),
        2,
        "should fall back to converter and produce 2 blocks"
    );
    assert_eq!(blocks[0].content, "Test Heading");
    assert_eq!(blocks[0].meta.heading_level, Some(2));
    assert_eq!(blocks[1].content, "Some paragraph with **bold** text.");
}

#[test]
fn test_convert_heading_after_text_no_blank_line() {
    let body = "Some intro text\n## Algorithms\nMore text here\n";
    let blocks = convert_body_to_blocks(body);
    assert_eq!(
        blocks.len(),
        3,
        "should produce 3 blocks: para, heading, para"
    );
    assert_eq!(blocks[0].content, "Some intro text");
    assert!(blocks[0].meta.heading_level.is_none());
    assert_eq!(blocks[1].content, "Algorithms");
    assert_eq!(blocks[1].meta.heading_level, Some(2));
    assert_eq!(blocks[2].content, "More text here");
    assert!(blocks[2].meta.heading_level.is_none());
}

#[test]
fn test_convert_user_content_exact() {
    let body = "Supervised learning (classification, regression)\n\nUnsupervised learning (clustering, dimensionality reduction)\n\nReinforcement learning (agents, environments)\n\nSemi-supervised and self-supervised learning\\\n## Algorithms\n\nLinear/Logistic regression\n\nDecision trees and Random Forests\n\nSVM (Support Vector Machines)\n\nk-Nearest Neighbors\n\nNeural networks (see [[[Neural-Networks]]]())\\\n## Pipeline\\\n1. Data collection and cleaning\\\n2. Feature engineering and selection\\\n3. Model selection and training\\\n4. Evaluation and validation\\\n5. Deployment and monitoring\\\n## Related\n\n[[[AI]]]() — broader AI context\n\n[[[Python]]]() — scikit-learn, pandas, NumPy\n\n[[[Mathematics]]]() — statistics and linear algebra\n\n[[[Databases]]]() — data storage for ML\\\n#machinelearning #ml #datascience #ai";
    let blocks = convert_body_to_blocks(body);
    // At minimum: paras before ## Algorithms, then Algos heading, then more paragraphs,
    // then ## Pipeline heading, then 5 ordered list items, then ## Related heading, then more paragraphs
    assert!(
        blocks.len() >= 12,
        "should produce at least 12 blocks, got {}",
        blocks.len()
    );

    // Find the ## Algorithms heading
    let algo = blocks
        .iter()
        .find(|b| b.content == "Algorithms" && b.meta.heading_level == Some(2));
    assert!(algo.is_some(), "should find ## Algorithms heading block");

    // Find the ## Pipeline heading
    let pipe = blocks
        .iter()
        .find(|b| b.content == "Pipeline\\" && b.meta.heading_level == Some(2));
    assert!(pipe.is_some(), "should find ## Pipeline heading block");

    // Find the ## Related heading
    let related = blocks
        .iter()
        .find(|b| b.content == "Related" && b.meta.heading_level == Some(2));
    assert!(related.is_some(), "should find ## Related heading block");

    // Should have ordered list items (1. 2. 3. 4. 5.)
    let list_items: Vec<_> = blocks
        .iter()
        .filter(|b| b.content.contains("Data collection"))
        .collect();
    assert!(!list_items.is_empty(), "should find ordered list items");
}

#[test]
fn test_convert_preserves_wiki_links() {
    let body = "[[React]] with [[TypeScript]] and [[Tailwind CSS]]\n\n## Backend\n\n[[Node.js]] / [[Express]] or [[Fastify]]\n";
    let blocks = convert_body_to_blocks(body);
    assert_eq!(
        blocks.len(),
        3,
        "should produce 3 blocks: para, heading, para"
    );
    assert!(blocks[0].content.contains("[[React]]"));
    assert!(blocks[0].content.contains("[[TypeScript]]"));
    assert!(blocks[0].content.contains("[[Tailwind CSS]]"));
    assert_eq!(blocks[1].content, "Backend");
    assert_eq!(blocks[1].meta.heading_level, Some(2));
    assert!(blocks[2].content.contains("[[Node.js]]"));
}

#[test]
fn test_convert_preserves_bold_italic() {
    let body = "**bold** and *italic* and `code` and ~~strike~~\n\nSome plain text";
    let blocks = convert_body_to_blocks(body);
    assert_eq!(blocks.len(), 2);
    assert_eq!(
        blocks[0].content,
        "**bold** and *italic* and `code` and ~~strike~~"
    );
    assert_eq!(blocks[1].content, "Some plain text");
}

#[test]
fn test_convert_detects_atx_headings() {
    let body = "# H1\n\n## H2\n\n### H3\n";
    let blocks = convert_body_to_blocks(body);
    assert_eq!(blocks.len(), 3);
    assert_eq!(blocks[0].content, "H1");
    assert_eq!(blocks[0].meta.heading_level, Some(1));
    assert_eq!(blocks[1].content, "H2");
    assert_eq!(blocks[1].meta.heading_level, Some(2));
    assert_eq!(blocks[2].content, "H3");
    assert_eq!(blocks[2].meta.heading_level, Some(3));
}

#[test]
fn test_convert_preserves_emoji_lines() {
    let body = "🐍 The Ouroboros Symbol\n\nSome text here.\n";
    let blocks = convert_body_to_blocks(body);
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0].content, "🐍 The Ouroboros Symbol");
    assert_eq!(blocks[1].content, "Some text here.");
}

#[test]
fn test_convert_list_items() {
    let body = "- Item 1\n- Item 2\n  - Nested item\n";
    let blocks = convert_body_to_blocks(body);
    // 3 items in one chunk: Item 1, Item 2, Nested item
    assert_eq!(blocks.len(), 3);
    assert_eq!(blocks[0].content, "Item 1");
    assert_eq!(blocks[1].content, "Item 2");
    assert_eq!(blocks[2].content, "Nested item");
    // Nested item has a parent
    assert!(blocks[2].parent_id.is_some());
    assert_eq!(blocks[2].parent_id, Some(blocks[1].id));
}

#[test]
fn test_parse_atx_heading() {
    assert_eq!(parse_atx_heading("# H1"), Some(1));
    assert_eq!(parse_atx_heading("## H2"), Some(2));
    assert_eq!(parse_atx_heading("### H3"), Some(3));
    assert_eq!(parse_atx_heading("#### H4"), Some(4));
    assert_eq!(parse_atx_heading("##### H5"), Some(5));
    assert_eq!(parse_atx_heading("###### H6"), Some(6));
    assert_eq!(parse_atx_heading("not a heading"), None);
    assert_eq!(parse_atx_heading("####### too many"), None);
    assert_eq!(parse_atx_heading("🐍 emoji line"), None);
}

#[test]
fn test_is_thematic_break() {
    assert!(is_thematic_break("---"));
    assert!(is_thematic_break("***"));
    assert!(is_thematic_break("___"));
    assert!(is_thematic_break("  ---  "));
    assert!(!is_thematic_break("--"));
    assert!(!is_thematic_break("not a break"));
}

#[test]
fn test_strip_atx_marker() {
    assert_eq!(strip_atx_marker("## Backend"), "Backend");
    assert_eq!(strip_atx_marker("# Only"), "Only");
    assert_eq!(strip_atx_marker("### Deep"), "Deep");
    assert_eq!(strip_atx_marker("No hash"), "No hash");
    assert_eq!(strip_atx_marker(""), "");
}

#[test]
fn test_parse_document_preserves_block_syntax() {
    let raw = "---\ntitle: Existing\n---\n- First block\n  .id: 65f8a1e2-3a4b-4c5d-6e7f-8a9b0c1d2e3f\n- Second block\n";
    let (fm, _body, blocks) = parse_document(raw);
    assert_eq!(fm.title, Some("Existing".into()));
    assert_eq!(blocks.len(), 2, "should still parse - blocks normally");
    assert_eq!(blocks[0].content, "First block");
}

#[test]
fn test_heading_level_round_trip() {
    let id = uuid::Uuid::new_v4();
    let mut block = pkm_block::Block::new(id, "Hello".into());
    block.meta.heading_level = Some(3);
    let blocks = vec![block];
    let serialized = serialize_blocks(&blocks);
    assert!(serialized.contains(".heading-level: 3"));
    // Re-parse
    let (_fm, _body, reparsed) = parse_document(&serialized);
    assert_eq!(reparsed.len(), 1);
    assert_eq!(reparsed[0].meta.heading_level, Some(3));
}

#[test]
fn test_serialize_round_trip() {
    let id1 = Uuid::new_v4();
    let id2 = Uuid::new_v4();

    let mut block1 = Block::new(id1, "Hello".into());
    block1.left_id = None;
    block1.parent_id = None;

    let mut block2 = Block::new(id2, "World".into());
    block2.left_id = Some(id1);
    block2.parent_id = None;

    let blocks = vec![block1, block2];
    let serialized = serialize_blocks(&blocks);
    assert!(serialized.contains("- Hello"));
    assert!(serialized.contains("- World"));
    assert!(serialized.contains(&format!(".id: {}", id1)));
    assert!(serialized.contains(&format!(".id: {}", id2)));
}

#[test]
fn test_assemble_blocks_markdown_preserves_frontmatter() {
    let existing = "---\ntitle: Keep Me\ntags:\n  - alpha\naliases:\n  - alt\ncreated: 2026-01-01\n---\n\n- Old body\n";
    let body = "- New body\n";
    let out = assemble_blocks_markdown(existing, body, None);
    // Body replaced, frontmatter fields retained.
    assert!(out.contains("- New body"));
    assert!(!out.contains("Old body"));
    assert!(out.contains("alpha"));
    assert!(out.contains("alt"));
    assert!(out.contains("created: 2026-01-01"));

    // Reparse: tags/aliases/created are preserved.
    let (fm, parsed_body, blocks) = parse_document(&out);
    assert_eq!(fm.tags, vec!["alpha"]);
    assert_eq!(fm.aliases, vec!["alt"]);
    assert_eq!(fm.created.as_deref(), Some("2026-01-01"));
    assert_eq!(fm.title.as_deref(), Some("Keep Me"));
    assert_eq!(parsed_body.trim(), "- New body");
    assert_eq!(blocks.len(), 1);
    assert!(blocks[0].content.contains("New body"));
}

#[test]
fn test_assemble_blocks_markdown_updates_title() {
    let existing = "---\ntitle: Old\ntags:\n  - beta\n---\n\n- old\n";
    let out = assemble_blocks_markdown(existing, "- new\n", Some("Renamed"));
    let (fm, _, _) = parse_document(&out);
    assert_eq!(fm.title.as_deref(), Some("Renamed"));
    // Non-title fields survive a title update.
    assert_eq!(fm.tags, vec!["beta"]);
    assert!(out.contains("- new"));
}

#[test]
fn test_assemble_blocks_markdown_no_frontmatter() {
    // No existing frontmatter + no title => body only.
    assert_eq!(assemble_blocks_markdown("hello", "- b\n", None), "- b\n");
    // No existing frontmatter + title => title-only frontmatter.
    assert_eq!(
        assemble_blocks_markdown("hello", "- b\n", Some("T")),
        "---\ntitle: T\n---\n\n- b\n"
    );
}
