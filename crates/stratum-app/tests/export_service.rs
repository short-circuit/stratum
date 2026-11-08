//! Headless tests for the export service commands:
//! `export_html`, `export_json`, `export_pdf`, `export_zip`.

mod common;

use common::{block_dto, create_test_vault};
use std::path::Path;

/// Seed a page through the service so it exists both on disk and in the block
/// store (the export commands enumerate pages from the store).
async fn seed_page(vault: &common::TestVault) {
    vault
        .service
        .save_blocks(
            "pages/export-me.md".into(),
            vec![block_dto("exported body")],
            Some("Export Me".into()),
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn export_html_writes_site_and_index() {
    let vault = create_test_vault();
    seed_page(&vault).await;
    let out = tempfile::tempdir().unwrap();

    let result = vault
        .service
        .export_html(out.path().to_string_lossy().to_string())
        .await
        .unwrap();

    assert!(result.pages_exported >= 1);
    assert!(out.path().join("index.html").exists());
    assert!(out.path().join("pages").join("export-me.html").exists());
}

#[tokio::test]
async fn export_json_writes_page_json() {
    let vault = create_test_vault();
    seed_page(&vault).await;
    let out = tempfile::tempdir().unwrap();

    let result = vault
        .service
        .export_json(out.path().to_string_lossy().to_string())
        .await
        .unwrap();

    assert!(result.pages_exported >= 1);
    let json_path = out.path().join("pages").join("export-me.json");
    assert!(json_path.exists());
    let body: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&json_path).unwrap()).unwrap();
    assert_eq!(body["title"].as_str(), Some("Export Me"));
}

#[tokio::test]
async fn export_zip_creates_archive() {
    let vault = create_test_vault();
    seed_page(&vault).await;
    let out = tempfile::tempdir().unwrap();
    let zip_path = out.path().join("vault.zip");

    let result = vault
        .service
        .export_zip(Some(zip_path.to_string_lossy().to_string()))
        .await
        .unwrap();

    assert_eq!(result.output_path, zip_path.to_string_lossy());
    assert!(zip_path.exists());
    assert!(std::fs::metadata(&zip_path).unwrap().len() > 0);
}

#[tokio::test]
async fn export_pdf_writes_file_when_chromium_is_available() {
    if stratum_app::export::find_chromium().is_none() {
        eprintln!("skipping export_pdf: no chromium/chrome binary on PATH");
        return;
    }

    let vault = create_test_vault();
    seed_page(&vault).await;
    let out = tempfile::tempdir().unwrap();
    let pdf_path = out.path().join("page.pdf");

    let result = vault
        .service
        .export_pdf(
            "pages/export-me.md".into(),
            Some(pdf_path.to_string_lossy().to_string()),
        )
        .await
        .unwrap();

    assert_eq!(Path::new(&result.output_path), pdf_path.as_path());
    assert!(pdf_path.exists());
    assert!(std::fs::metadata(&pdf_path).unwrap().len() > 0);
}
