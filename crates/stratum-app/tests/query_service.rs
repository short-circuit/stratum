//! Headless tests for the Datalog query service commands:
//! `run_query`, `list_saved_queries`, `save_saved_query`,
//! `rename_saved_query`, `delete_saved_query`.

mod common;

use common::{block_dto, create_test_vault};

#[tokio::test]
async fn run_query_executes_datalog_against_blocks() {
    let vault = create_test_vault();
    let mut block = block_dto("a todo block");
    block.marker = Some("TODO".into());
    vault
        .service
        .save_blocks("pages/tasks.md".into(), vec![block], None)
        .await
        .unwrap();

    let result = vault
        .service
        .run_query(
            r#"{:query [:find ?b ?content :where [?b :block/marker "TODO"] [?b :block/content ?content]]}"#
                .into(),
        )
        .await
        .unwrap();

    assert_eq!(result.columns.len(), 2);
    assert_eq!(result.rows.len(), 1);
    assert!(result.rows[0].iter().any(|v| v == "a todo block"));
}

#[tokio::test]
async fn saved_query_lifecycle_round_trips_through_disk() {
    let vault = create_test_vault();

    let saved = vault
        .service
        .save_saved_query("todos".into(), "{:query [:find ?b]}".into())
        .await
        .unwrap();
    assert_eq!(saved.name, "todos");

    let listed = vault.service.list_saved_queries().await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].query, "{:query [:find ?b]}");

    let renamed = vault
        .service
        .rename_saved_query("todos".into(), "open-todos".into())
        .await
        .unwrap();
    assert_eq!(renamed.name, "open-todos");

    vault
        .service
        .delete_saved_query("open-todos".into())
        .await
        .unwrap();
    assert!(vault.service.list_saved_queries().await.unwrap().is_empty());
}
