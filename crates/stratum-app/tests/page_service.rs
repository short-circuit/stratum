//! Headless tests for the page service commands:
//! `list_pages`, `open_page`, `save_page`, `create_page`,
//! `ensure_today_journal`, `delete_page`, `move_page`, `reindex_vault`,
//! `reindex_page`, `repair_db_from_disk`, `normalize_file`,
//! `normalize_all_files`.

mod common;

use common::{block_dto, create_test_vault, RecordingSink};

#[tokio::test]
async fn create_page_writes_markdown_and_registers_the_page() {
    let vault = create_test_vault();

    let page = vault
        .service
        .create_page("pages/test".into(), Some("Test".into()))
        .await
        .unwrap();

    assert_eq!(page.path, "pages/test.md");
    assert_eq!(page.slug, "test");
    assert_eq!(page.title.as_deref(), Some("Test"));
    let content = vault.read_file("pages/test.md");
    assert!(
        content.contains("title: Test"),
        "frontmatter written: {content}"
    );

    let list = vault.service.list_pages().await.unwrap();
    assert!(
        list.pages.iter().any(|p| p.path == "pages/test.md"),
        "created page must be listed"
    );
}

#[tokio::test]
async fn create_page_rejects_duplicate() {
    let vault = create_test_vault();
    vault
        .service
        .create_page("pages/dup.md".into(), None)
        .await
        .unwrap();

    let err = vault
        .service
        .create_page("pages/dup.md".into(), None)
        .await
        .unwrap_err();
    assert!(err.contains("Page already exists"), "got: {err}");
}

#[tokio::test]
async fn list_pages_includes_disk_only_journal() {
    let vault = create_test_vault();
    vault.write_file("journals/2020-01-02.md", "---\ntitle: 2020-01-02\n---\n");

    let list = vault.service.list_pages().await.unwrap();
    assert!(
        list.pages
            .iter()
            .any(|p| p.path == "journals/2020-01-02.md"),
        "disk-only journal must surface in list_pages"
    );
}

#[tokio::test]
async fn open_page_returns_metadata_for_saved_content() {
    let vault = create_test_vault();
    vault
        .service
        .save_page(
            "pages/open.md".into(),
            "---\ntitle: Opened\n---\n\n- hello\n".into(),
        )
        .await
        .unwrap();

    let page = vault
        .service
        .open_page("pages/open.md".into())
        .await
        .unwrap();
    assert_eq!(page.title.as_deref(), Some("Opened"));
    assert!(page.block_count >= 1, "saved block must be registered");
}

#[tokio::test]
async fn save_page_persists_content_and_blocks() {
    let vault = create_test_vault();
    let markdown = "---\ntitle: Saved\n---\n\n- first block\n- second block\n";

    vault
        .service
        .save_page("pages/saved.md".into(), markdown.into())
        .await
        .unwrap();

    assert_eq!(vault.read_file("pages/saved.md"), markdown);
    let blocks = vault
        .service
        .get_blocks("pages/saved.md".into())
        .await
        .unwrap();
    assert!(blocks.blocks.len() >= 2, "both blocks must be stored");
}

#[tokio::test]
async fn ensure_today_journal_creates_todays_file() {
    let vault = create_test_vault();

    let page = vault.service.ensure_today_journal().await.unwrap();

    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    assert_eq!(page.path, format!("journals/{today}.md"));
    assert!(vault.exists(&page.path), "journal file must be created");
    assert_eq!(page.slug, today);
}

#[tokio::test]
async fn delete_page_removes_file_and_registration() {
    let vault = create_test_vault();
    vault
        .service
        .create_page("pages/gone.md".into(), None)
        .await
        .unwrap();

    vault
        .service
        .delete_page("pages/gone.md".into())
        .await
        .unwrap();

    assert!(!vault.exists("pages/gone.md"), "file must be removed");
    let list = vault.service.list_pages().await.unwrap();
    assert!(
        !list.pages.iter().any(|p| p.path == "pages/gone.md"),
        "deleted page must not be listed"
    );
}

#[tokio::test]
async fn move_page_renames_and_rewrites_inbound_links() {
    let vault = create_test_vault();
    let sink = RecordingSink::new();

    vault
        .service
        .create_page("pages/a.md".into(), None)
        .await
        .unwrap();
    vault
        .service
        .save_blocks("pages/source.md".into(), vec![block_dto("see [[a]]")], None)
        .await
        .unwrap();

    let dto = vault
        .service
        .move_page("pages/a.md".into(), "pages/b.md".into(), &sink)
        .await
        .unwrap();

    assert_eq!(dto.path, "pages/b.md");
    assert!(!vault.exists("pages/a.md"), "old path must be gone");
    assert!(vault.exists("pages/b.md"), "new path must exist");
    assert!(
        vault.read_file("pages/source.md").contains("[[b]]"),
        "inbound link must be rewritten"
    );
    assert!(
        sink.has_event("pages-changed"),
        "move must emit pages-changed"
    );
}

#[tokio::test]
async fn move_page_rejects_existing_destination() {
    let vault = create_test_vault();
    let sink = RecordingSink::new();
    vault
        .service
        .create_page("pages/a.md".into(), None)
        .await
        .unwrap();
    vault
        .service
        .create_page("pages/b.md".into(), None)
        .await
        .unwrap();

    let err = vault
        .service
        .move_page("pages/a.md".into(), "pages/b.md".into(), &sink)
        .await
        .unwrap_err();
    assert!(err.contains("Page already exists"), "got: {err}");
}

#[tokio::test]
async fn reindex_vault_syncs_disk_pages_into_sqlite() {
    let vault = create_test_vault();
    let sink = RecordingSink::new();
    vault.write_file(
        "pages/disk-only.md",
        "---\ntitle: Disk Only\n---\n\n- content\n",
    );

    let result = vault.service.reindex_vault(&sink).await.unwrap();

    assert!(result.processed >= 1, "must process the disk page");
    assert_eq!(result.failed, 0, "errors: {:?}", result.errors);
    assert!(sink.has_event("reindex-progress"));
    let list = vault.service.list_pages().await.unwrap();
    assert!(list.pages.iter().any(|p| p.path == "pages/disk-only.md"));
}

#[tokio::test]
async fn reindex_page_syncs_one_disk_page() {
    let vault = create_test_vault();
    let sink = RecordingSink::new();
    vault.write_file("pages/single.md", "---\ntitle: Single\n---\n\n- one\n");

    let result = vault
        .service
        .reindex_page("pages/single.md".into(), &sink)
        .await
        .unwrap();

    assert_eq!(result.succeeded, 1);
    assert_eq!(result.failed, 0, "errors: {:?}", result.errors);
}

#[tokio::test]
async fn repair_db_from_disk_heals_missing_page_rows() {
    let vault = create_test_vault();
    let sink = RecordingSink::new();
    vault.write_file("pages/lost.md", "---\ntitle: Lost\n---\n\n- recovered\n");

    let result = vault.service.repair_db_from_disk(&sink).await.unwrap();

    assert!(result.processed >= 1, "must scan the disk page");
    assert_eq!(result.failed, 0, "errors: {:?}", result.errors);
    assert!(sink.has_event("reindex-progress"));
}

#[tokio::test]
async fn normalize_file_rewrites_the_file() {
    let vault = create_test_vault();
    let sink = RecordingSink::new();
    vault.write_file("pages/raw.md", "---\ntitle: Raw\n---\n\n- a block\n");

    vault
        .service
        .normalize_file("pages/raw.md".into(), &sink)
        .await
        .unwrap();

    let content = vault.read_file("pages/raw.md");
    assert!(content.contains("a block"), "content preserved: {content}");
}

#[tokio::test]
async fn normalize_all_files_processes_every_markdown_file() {
    let vault = create_test_vault();
    let sink = RecordingSink::new();
    vault.write_file("pages/one.md", "- first\n");
    vault.write_file("pages/two.md", "- second\n");

    let count = vault.service.normalize_all_files(&sink).await.unwrap();

    assert!(count >= 2, "must normalize both files, got {count}");
    assert!(sink.has_event("reindex-progress"));
}
