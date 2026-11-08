//! Headless tests for the whiteboard service commands:
//! `list_whiteboards`, `save_whiteboard`, `load_whiteboard`,
//! `rename_whiteboard`, `delete_whiteboard`, `save_library`, `load_library`,
//! `load_extra_libraries`.

mod common;

use common::create_test_vault;

#[tokio::test]
async fn whiteboard_lifecycle_round_trips_on_disk() {
    let vault = create_test_vault();
    let content = r#"{"type":"excalidraw","elements":[]}"#;

    vault
        .service
        .save_whiteboard("board".into(), content.into())
        .await
        .unwrap();

    let boards = vault.service.list_whiteboards().await.unwrap();
    assert!(boards.iter().any(|b| b.name == "board"));
    assert_eq!(
        vault.service.load_whiteboard("board".into()).await.unwrap(),
        content
    );

    vault
        .service
        .rename_whiteboard("board".into(), "renamed".into())
        .await
        .unwrap();
    assert!(vault.exists("whiteboards/renamed.excalidraw"));
    assert!(!vault.exists("whiteboards/board.excalidraw"));

    vault
        .service
        .delete_whiteboard("renamed".into())
        .await
        .unwrap();
    assert!(vault.service.list_whiteboards().await.unwrap().is_empty());
}

#[tokio::test]
async fn load_whiteboard_returns_empty_canvas_when_missing() {
    let vault = create_test_vault();

    let content = vault
        .service
        .load_whiteboard("missing".into())
        .await
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(parsed["elements"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn library_round_trips_and_extra_libraries_merge() {
    let vault = create_test_vault();
    let library = r#"[{"id":"lib-item"}]"#;
    let extra = r#"[{"id":"extra-item"}]"#;

    vault.service.save_library(library.into()).await.unwrap();
    assert_eq!(vault.service.load_library().await.unwrap(), library);

    vault.write_file("whiteboards/extra.excalidrawlib", extra);

    let merged = vault.service.load_extra_libraries().await.unwrap();
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&merged).unwrap();
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0]["id"].as_str(), Some("extra-item"));
}

#[tokio::test]
async fn rename_whiteboard_rejects_existing_destination() {
    let vault = create_test_vault();
    vault
        .service
        .save_whiteboard("a".into(), "{}".into())
        .await
        .unwrap();
    vault
        .service
        .save_whiteboard("b".into(), "{}".into())
        .await
        .unwrap();

    let err = vault
        .service
        .rename_whiteboard("a".into(), "b".into())
        .await
        .unwrap_err();
    assert!(err.contains("already exists"), "got: {err}");
}
