mod common;

use common::create_test_vault;
use std::fs;

#[test]
fn test_parse_simple_page() {
    let tv = create_test_vault();
    let md = r#"---
title: Test Page
tags: [test, demo]
---

# Heading 1
This is a paragraph with **bold** and *italic*.

[[WikiLink]] at the end.

- A task
  .marker: TODO
- Working
  .marker: DOING
- Finished
  .marker: DONE
"#;
    tv.create_md_file("pages/test.md", md);

    let content = fs::read_to_string(tv.vault_path.join("pages/test.md")).unwrap();
    let (frontmatter, _body, blocks) = pkm_markdown::block_parser::parse_document(&content);

    assert_eq!(frontmatter.title.as_deref(), Some("Test Page"));
    assert!(!blocks.is_empty(), "should parse blocks");
    assert!(blocks.iter().any(|b| b.meta.heading_level == Some(1)));
    assert!(blocks
        .iter()
        .any(|b| b.marker == Some(pkm_block::TaskMarker::Todo)));
    assert!(blocks
        .iter()
        .any(|b| b.marker == Some(pkm_block::TaskMarker::Doing)));
    assert!(blocks
        .iter()
        .any(|b| b.marker == Some(pkm_block::TaskMarker::Done)));
}

#[test]
fn test_empty_content() {
    let tv = create_test_vault();
    tv.create_md_file("pages/empty.md", "---\ntitle: Empty\n---");
    let content = fs::read_to_string(tv.vault_path.join("pages/empty.md")).unwrap();
    let (frontmatter, _body, blocks) = pkm_markdown::block_parser::parse_document(&content);
    assert_eq!(frontmatter.title.as_deref(), Some("Empty"));
    assert!(blocks.is_empty());
}

#[test]
fn test_no_frontmatter() {
    let tv = create_test_vault();
    tv.create_md_file("pages/plain.md", "# Just a heading\n\nSome text.");
    let content = fs::read_to_string(tv.vault_path.join("pages/plain.md")).unwrap();
    let (frontmatter, _body, blocks) = pkm_markdown::block_parser::parse_document(&content);
    assert!(frontmatter.title.is_none());
    assert!(!blocks.is_empty());
}

#[test]
fn test_multiple_headings() {
    let tv = create_test_vault();
    tv.create_md_file("pages/headings.md", "# H1\n\n## H2\n\n### H3");
    let content = fs::read_to_string(tv.vault_path.join("pages/headings.md")).unwrap();
    let (_, _body, blocks) = pkm_markdown::block_parser::parse_document(&content);
    let heading_levels: Vec<u8> = blocks.iter().filter_map(|b| b.meta.heading_level).collect();
    assert_eq!(heading_levels, vec![1, 2, 3]);
}

#[test]
fn test_wiki_links_extracted() {
    let tv = create_test_vault();
    tv.create_md_file(
        "pages/links.md",
        "See [[Target Page]] and [[Other Page|display text]].",
    );
    let content = fs::read_to_string(tv.vault_path.join("pages/links.md")).unwrap();
    let links = pkm_markdown::linker::extract_links(&content);
    assert_eq!(links.len(), 2);
    assert_eq!(links[0].target, "Target Page");
    assert_eq!(links[1].target, "Other Page");
    assert_eq!(links[1].display_text.as_deref(), Some("display text"));
}

#[test]
fn test_tags_extracted() {
    let tv = create_test_vault();
    tv.create_md_file(
        "pages/tags.md",
        "---\ntags: [frontmatter-tag]\n---\nThis is about #machine-learning and #rust.",
    );
    let content = fs::read_to_string(tv.vault_path.join("pages/tags.md")).unwrap();
    let (fm, _body, _blocks) = pkm_markdown::block_parser::parse_document(&content);
    let tags = pkm_markdown::tagger::extract_tags(&content, &fm);
    assert!(tags.iter().any(|t| t.name == "frontmatter-tag"));
    assert!(tags.iter().any(|t| t.name == "machine-learning"));
    assert!(tags.iter().any(|t| t.name == "rust"));
}

#[test]
fn test_formatting_preserved() {
    let tv = create_test_vault();
    let md = "**bold** *italic* ~~strikethrough~~ `code`";
    tv.create_md_file("pages/format.md", md);
    let content = fs::read_to_string(tv.vault_path.join("pages/format.md")).unwrap();
    let (_, _body, blocks) = pkm_markdown::block_parser::parse_document(&content);
    assert!(!blocks.is_empty());
}
