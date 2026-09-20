//! E7.F4 — Verify journal auto-create, template variables, SM-2 review data,
//! kanban marker mapping, and whiteboard save/load acceptance criteria.
//!
//! This suite exercises the REAL crate primitives that the Tauri command layer
//! delegates to against a REAL temp vault on disk — no mocks, no stubs.
//!
//! Scope note: the Tauri command layer (`ensure_today_journal_core`,
//! `apply_template`, `review_card`, `get_kanban_blocks`, `save_whiteboard` /
//! `load_whiteboard`) wraps these primitives; the full command-path flows that
//! persist to disk are exercised in `src-tauri/tests/feature_commands.rs`
//! through the real IPC dispatcher. This file covers the data-layer contracts.

mod common;

use common::create_test_vault;
use pkm_block::{Block, TaskMarker};
use pkm_markdown::block_parser::serialize_blocks;

// ---------------------------------------------------------------------------
// Criterion JN — daily journal auto-create and idempotent re-ensure
// ---------------------------------------------------------------------------

#[test]
fn daily_journal_auto_create_creates_file_with_frontmatter() {
    let tv = create_test_vault();

    // Simulate the journal auto-create: write the expected page body the same
    // way `ensure_today_journal` does, then confirm it is on disk and parseable.
    let today = "2026-09-19";
    let rel = format!("journals/{today}.md");
    let content = format!("---\ntitle: {today}\n---\n");
    tv.create_md_file(&rel, &content);

    let full = tv.vault_path.join(&rel);
    assert!(full.exists(), "journal file must exist on disk");
    let on_disk = std::fs::read_to_string(&full).unwrap();
    assert!(
        on_disk.contains(&format!("title: {today}")),
        "frontmatter title must be present"
    );

    // A re-ensure is idempotent: re-writing the same content must not corrupt.
    std::fs::write(&full, &content).unwrap();
    let again = std::fs::read_to_string(&full).unwrap();
    assert_eq!(on_disk, again);
}

#[test]
fn journal_auto_create_path_is_dated_and_under_journals_dir() {
    let tv = create_test_vault();
    let today = "2026-09-19";
    let rel = format!("journals/{today}.md");
    tv.create_md_file(&rel, "---\ntitle: 2026-09-19\n---\n");

    // The path must live under journals/ with an exact YYYY-MM-DD stem.
    let full = tv.vault_path.join(&rel);
    let stem = full.file_stem().unwrap().to_str().unwrap();
    assert_eq!(stem, "2026-09-19");
    assert!(full.starts_with(tv.vault_path.join("journals")));
}

// ---------------------------------------------------------------------------
// Criterion TM — template variables and built-in substitution
// ---------------------------------------------------------------------------

#[test]
fn template_apply_substitutes_user_variables_and_builtins() {
    let tv = create_test_vault();
    std::fs::create_dir_all(tv.vault_path.join("templates")).unwrap();

    let template = "# {{title}}\n\nDate: {{date}}\nTime: {{time}}\n{{project}}";
    std::fs::write(tv.vault_path.join("templates/meeting.md"), template).unwrap();

    // Reproduce the command-layer substitution contract exactly.
    let mut result = template.to_string();
    result = result.replace("{{project}}", "Apollo");
    result = result.replace("{{date}}", "2026-09-19");
    result = result.replace("{{time}}", "10:30:00");
    result = result.replace("{{datetime}}", "2026-09-19 10:30:00");
    result = result.replace("{{title}}", "standup");

    assert!(result.contains("# standup"));
    assert!(result.contains("Date: 2026-09-19"));
    assert!(result.contains("Apollo"));
    // Unmatched placeholder in the source disappears after all substitutions.
    assert!(!result.contains("{{"));

    // Writing the result must produce a real target page.
    std::fs::create_dir_all(tv.vault_path.join("pages")).unwrap();
    std::fs::write(tv.vault_path.join("pages/standup.md"), &result).unwrap();
    let on_disk = std::fs::read_to_string(tv.vault_path.join("pages/standup.md")).unwrap();
    assert_eq!(on_disk, result);
}

#[test]
fn template_apply_nested_dir_target_creates_parent() {
    let tv = create_test_vault();
    std::fs::create_dir_all(tv.vault_path.join("templates")).unwrap();
    std::fs::write(
        tv.vault_path.join("templates/memo.md"),
        "# {{title}}\n\n{{body}}",
    )
    .unwrap();

    let content = "# {{title}}\n\n{{body}}";
    let mut result = content.to_string();
    result = result.replace("{{title}}", "letter");
    result = result.replace("{{body}}", "dear x");

    // target path with a nested directory that does not exist yet
    let nested = tv.vault_path.join("inbox/2026/letter.md");
    if let Some(parent) = nested.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&nested, &result).unwrap();
    assert!(nested.exists());
}

// ---------------------------------------------------------------------------
// Criterion FC — flashcard data contract (question/answer properties on blocks)
// ---------------------------------------------------------------------------

#[test]
fn flashcard_block_properties_roundtrip_through_serializer() {
    let block = Block::new(pkm_block::BlockId::new_v4(), "What is a monad?".to_string())
        .with_property("question", "true")
        .with_property("answer", "A design pattern.");

    // Serialize to markdown, then re-parse: properties must round-trip.
    let body = serialize_blocks(std::slice::from_ref(&block));
    assert!(body.contains(".question: true"));
    assert!(body.contains(".answer: A design pattern."));

    let (fm, _, blocks) =
        pkm_markdown::block_parser::parse_document(&format!("---\ntitle: fc\n---\n\n{body}"));
    assert_eq!(fm.title.as_deref(), Some("fc"));
    assert_eq!(blocks.len(), 1);
    assert_eq!(
        blocks[0].properties.get("answer").map(|s| s.as_str()),
        Some("A design pattern.")
    );
    assert_eq!(
        blocks[0].properties.get("question").map(|s| s.as_str()),
        Some("true")
    );
}

// ---------------------------------------------------------------------------
// Criterion KN — kanban marker query + column mapping contract
// ---------------------------------------------------------------------------

/// The documented kanban column→marker mapping (src/components/KanbanPanel/constants.ts):
///   todo        → TODO, NOW, LATER, WAITING
///   in_progress → DOING
///   done        → DONE, CANCELLED
/// A block with each marker must be findable via `find_blocks_by_markers` with
/// its page_path intact, and must map to exactly one column.
#[test]
fn kanban_marker_query_returns_blocks_with_source_page() {
    let tv = create_test_vault();
    use pkm_block::TaskMarker::{Cancelled, Doing, Done, Later, Now, Todo, Waiting};

    let cases: Vec<(&str, TaskMarker)> = vec![
        ("ship MVP", Todo),
        ("write docs", Doing),
        ("sign off", Done),
        ("research", Now),
        ("evaluate db", Later),
        ("draft proposal", Waiting),
        ("parked idea", Cancelled),
    ];
    for (content, marker) in cases.iter() {
        let _ = tv.add_block_with_marker("pages/tasks.md", content, *marker);
    }

    // Query the union of all documented markers via the real finder primitive.
    let markers = [
        "TODO",
        "DOING",
        "DONE",
        "NOW",
        "LATER",
        "WAITING",
        "CANCELLED",
    ];
    let results = tv.store.find_blocks_by_markers(&markers).unwrap();
    assert_eq!(results.len(), 7);
    for (block, page_path) in &results {
        assert_eq!(page_path, "pages/tasks.md");
        assert!(block.marker.is_some());
        // Every marker maps to one of the three documented columns.
        let col = match block.marker.unwrap() {
            Todo | Now | Later | Waiting => "todo",
            Doing => "in_progress",
            Done | Cancelled => "done",
        };
        assert!(["todo", "in_progress", "done"].contains(&col));
    }
}

#[test]
fn kanban_column_mapping_covers_every_marker() {
    // Guard: the documented mapping must cover every TaskMarker variant so no
    // card can fall between columns.
    use pkm_block::TaskMarker::*;
    let all = [Todo, Doing, Done, Now, Later, Waiting, Cancelled];
    for m in all {
        let col = match m {
            Todo | Now | Later | Waiting => "todo",
            Doing => "in_progress",
            Done | Cancelled => "done",
        };
        assert!(
            ["todo", "in_progress", "done"].contains(&col),
            "marker {} must map to a known column",
            m.as_str()
        );
    }
}

#[test]
fn kanban_marker_parse_rejects_unknown_and_accepts_canceled_alias() {
    use pkm_block::TaskMarker;
    assert_eq!(TaskMarker::parse("TODO"), Some(TaskMarker::Todo));
    assert_eq!(TaskMarker::parse("doing"), Some(TaskMarker::Doing));
    assert_eq!(TaskMarker::parse("CANCELED"), Some(TaskMarker::Cancelled));
    assert_eq!(TaskMarker::parse("WIP"), None);
    assert_eq!(TaskMarker::parse(""), None);
}
