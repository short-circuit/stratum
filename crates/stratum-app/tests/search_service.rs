//! Headless tests for the search service commands:
//! `search_blocks`, `search_by_tag`, `rebuild_search_index`, `get_backlinks`,
//! `get_page_backlinks`, `get_backlink_snippet`, `autocomplete`,
//! `suggest_connections`, `get_backlink_context`.

mod common;

use common::{block_dto, create_test_vault, RecordingSink};

#[tokio::test]
async fn search_blocks_finds_indexed_content() {
    let vault = create_test_vault();
    vault
        .service
        .save_blocks(
            "pages/rust.md".into(),
            vec![block_dto("memory safety in rust")],
            None,
        )
        .await
        .unwrap();

    let results = vault
        .service
        .search_blocks("memory safety".into(), Some(10))
        .await
        .unwrap();

    assert!(
        results
            .results
            .iter()
            .any(|r| r.page_path == "pages/rust.md" && r.content.contains("memory safety")),
        "search must return the indexed block: {:?}",
        results.results
    );
}

#[tokio::test]
async fn search_by_tag_finds_inline_tag() {
    let vault = create_test_vault();
    vault
        .service
        .save_blocks(
            "pages/tagged.md".into(),
            vec![block_dto("a note about #rust")],
            None,
        )
        .await
        .unwrap();

    let results = vault.service.search_by_tag("rust".into()).await.unwrap();

    assert!(
        results
            .results
            .iter()
            .any(|r| r.page_path == "pages/tagged.md"),
        "tag search must surface the page: {:?}",
        results.results
    );
}

#[tokio::test]
async fn rebuild_search_index_indexes_pages_and_emits_progress() {
    let vault = create_test_vault();
    let sink = RecordingSink::new();
    vault
        .service
        .save_blocks("pages/one.md".into(), vec![block_dto("alpha")], None)
        .await
        .unwrap();
    vault
        .service
        .save_blocks("pages/two.md".into(), vec![block_dto("beta")], None)
        .await
        .unwrap();

    let summary = vault.service.rebuild_search_index(&sink).await.unwrap();

    assert!(summary.contains("Indexed"), "summary: {summary}");
    assert!(sink.has_event("reindex-progress"));
}

#[tokio::test]
async fn get_page_backlinks_finds_linking_block() {
    let vault = create_test_vault();
    vault
        .service
        .create_page("pages/target.md".into(), Some("Target".into()))
        .await
        .unwrap();
    vault
        .service
        .save_blocks(
            "pages/source.md".into(),
            vec![block_dto("see [[target]] for details")],
            None,
        )
        .await
        .unwrap();

    let backlinks = vault
        .service
        .get_page_backlinks("pages/target.md".into())
        .await
        .unwrap();

    assert!(
        backlinks
            .iter()
            .any(|b| b.source_page == "pages/source.md" && b.is_linked),
        "backlink must be linked: {backlinks:?}"
    );
}

#[tokio::test]
async fn get_backlinks_returns_block_reference_sources() {
    let vault = create_test_vault();
    let target = block_dto("target block");
    let target_id = target.id.clone();
    vault
        .service
        .save_blocks("pages/target.md".into(), vec![target], None)
        .await
        .unwrap();

    let source = block_dto("source block");
    let source_id = source.id.clone();
    vault
        .service
        .save_blocks("pages/source.md".into(), vec![source], None)
        .await
        .unwrap();

    // Block-level backlinks are stored as `block_ref` rows.
    vault
        .store()
        .insert_link(
            uuid::Uuid::parse_str(&source_id).unwrap(),
            "block_ref",
            None,
            Some(uuid::Uuid::parse_str(&target_id).unwrap()),
        )
        .unwrap();

    let results = vault.service.get_backlinks(target_id).await.unwrap();
    assert_eq!(results.results.len(), 1);
    assert_eq!(results.results[0].block_id, source_id);
    assert_eq!(results.results[0].page_path, "pages/source.md");
}

#[tokio::test]
async fn get_backlink_snippet_returns_anchor_and_context() {
    let vault = create_test_vault();
    let blocks = vec![
        block_dto("intro paragraph"),
        block_dto("anchor with [[target]]"),
        block_dto("follow-up paragraph"),
    ];
    let anchor_id = blocks[1].id.clone();
    vault
        .service
        .save_blocks("pages/note.md".into(), blocks, Some("Note".into()))
        .await
        .unwrap();

    let snippet = vault
        .service
        .get_backlink_snippet("pages/note.md".into(), anchor_id.clone())
        .await
        .unwrap();

    assert_eq!(snippet.note_id, "pages/note.md");
    assert_eq!(snippet.note_title, "Note");
    assert_eq!(snippet.anchor_id, anchor_id);
    assert_eq!(snippet.anchor_content, "anchor with [[target]]");
    assert!(
        snippet.context.len() >= 3,
        "context must include surrounding blocks: {:?}",
        snippet.context
    );
}

#[tokio::test]
async fn autocomplete_finds_pages_by_query() {
    let vault = create_test_vault();
    vault
        .service
        .create_page("pages/rust-notes.md".into(), None)
        .await
        .unwrap();

    let items = vault
        .service
        .autocomplete("rust".into(), "page".into())
        .await
        .unwrap();

    assert!(
        items
            .iter()
            .any(|i| i.kind == "page" && i.text.contains("rust")),
        "autocomplete must return the page: {items:?}"
    );
}

#[tokio::test]
async fn suggest_connections_finds_related_page() {
    let vault = create_test_vault();
    let sink = RecordingSink::new();
    vault
        .service
        .save_blocks(
            "pages/rust.md".into(),
            vec![block_dto("memory safety systems programming rust")],
            None,
        )
        .await
        .unwrap();
    vault
        .service
        .save_blocks(
            "pages/systems.md".into(),
            vec![block_dto("systems programming and memory safety")],
            None,
        )
        .await
        .unwrap();
    vault.service.rebuild_search_index(&sink).await.unwrap();

    let suggestions = vault
        .service
        .suggest_connections("pages/rust.md".into())
        .await
        .unwrap();

    assert!(
        suggestions
            .iter()
            .any(|s| s.page_path == "pages/systems.md"),
        "related note must be suggested: {suggestions:?}"
    );
}

#[tokio::test]
async fn get_backlink_context_finds_linking_block_in_target_page() {
    let vault = create_test_vault();
    vault
        .service
        .create_page("pages/current.md".into(), Some("Current".into()))
        .await
        .unwrap();
    vault
        .service
        .save_blocks(
            "pages/target.md".into(),
            vec![block_dto("context mentions [[current]] here")],
            Some("Target".into()),
        )
        .await
        .unwrap();

    let context = vault
        .service
        .get_backlink_context("pages/target.md".into(), "pages/current.md".into())
        .await
        .unwrap()
        .expect("context must exist");

    assert!(context.content.contains("[[current]]"));
    assert_eq!(context.page_title.as_deref(), Some("Target"));
}
