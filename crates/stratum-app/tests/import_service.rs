//! Headless tests for the import service commands:
//! `preview_import`, `import_vault`.

mod common;

use common::{create_test_vault, temp_dir};
use pkm_import::{ImportFormat, ImportOptions};

fn generic_options(destination: std::path::PathBuf, dry_run: bool) -> ImportOptions {
    ImportOptions {
        destination,
        format: ImportFormat::Generic,
        dry_run,
        subdir: None,
    }
}

#[tokio::test]
async fn preview_import_reports_planned_pages_without_writing() {
    let vault = create_test_vault();
    let source = temp_dir();
    std::fs::write(source.path().join("note.md"), "# Note\n\nbody\n").unwrap();

    let preview = vault
        .service
        .preview_import(
            source.path().to_string_lossy().to_string(),
            generic_options(vault.vault_path.clone(), true),
        )
        .await
        .unwrap();

    assert!(preview.total_pages >= 1, "must plan the markdown page");
    assert!(
        preview
            .pages
            .iter()
            .all(|p| p.rel_path.starts_with("imported/")),
        "preview must target imported/: {:?}",
        preview.pages
    );
    assert!(
        !vault.exists(&preview.pages[0].rel_path),
        "preview must not write to disk"
    );
}

#[tokio::test]
async fn import_vault_writes_files_and_syncs_the_page() {
    let vault = create_test_vault();
    let source = temp_dir();
    std::fs::write(
        source.path().join("welcome.md"),
        "---\ntitle: Welcome\n---\n\n- hi\n",
    )
    .unwrap();

    let report = vault
        .service
        .import_vault(
            source.path().to_string_lossy().to_string(),
            generic_options(vault.vault_path.clone(), false),
        )
        .await
        .unwrap();

    assert!(!report.dry_run);
    assert!(report.pages_imported >= 1, "must import the page");
    assert!(!report.imported_paths.is_empty());
    let imported_rel = &report.imported_paths[0];
    assert!(
        vault.exists(imported_rel),
        "imported file must exist at {imported_rel}"
    );

    let pages = vault.service.list_pages().await.unwrap();
    assert!(
        pages.pages.iter().any(|p| &p.path == imported_rel),
        "imported page must be registered in SQLite"
    );
}

#[tokio::test]
async fn import_vault_dry_run_writes_nothing() {
    let vault = create_test_vault();
    let source = temp_dir();
    std::fs::write(source.path().join("dry.md"), "- dry run\n").unwrap();

    let report = vault
        .service
        .import_vault(
            source.path().to_string_lossy().to_string(),
            generic_options(vault.vault_path.clone(), true),
        )
        .await
        .unwrap();

    assert!(report.dry_run);
    assert_eq!(report.pages_imported, 0);
    assert!(report.imported_paths.is_empty());
}
