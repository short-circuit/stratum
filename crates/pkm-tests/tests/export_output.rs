//! E7.F5 — HTML / JSON export acceptance tests.
//!
//! These tests drive the REAL export pipeline (`export_html_core` /
//! `export_json_core` in the stratum-tauri command layer): a temp vault is
//! seeded with real `.md` files produced through the same serializer the app
//! uses (`serialize_blocks` + `assemble_blocks_markdown`), then the real
//! export functions are invoked and the emitted artifacts are verified to be
//! valid, complete, and self-consistent.
//!
//! Acceptance mapping (E7.F5 "export HTML/JSON produces valid, complete
//! output for a sample vault"):
//!   * HTML: every seeded page is emitted as `<slug>.html` mirroring the vault
//!     directory structure, plus a linking `index.html`; the body HTML is
//!     well-formed (parses) and contains the rendered page content; the index
//!     links every exported page.
//!   * JSON: every seeded page is emitted as `<page>.json`, the JSON parses,
//!     and the `path` / `title` / `tags` / `body` / `blocks` fields round-trip
//!     the seeded page exactly (block ids, markers and priorities survive).

mod common;

use common::create_test_vault;
use pkm_block::{Block, Priority, TaskMarker};
use pkm_markdown::block_parser::{assemble_blocks_markdown, serialize_blocks};
use pkm_tests::assert_html_well_formed;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Seed a sample vault through the real serializer and return the expected
/// page facts (relative path, title, tags, block data).
struct PageFacts {
    rel_path: String,
    title: String,
    tags: Vec<String>,
    expected_content: String,
    blocks: Vec<(String, String, Option<&'static str>, Option<&'static str>)>,
}

fn seed_sample_vault() -> (tempfile::TempDir, PathBuf, Vec<PageFacts>) {
    let tv = create_test_vault();
    let mut facts = Vec::new();

    // Page 1: pages/tasks.md with two blocks (one TODO+A, one DONE).
    let mut b1 = Block::new(Uuid::new_v4(), "Buy groceries".into());
    b1 = b1.with_marker(TaskMarker::Todo).with_priority(Priority::A);
    let mut b2 = Block::new(Uuid::new_v4(), "Completed task".into());
    b2 = b2.with_marker(TaskMarker::Done);

    let body1 = serialize_blocks(&[b1.clone(), b2.clone()]);
    let md1 = assemble_blocks_markdown(
        "---\ntitle: Tasks\ntags: [work]\n---\n",
        &body1,
        Some("Tasks"),
    );
    let p1 = create_md_file(&tv.vault_path, "pages/tasks.md", &md1);
    assert!(p1.is_file());

    let mut page1 = pkm_block::Page::new(p1, &tv.vault_path);
    page1.frontmatter.title = Some("Tasks".into());
    page1.frontmatter.tags = vec!["work".to_string()];
    tv.store.upsert_page(&page1).unwrap();
    facts.push(PageFacts {
        rel_path: "pages/tasks.md".into(),
        title: "Tasks".into(),
        tags: vec!["work".to_string()],
        expected_content: "Buy groceries".into(),
        blocks: vec![
            (
                b1.id.to_string(),
                "Buy groceries".into(),
                Some("TODO"),
                Some("A"),
            ),
            (
                b2.id.to_string(),
                "Completed task".into(),
                Some("DONE"),
                None,
            ),
        ],
    });

    // Page 2: notes.md with a tagged TODO containing a link + inline tag.
    let mut b3 = Block::new(Uuid::new_v4(), "Ship v1 #project".into());
    b3 = b3.with_marker(TaskMarker::Todo);
    let body2 = serialize_blocks(&[b3.clone()]);
    let md2 = assemble_blocks_markdown(
        "---\ntitle: Notes\ntags: [personal]\n---\n",
        &body2,
        Some("Notes"),
    );
    let p2 = create_md_file(&tv.vault_path, "notes.md", &md2);
    assert!(p2.is_file());
    let mut page2 = pkm_block::Page::new(p2, &tv.vault_path);
    page2.frontmatter.title = Some("Notes".into());
    page2.frontmatter.tags = vec!["personal".to_string()];
    tv.store.upsert_page(&page2).unwrap();
    facts.push(PageFacts {
        rel_path: "notes.md".into(),
        title: "Notes".into(),
        tags: vec!["personal".to_string()],
        expected_content: "Ship v1 #project".into(),
        blocks: vec![(
            b3.id.to_string(),
            "Ship v1 #project".into(),
            Some("TODO"),
            None,
        )],
    });

    let _ = &facts;

    (tv._dir, tv.vault_path, facts)
}

fn create_md_file(vault_root: &Path, rel: &str, content: &str) -> PathBuf {
    let path = vault_root.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&path, content).unwrap();
    path
}

#[test]
fn e7f5_export_html_emits_complete_valid_pages() {
    let (_dir, vault, facts) = seed_sample_vault();

    let out_dir = vault.join("dist");
    let pages: Vec<String> = facts.iter().map(|f| f.rel_path.clone()).collect();
    let res = app_lib::commands::export::export_html_core(&vault, &out_dir, &pages)
        .expect("html export must succeed");
    assert_eq!(res.pages_exported, 2, "both pages exported");
    assert!(out_dir.join("index.html").is_file(), "index.html written");

    // Each page is emitted mirroring the vault directory structure.
    let html_paths = [out_dir.join("pages/tasks.html"), out_dir.join("notes.html")];
    for (idx, hp) in html_paths.iter().enumerate() {
        assert!(
            hp.is_file(),
            "expected HTML at {} (idx {idx})",
            hp.display()
        );
        let content = fs::read_to_string(hp).unwrap();
        assert!(
            content.contains("<!DOCTYPE html>"),
            "valid HTML doctype for {}",
            hp.display()
        );
        // Body content is present and well-formed.
        assert!(
            content.contains(&facts[idx].expected_content),
            "rendered content must appear in {}",
            hp.display()
        );
        assert_html_well_formed(&content, &hp.display().to_string());
    }

    // index.html links every exported page.
    let index = fs::read_to_string(out_dir.join("index.html")).unwrap();
    assert!(index.contains("pages/tasks.html"));
    assert!(index.contains("notes.html"));
    assert_html_well_formed(&index, "index.html");
}

#[test]
fn e7f5_export_json_round_trips_blocks_and_metadata() {
    let (_dir, vault, facts) = seed_sample_vault();

    let out_dir = vault.join("dist-json");
    let pages: Vec<String> = facts.iter().map(|f| f.rel_path.clone()).collect();
    let res = app_lib::commands::export::export_json_core(&vault, &out_dir, &pages)
        .expect("json export must succeed");
    assert_eq!(res.pages_exported, 2, "both pages exported");

    // tasks.md -> tasks.json, notes.md -> notes.json
    let json_paths = [out_dir.join("pages/tasks.json"), out_dir.join("notes.json")];
    for (idx, jp) in json_paths.iter().enumerate() {
        assert!(jp.is_file(), "expected JSON at {}", jp.display());
        let raw = fs::read_to_string(jp).unwrap();
        let parsed: serde_json::Value =
            serde_json::from_str(&raw).unwrap_or_else(|e| panic!("JSON must parse: {e}"));

        let f = &facts[idx];
        assert_eq!(parsed["path"], f.rel_path);
        assert_eq!(parsed["title"], f.title);
        assert_eq!(
            parsed["tags"],
            serde_json::json!(f.tags),
            "tags must round-trip"
        );
        assert!(
            parsed["body"]
                .as_str()
                .unwrap_or("")
                .contains(&f.expected_content),
            "body must contain seeded content"
        );

        // Every seeded block is present with id/content/marker/priority.
        let blocks = parsed["blocks"].as_array().unwrap_or(&vec![]).clone();
        assert_eq!(blocks.len(), f.blocks.len(), "block count must match");
        for (bid, content, marker, priority) in &f.blocks {
            let hit = blocks.iter().find(|b| b["id"] == *bid);
            assert!(hit.is_some(), "block id {bid} must be present");
            let hit = hit.unwrap();
            assert_eq!(hit["content"], *content);
            if let Some(m) = marker {
                assert_eq!(hit["marker"], *m, "marker must round-trip");
            } else {
                assert!(hit["marker"].is_null(), "no marker expected");
            }
            if let Some(p) = priority {
                assert_eq!(hit["priority"], *p, "priority must round-trip");
            } else {
                assert!(hit["priority"].is_null(), "no priority expected");
            }
        }
    }
}

#[test]
fn e7f5_export_skips_missing_files_without_error() {
    let (_dir, vault, _facts) = seed_sample_vault();

    // A page registered in the DB but missing on disk must be skipped, not
    // crash the export.
    let ghost = vault.join("ghost.md");
    let mut page = pkm_block::Page::new(ghost, &vault);
    page.frontmatter.title = Some("Ghost".into());
    let db_path = vault.join(".pkm/blocks.db");
    let store = pkm_block::BlockStore::open(&db_path).unwrap();
    store.upsert_page(&page).unwrap();

    let out_dir = vault.join("dist2");
    let mut pages = Vec::new();
    if let Ok(listed) = store.list_pages() {
        pages = listed;
    }
    let res = app_lib::commands::export::export_html_core(&vault, &out_dir, &pages)
        .expect("export with ghost page must succeed");
    assert!(res.pages_exported >= 2, "real pages still exported");
    assert!(
        !out_dir.join("ghost.html").exists(),
        "missing source page must be skipped, not crash"
    );
}
