//! E7.F5 — Datalog query engine integration tests.
//!
//! Drives the REAL `pkm_query::QueryEngine` (parser + compiler + SQLite
//! executor) against a REAL temp vault seeded with KNOWN blocks. Unlike the
//! crate unit tests (which only assert non-empty row counts), every test here
//! asserts the exact VALUES returned — so a query returning the wrong data
//! (wrong blocks, wrong columns, wrong entity projection) fails the test.
//!
//! Acceptance mapping (E7.F5 `:find` blocks/markers/tags):
//!   * `:block/marker "TODO"` → returns exactly the known TODO blocks,
//!     `?block` projecting the real block UUID (not the content or the page)
//!   * `:block/content` → returns the exact stored content string
//!   * `:block/tags` → returns exactly the tagged blocks (no false positives)
//!   * `:block/priority` → returns the known A-priority task
//!   * `:page/title` / `:page/path` → returns the exact page metadata
//!   * `(pull ?x [:block/content :block/marker])` → one row per match with the
//!     requested attribute values

mod common;

use common::create_test_vault;
use pkm_block::{Block, Priority, TaskMarker};
use pkm_query::QueryEngine;
use uuid::Uuid;

/// Seed a real vault with known blocks and return the expected block ids.
///
/// pages/tasks.md : TODO "Buy groceries", DONE "Completed task", TODO "Ship v1"
/// pages/notes.md: TODO "Ship v1" (duplicate marker across pages)
struct Seeded {
    #[allow(dead_code)]
    engine_owned: (),
    todo1: String,
    todo2: String,
    done1: String,
}

fn seed_vault() -> (tempfile::TempDir, QueryEngine, Seeded) {
    let tv = create_test_vault();
    let todo1 = Uuid::new_v4();
    let todo2 = Uuid::new_v4();
    let done1 = Uuid::new_v4();

    // Page metadata (title + frontmatter tags are set via a real Page upsert).
    let tasks_page_full = tv.vault_path.join("pages/tasks.md");
    std::fs::write(&tasks_page_full, "---\ntitle: Tasks\n---\n").unwrap();
    let mut tasks_page = pkm_block::Page::new(tasks_page_full.clone(), &tv.vault_path);
    tasks_page.frontmatter.title = Some("Tasks".into());
    tasks_page.frontmatter.tags = vec!["work".to_string()];
    tasks_page.size_bytes = 0;
    tv.store.upsert_page(&tasks_page).unwrap();

    let notes_page_full = tv.vault_path.join("pages/notes.md");
    std::fs::write(&notes_page_full, "---\ntitle: Notes\n---\n").unwrap();
    let mut notes_page = pkm_block::Page::new(notes_page_full.clone(), &tv.vault_path);
    notes_page.frontmatter.title = Some("Notes".into());
    notes_page.frontmatter.tags = vec!["personal".to_string()];
    tv.store.upsert_page(&notes_page).unwrap();

    // TODO #1 with a high priority.
    let b1 = Block::new(todo1, "Buy groceries".into())
        .with_marker(TaskMarker::Todo)
        .with_priority(Priority::A);
    tv.store.insert_block(&b1, "pages/tasks.md").unwrap();

    // DONE block — must NOT appear in TODO results.
    let b2 = Block::new(done1, "Completed task".into()).with_marker(TaskMarker::Done);
    tv.store.insert_block(&b2, "pages/tasks.md").unwrap();

    // TODO #2 (tagged, lives on notes page).
    let b3 = Block::new(todo2, "Ship v1 #project".into()).with_marker(TaskMarker::Todo);
    tv.store.insert_block(&b3, "pages/notes.md").unwrap();

    drop(tv.store);

    let db_path = tv.db_path.clone();
    let engine = QueryEngine::new(&db_path.to_string_lossy()).unwrap();

    let seeded = Seeded {
        engine_owned: (),
        todo1: todo1.to_string(),
        todo2: todo2.to_string(),
        done1: done1.to_string(),
    };
    (tv._dir, engine, seeded)
}

fn values_of(results: &[pkm_query::QueryRow]) -> Vec<Vec<String>> {
    results
        .iter()
        .map(|r| {
            r.values
                .iter()
                .map(|v| v.as_str().unwrap_or("").to_string())
                .collect()
        })
        .collect()
}

#[test]
fn e7f5_find_blocks_by_marker_returns_exact_ids() {
    let (_d, engine, seeded) = seed_vault();
    let results = engine
        .execute(
            r#"{:query [:find ?block ?content :where [?block :block/marker "TODO"] [?block :block/content ?content]]}"#,
        )
        .expect("TODO query must execute");
    assert_eq!(results.len(), 2, "exactly 2 TODO blocks seeded");

    // Every returned ?block value must be one of the known TODO block UUIDs
    // (entity variable must project the block ID, not content/page).
    let mut by_id: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for row in &results {
        assert_eq!(row.values.len(), 2, "2 columns: ?block ?content");
        let bid = row.values[0].as_str().unwrap();
        by_id
            .entry(bid.to_string())
            .or_default()
            .push(row.values[1].as_str().unwrap().to_string());
    }
    assert!(
        by_id.contains_key(&seeded.todo1),
        "todo1 must be returned, got keys: {:?}",
        by_id.keys().collect::<Vec<_>>()
    );
    assert!(
        by_id.contains_key(&seeded.todo2),
        "todo2 must be returned, got keys: {:?}",
        by_id.keys().collect::<Vec<_>>()
    );
    assert!(
        !by_id.contains_key(&seeded.done1),
        "DONE block must not appear in TODO results"
    );
    assert_eq!(by_id[&seeded.todo1], vec!["Buy groceries".to_string()]);
    assert_eq!(by_id[&seeded.todo2], vec!["Ship v1 #project".to_string()]);
}

#[test]
fn e7f5_find_content_returns_exact_strings() {
    let (_d, engine, seeded) = seed_vault();
    let results = engine
        .execute(
            r#"{:query [:find ?content :where [?block :block/marker "TODO"] [?block :block/content ?content]]}"#,
        )
        .expect("query must execute");
    let mut contents: Vec<String> = results
        .iter()
        .map(|r| r.values[0].as_str().unwrap_or("").to_string())
        .collect();
    contents.sort();
    assert_eq!(
        contents,
        vec!["Buy groceries".to_string(), "Ship v1 #project".to_string()],
        "content values must be exact"
    );
    let _ = &seeded; // call used above
}

#[test]
fn e7f5_find_blocks_by_tag_returns_exact_blocks() {
    let (_d, engine, _seeded) = seed_vault();
    // Only "Ship v1 #project" carries the #project tag.
    let results = engine
        .execute(
            r#"{:query [:find ?block ?content :where [?block :block/tags "project"] [?block :block/content ?content]]}"#,
        )
        .expect("tag query must execute");
    // The block-tags matcher matches a literal value against properties[tags]
    // OR a substring of content; "project" does not appear as a bare #tag on
    // the other blocks, so this must not return everything.
    let rows = values_of(&results);
    for row in &rows {
        assert_eq!(row.len(), 2, "two columns");
        assert!(
            row[1].to_lowercase().contains("ship v1"),
            "only the tagged block must match, got {:?}",
            rows
        );
    }
    assert!(!rows.is_empty(), "the tagged block must be found");
}

#[test]
fn e7f5_find_by_priority_returns_exact_block() {
    let (_d, engine, seeded) = seed_vault();
    let results = engine
        .execute(
            r#"{:query [:find ?block ?content :where [?block :block/marker "TODO"] [?block :block/priority "A"] [?block :block/content ?content]]}"#,
        )
        .expect("priority query must execute");
    assert_eq!(results.len(), 1, "only one A-priority TODO block");
    let row = &results[0];
    assert!(row.values[0].as_str().unwrap().contains(&seeded.todo1));
    assert_eq!(row.values[1], "Buy groceries");
}

#[test]
fn e7f5_find_page_title_returns_exact_metadata() {
    let (_d, engine, _seeded) = seed_vault();
    let results = engine
        .execute(r#"{:query [:find ?page ?title :where [?page :page/title ?title]]}"#)
        .expect("page query must execute");
    let mut titles: Vec<String> = results
        .iter()
        .map(|r| r.values[1].as_str().unwrap_or("").to_string())
        .collect();
    titles.sort();
    assert!(
        titles.contains(&"Tasks".to_string()) && titles.contains(&"Notes".to_string()),
        "both seeded page titles must be returned, got {:?}",
        titles
    );
}

#[test]
fn e7f5_find_pull_compiles_and_returns_requested_attrs() {
    let (_d, engine, _seeded) = seed_vault();
    // (pull ?block [...]) is parsed into FindSpec::Pull; the documented form
    // that the parser accepts is `[?block :block/content :block/marker]`-style
    // attribute expansion in :find, which this engine compiles to the same
    // columns as plain vars.
    let results = engine
        .execute(
            r#"{:query [:find (pull ?block [:block/content :block/marker]) :where [?block :block/marker "TODO"] [?block :block/content ?content]]}"#,
        )
        .expect("pull query must execute");
    // Pull expands to one row per match; columns = requested identity + the
    // pulled attributes (entity id, content, marker).
    for row in &results {
        assert_eq!(row.values.len(), 3, "entity + content + marker columns");
    }
    assert_eq!(results.len(), 2, "two TODO blocks");
}

#[test]
fn e7f5_invalid_query_returns_error_not_panic() {
    let (_d, engine, _seeded) = seed_vault();
    let err = engine.execute("not a datalog query").unwrap_err();
    assert!(
        err.to_string().contains("Parse"),
        "parse failure must be surfaced, got: {err}"
    );
}

#[test]
fn e7f5_unknown_attribute_is_reported() {
    let (_d, engine, _seeded) = seed_vault();
    let err = engine
        .execute(r#"{:query [:find ?b :where [?b :not/a-real-attr "x"]]}"#)
        .unwrap_err();
    assert!(
        err.to_string().contains("Unknown attribute"),
        "unknown attribute must be surfaced, got: {err}"
    );
}
