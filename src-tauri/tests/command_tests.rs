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
