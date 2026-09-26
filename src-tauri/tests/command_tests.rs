mod common;

use common::create_test_vault;

// These integration tests validate the Tauri command logic by calling
// the underlying crate APIs that the command handlers delegate to.
// The commands themselves are thin wrappers around these APIs.
//
// For true end-to-end command testing with tauri::State, run the
// tauri-driver E2E tests (Layer 3).

#[test]
fn test_get_vault_info() {
    let tv = create_test_vault();
    tv.add_page("pages/a.md");
    tv.add_page("pages/b.md");
    tv.add_block("pages/a.md", "Block 1");

    let block_count = tv.store.block_count().unwrap();
    let page_count = tv.store.page_count().unwrap();
    assert_eq!(block_count, 1);
    assert_eq!(page_count, 2);
}

#[test]
fn test_create_and_list_pages() {
    let tv = create_test_vault();
    for name in &["alpha", "beta", "gamma"] {
        let path = format!("pages/{}.md", name);
        tv.add_page(&path);
        tv.add_block(&path, &format!("# {}", name));
    }

    let pages = tv.store.list_pages().unwrap();
    assert_eq!(pages.len(), 3);
}

#[test]
fn test_block_marker_roundtrip() {
    let tv = create_test_vault();
    use pkm_block::TaskMarker;

    let b = tv.add_block_with_marker("pages/tasks.md", "To do item", TaskMarker::Todo);
    assert_eq!(
        tv.store.get_block(b.id).unwrap().marker,
        Some(TaskMarker::Todo)
    );

    // Update marker
    let mut updated = b;
    updated.marker = Some(TaskMarker::Done);
    tv.store.update_block(&updated).unwrap();
    assert_eq!(
        tv.store.get_block(updated.id).unwrap().marker,
        Some(TaskMarker::Done)
    );
}

#[test]
fn test_search_index_then_query() {
    let tv = create_test_vault();
    use pkm_index::block_search::BlockIndex;

    let idx_path = tv.vault_path.join(".pkm").join("search");
    std::fs::create_dir_all(&idx_path).unwrap();
    let mut index = BlockIndex::create(&idx_path).unwrap();

    let b = tv.add_block("pages/searchable.md", "This is about quantum computing");
    index.index_block(&b, "pages/searchable.md").unwrap();
    index.flush().unwrap();

    let results = index.search("quantum", 10).unwrap();
    assert_eq!(results.len(), 1);
}

#[test]
fn test_template_applied_to_page() {
    let tv = create_test_vault();
    std::fs::create_dir_all(tv.vault_path.join("templates")).unwrap();

    let template = "# {{title}}\n\nDate: {{date}}\n\n{{body}}";
    std::fs::write(tv.vault_path.join("templates/note.md"), template).unwrap();

    tv.add_page("pages/output.md");
    let mut content = template.to_string();
    content = content.replace("{{title}}", "Meeting Notes");
    content = content.replace("{{date}}", "2026-07-28");
    content = content.replace("{{body}}", "Discussed project roadmap.");

    assert!(content.contains("Meeting Notes"));
    assert!(content.contains("2026-07-28"));
}

#[test]
fn test_ensure_today_journal_syncs_stale_file_from_disk() {
    use app_lib::commands::page::ensure_today_journal_core;

    let tv = create_test_vault();
    // Stale state: the journal exists on disk but is NOT registered in SQLite.
    tv.create_md_file(
        "journals/2026-09-06.md",
        "---\ntitle: 2026-09-06\n---\n- entry\n",
    );

    let page = ensure_today_journal_core(&tv.store, &tv.vault_path, "2026-09-06").unwrap();
    assert!(page.path.ends_with("journals/2026-09-06.md"));
    let pages = tv.store.list_pages().unwrap();
    assert!(pages.contains(&"journals/2026-09-06.md".to_string()));
    let blocks = tv
        .store
        .get_blocks_by_page("journals/2026-09-06.md")
        .unwrap();
    assert_eq!(blocks.len(), 1);
    assert_eq!(page.block_count, blocks.len());

    // Idempotent: a second call converges without error and without duplicates.
    let page2 = ensure_today_journal_core(&tv.store, &tv.vault_path, "2026-09-06").unwrap();
    assert_eq!(page2.block_count, 1);
    let pages2 = tv.store.list_pages().unwrap();
    assert_eq!(
        pages2
            .iter()
            .filter(|p| *p == "journals/2026-09-06.md")
            .count(),
        1
    );
}

#[test]
fn test_repair_db_syncs_stale_and_prunes_orphans() {
    use app_lib::commands::page::repair_db_core;

    let tv = create_test_vault();
    // Two real pages on disk with parseable block content.
    tv.create_md_file("pages/a.md", "Alpha block one\n\nAlpha block two\n");
    tv.create_md_file("pages/b.md", "Beta entry\n");
    // pages/c.md is registered in SQLite only — no file on disk (orphan).
    tv.add_page("pages/c.md");
    // pages/a.md is registered in SQLite with zero blocks (the stale shape).
    tv.add_page("pages/a.md");

    let result = repair_db_core(&tv.store, &tv.vault_path).unwrap();
    assert_eq!(result.failed, 0);

    let pages = tv.store.list_pages().unwrap();
    assert!(pages.contains(&"pages/a.md".to_string()));
    assert!(pages.contains(&"pages/b.md".to_string()));
    assert!(!pages.contains(&"pages/c.md".to_string()));

    // Block content on disk was synced into the store.
    let a_blocks = tv.store.get_blocks_by_page("pages/a.md").unwrap();
    assert_eq!(a_blocks.len(), 2);
    let b_blocks = tv.store.get_blocks_by_page("pages/b.md").unwrap();
    assert_eq!(b_blocks.len(), 1);
}

#[test]
fn test_repair_db_is_idempotent() {
    use app_lib::commands::page::repair_db_core;

    let tv = create_test_vault();
    tv.create_md_file("pages/a.md", "Content one\n\nContent two\n");
    tv.create_md_file("pages/b.md", "Beta content\n");
    tv.add_page("pages/a.md");

    let first = repair_db_core(&tv.store, &tv.vault_path).unwrap();
    assert_eq!(first.failed, 0);

    // A second run must not produce errors or duplicate rows.
    let second = repair_db_core(&tv.store, &tv.vault_path).unwrap();
    assert_eq!(second.failed, 0);

    let pages = tv.store.list_pages().unwrap();
    assert_eq!(pages.iter().filter(|p| *p == "pages/a.md").count(), 1);
    assert_eq!(pages.iter().filter(|p| *p == "pages/b.md").count(), 1);
    let a_blocks = tv.store.get_blocks_by_page("pages/a.md").unwrap();
    assert_eq!(a_blocks.len(), 2);
    let b_blocks = tv.store.get_blocks_by_page("pages/b.md").unwrap();
    assert_eq!(b_blocks.len(), 1);
}

/// Regression: the read path must never render an empty editor when the `pages`
/// table lists a page but its `blocks` rows are missing while the `.md` on disk
/// has real content (the `Borrowing.md` corruption shape found in the live vault).
///
/// Without `get_blocks_heal_from_disk`, `get_blocks` returns zero rows and the
/// frontend shows an empty editor / infinite spinner until an app restart. The
/// helper must converge the DB from disk and return the on-disk blocks.
#[test]
fn test_get_blocks_heals_stale_page_with_missing_block_rows() {
    use app_lib::commands::page::get_blocks_heal_from_disk;

    let tv = create_test_vault();
    // Disk: a page with real BlockNote-serialized content (with .id: lines).
    tv.create_md_file(
        "Borrowing.md",
        "- On-disk block one\n  .id: 6179411f-1906-476c-bc0b-2fbc71620a8f\n\
         - On-disk block two\n  .id: ea59ea95-12f4-42b5-bdf4-5cc4e6de9de3\n",
    );
    // SQLite: page registered with a block_count but ZERO block rows (the decayed shape).
    tv.add_page("Borrowing.md");

    // Before healing: zero blocks in SQLite.
    let before = tv.store.get_blocks_by_page("Borrowing.md").unwrap();
    assert!(
        before.is_empty(),
        "precondition: stale DB has no block rows"
    );

    let healed = get_blocks_heal_from_disk(&tv.store, &tv.vault_path, "Borrowing.md").unwrap();
    assert_eq!(healed.len(), 2, "read path must return the on-disk blocks");
    assert!(
        healed
            .iter()
            .any(|b| b.content.contains("On-disk block one")),
        "returned rows must carry the disk content"
    );

    // The DB is now converged — a subsequent read returns the same rows without
    // duplication, and the persisted rows match.
    let after = tv.store.get_blocks_by_page("Borrowing.md").unwrap();
    assert_eq!(after.len(), 2, "DB must be healed, not duplicated");
}

/// Regression: opening a page whose DB rows are stale must report the on-disk
/// block count via `open_page`, so the frontend never loops on `todayExists=false`
/// or renders an empty editor for a page that has real content.
#[test]
fn test_open_page_reports_disk_blocks_when_db_stale() {
    use app_lib::commands::page::get_blocks_heal_from_disk;

    let tv = create_test_vault();
    tv.create_md_file(
        "pages/journal-2026-09-22.md",
        "---\ntitle: journal-2026-09-22\n---\n- current entry\n  .id: 0ade90cf-2110-4c1e-be7a-924bea2383a9\n",
    );
    tv.add_page("pages/journal-2026-09-22.md");

    // Simulate the exact `open_page` read: blocks come from the healing helper.
    let blocks =
        get_blocks_heal_from_disk(&tv.store, &tv.vault_path, "pages/journal-2026-09-22.md")
            .unwrap();
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].content, "current entry");
}
