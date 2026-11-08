//! Headless tests for the asset service commands:
//! `import_asset_path`, `import_asset_bytes`, `read_asset`, `delete_asset`,
//! `list_asset_refs`, `get_page_asset_refs`.

mod common;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use common::{block_dto, create_test_vault};

#[tokio::test]
async fn import_asset_path_registers_the_file() {
    let vault = create_test_vault();
    vault.write_file("photo.txt", "asset payload");

    let asset = vault
        .service
        .import_asset_path("photo.txt".into(), None)
        .await
        .unwrap();

    assert!(
        asset.vault_relative_path.starts_with("assets/"),
        "asset must live under assets/: {}",
        asset.vault_relative_path
    );
    assert!(vault.exists(&asset.vault_relative_path));
    assert!(asset.size > 0);
}

#[tokio::test]
async fn import_asset_bytes_registers_decoded_content() {
    let vault = create_test_vault();
    let encoded = STANDARD.encode(b"binary-ish content");

    let asset = vault
        .service
        .import_asset_bytes(
            encoded,
            "picture.png".into(),
            Some("image/png".into()),
            None,
        )
        .await
        .unwrap();

    assert!(vault.exists(&asset.vault_relative_path));
    assert_eq!(asset.mime, "image/png");
}

#[tokio::test]
async fn import_asset_bytes_rejects_invalid_base64() {
    let vault = create_test_vault();

    let err = vault
        .service
        .import_asset_bytes("!!!not-base64!!!".into(), "x.bin".into(), None, None)
        .await
        .unwrap_err();

    assert!(err.contains("Invalid base64 data"), "got: {err}");
}

#[tokio::test]
async fn read_asset_round_trips_bytes() {
    let vault = create_test_vault();
    let asset = vault
        .service
        .import_asset_bytes(
            STANDARD.encode(b"round trip"),
            "round.bin".into(),
            None,
            None,
        )
        .await
        .unwrap();

    let read = vault
        .service
        .read_asset(asset.vault_relative_path.clone())
        .await
        .unwrap();

    let decoded = STANDARD.decode(read.data_base64).unwrap();
    assert_eq!(decoded, b"round trip");
    assert_eq!(read.size, 10);
}

#[tokio::test]
async fn delete_asset_removes_file_and_record() {
    let vault = create_test_vault();
    let asset = vault
        .service
        .import_asset_bytes(
            STANDARD.encode(b"to delete"),
            "delete.bin".into(),
            None,
            None,
        )
        .await
        .unwrap();

    vault
        .service
        .delete_asset(asset.vault_relative_path.clone())
        .await
        .unwrap();

    assert!(!vault.exists(&asset.vault_relative_path));
    let listed = vault.service.list_asset_refs().await.unwrap();
    assert!(!listed
        .iter()
        .any(|a| a.vault_relative_path == asset.vault_relative_path));
}

#[tokio::test]
async fn list_asset_refs_includes_imported_assets() {
    let vault = create_test_vault();
    let asset = vault
        .service
        .import_asset_bytes(STANDARD.encode(b"listed"), "listed.bin".into(), None, None)
        .await
        .unwrap();

    let listed = vault.service.list_asset_refs().await.unwrap();
    assert!(
        listed
            .iter()
            .any(|a| a.vault_relative_path == asset.vault_relative_path),
        "imported asset must be listed"
    );
}

#[tokio::test]
async fn get_page_asset_refs_resolves_embedded_asset_links() {
    let vault = create_test_vault();
    let asset = vault
        .service
        .import_asset_bytes(
            STANDARD.encode(b"embedded"),
            "embedded.bin".into(),
            None,
            None,
        )
        .await
        .unwrap();

    vault
        .service
        .save_blocks(
            "pages/attached.md".into(),
            vec![block_dto(&format!("![[{}]]", asset.vault_relative_path))],
            None,
        )
        .await
        .unwrap();

    let refs = vault
        .service
        .get_page_asset_refs("pages/attached.md".into())
        .await
        .unwrap();
    assert!(
        refs.iter()
            .any(|r| r.vault_relative_path == asset.vault_relative_path),
        "page asset refs must resolve the embedded asset"
    );
}
