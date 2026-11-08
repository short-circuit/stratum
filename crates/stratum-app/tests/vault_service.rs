//! Headless tests for the vault service commands:
//! `get_vault_info`, `set_vault_path`, `init_vault`, `init_vault_at`
//! (the latter backs the `init_default_vault` / picker adapters).

mod common;

use common::{create_test_vault, drop_plugin_runtime, temp_dir};

#[tokio::test]
async fn get_vault_info_reports_vault_path_and_counts() {
    let vault = create_test_vault();
    vault
        .service
        .create_page("pages/hello.md".into(), None)
        .await
        .unwrap();

    let info = vault.service.get_vault_info().await.unwrap();
    assert_eq!(info.path, vault.vault_path.to_string_lossy());
    assert!(
        info.page_count >= 1,
        "page count must include the created page"
    );
}

#[tokio::test]
async fn set_vault_path_switches_the_active_vault() {
    let vault = create_test_vault();
    let other = temp_dir();

    vault
        .service
        .set_vault_path(other.path().to_string_lossy().to_string())
        .await
        .unwrap();

    assert_eq!(
        vault.service.vault_path(),
        other.path().to_path_buf(),
        "service must point at the new vault"
    );
}

#[tokio::test]
async fn set_vault_path_rejects_missing_directory() {
    let vault = create_test_vault();
    let missing = vault.vault_path.join("does-not-exist");

    let err = vault
        .service
        .set_vault_path(missing.to_string_lossy().to_string())
        .await
        .unwrap_err();

    assert!(err.contains("Vault path does not exist"), "got: {err}");
}

#[tokio::test]
async fn init_vault_creates_the_vault_layout() {
    let vault = create_test_vault();
    let fresh = temp_dir();
    let fresh_path = fresh.path().to_path_buf();

    let info = vault
        .service
        .init_vault(fresh_path.to_string_lossy().to_string())
        .await
        .unwrap();

    assert_eq!(info.path, fresh_path.to_string_lossy());
    assert!(fresh_path.join(".pkm").is_dir(), ".pkm must be created");
    assert!(
        fresh_path.join(".pkm").join("blocks.db").exists(),
        "blocks.db must be created"
    );
    assert_eq!(vault.service.vault_path(), fresh_path);
    drop_plugin_runtime(&vault.service);
}

#[tokio::test]
async fn init_vault_at_initializes_an_existing_directory() {
    let vault = create_test_vault();
    let fresh = temp_dir();

    let info = vault
        .service
        .init_vault_at(fresh.path().to_path_buf())
        .await
        .unwrap();

    assert_eq!(info.path, fresh.path().to_string_lossy());
    assert_eq!(info.block_count, 0);
    assert_eq!(info.page_count, 0);
    assert!(fresh.path().join(".pkm").join("blocks.db").exists());
    drop_plugin_runtime(&vault.service);
}
