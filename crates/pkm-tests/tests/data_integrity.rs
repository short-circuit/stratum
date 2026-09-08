//! Regression tests for #169 — data-integrity & indexing-state hardening.
//!
//! These exercise the crate-level primitives that `save_blocks` / `sync_page_from_disk`
//! rely on: frontmatter-preserving markdown assembly (ITEM 1) and links-table
//! reconciliation (ITEM 3). `assemble_blocks_markdown` is the exact assembly path the
//! block-save command uses; the store reconciliation mirrors the transaction body the
//! save/sync commands run.

mod common;

use common::create_test_vault;
use pkm_block::{Block, Page};
use pkm_markdown::block_parser::{assemble_blocks_markdown, parse_document, serialize_blocks};
use pkm_markdown::linker::extract_links;

/// ITEM 1 — a block save issued without a `title` must not strip frontmatter fields
/// (tags/aliases/created) from the on-disk .md file, and must not erase them from the
/// DB page record either.
#[test]
fn save_blocks_title_none_preserves_file_and_db_frontmatter() {
    let tv = create_test_vault();

    // Page on disk with rich frontmatter and one block.
    let existing = "---\ntitle: Keep Me\ntags:\n  - alpha\n  - beta\naliases:\n  - alt\ncreated: 2026-01-01\n---\n\n- original body\n";
    let full = tv.create_md_file("pages/note.md", existing);

    // Simulate the save path: parse the on-disk file, serialize its blocks, then
    // re-assemble WITHOUT a title (this is how the outliner saves).
    let (_fm, _body, blocks) = parse_document(existing);
    let serialized = serialize_blocks(&blocks);
    let out = assemble_blocks_markdown(existing, &serialized, None);

    // Write back to disk exactly as save_blocks does, then verify the file retained
    // its frontmatter fields.
    std::fs::write(&full, &out).unwrap();
    let written = std::fs::read_to_string(&full).unwrap();
    assert!(
        written.contains("alpha"),
        "tags must survive a title-less save"
    );
    assert!(
        written.contains("beta"),
        "tags must survive a title-less save"
    );
    assert!(
        written.contains("alt"),
        "aliases must survive a title-less save"
    );
    assert!(
        written.contains("created: 2026-01-01"),
        "created must survive a title-less save"
    );

    // DB-side retention: parse the written file into a Page and upsert it, then assert
    // the page's stored frontmatter still carries the metadata.
    let (fm, _, blocks) = parse_document(&written);
    let mut page = Page::new(full, &tv.vault_path);
    page.frontmatter = pkm_block::PageFrontmatter {
        title: fm.title,
        created: fm.created,
        modified: fm.modified,
        tags: fm.tags,
        aliases: fm.aliases,
        ..Default::default()
    };
    page.set_blocks(&blocks);
    tv.store.upsert_page(&page).unwrap();

    let stored = tv
        .store
        .get_page("pages/note.md")
        .unwrap()
        .expect("page present");
    assert_eq!(stored.tags, vec!["alpha".to_string(), "beta".to_string()]);
    assert_eq!(stored.aliases, vec!["alt".to_string()]);
    assert_eq!(stored.created.as_deref(), Some("2026-01-01"));
}

/// ITEM 3 — saving a page whose blocks contain `[[Target]]` wiki-links populates the
/// links table, and re-writing those blocks removes stale links for the page.
#[test]
fn saving_page_popsulates_and_cleans_links_table() {
    let tv = create_test_vault();

    // Insert blocks with wiki-links into the page, then reconcile the links table the
    // same way save_blocks does inside its transaction.
    let b1 = tv.add_block("pages/a.md", "See [[Target]] for more");
    let b2 = tv.add_block("pages/a.md", "Plain text, no link");
    let blocks = tv.store.get_blocks_by_page("pages/a.md").unwrap();

    tv.store.delete_links_for_page("pages/a.md").unwrap();
    for block in &blocks {
        for link in extract_links(&block.content) {
            tv.store
                .insert_link(block.id, "page_ref", Some(&link.target), None)
                .unwrap();
        }
    }

    // The backlink lookup now returns the new rows planted by the reconcile.
    let backlinks = tv.store.get_backlinks_for_page("Target").unwrap();
    assert_eq!(backlinks, vec![b1.id.to_string()]);
    // The non-linking block contributed nothing.
    assert!(!backlinks.contains(&b2.id.to_string()));

    // Re-saving the page with the link removed must clear the stale row: delete the
    // blocks and their links, then re-reconcile with the new content.
    tv.store.delete_blocks_by_page("pages/a.md").unwrap();
    let new_block = Block::new(uuid::Uuid::new_v4(), "No link now".into());
    tv.store.insert_block(&new_block, "pages/a.md").unwrap();
    let new_blocks = tv.store.get_blocks_by_page("pages/a.md").unwrap();
    tv.store.delete_links_for_page("pages/a.md").unwrap();
    for block in &new_blocks {
        for link in extract_links(&block.content) {
            tv.store
                .insert_link(block.id, "page_ref", Some(&link.target), None)
                .unwrap();
        }
    }

    assert!(
        tv.store
            .get_backlinks_for_page("Target")
            .unwrap()
            .is_empty(),
        "stale link must be removed when blocks no longer reference it"
    );
}
