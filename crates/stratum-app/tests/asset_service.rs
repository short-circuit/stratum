//! Headless tests for the asset service commands:
//! `import_asset_path`, `import_asset_bytes`, `read_asset`, `delete_asset`,
//! `list_asset_refs`, `get_page_asset_refs`, plus the checked upload/thumbnail
//! helpers (`import_asset_bytes_checked`, `import_asset_path_checked`,
//! `asset_thumbnail`).

mod common;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use common::{block_dto, create_test_vault};
use image::{ImageBuffer, Rgba, RgbaImage};
use stratum_app::asset::ASSET_MAX_BYTES;

/// Encode an RGBA image as PNG bytes.
fn png_bytes(image: &RgbaImage) -> Vec<u8> {
    let mut out = Vec::new();
    image
        .write_with_encoder(image::codecs::png::PngEncoder::new(&mut out))
        .unwrap();
    out
}

/// Build a deterministic RGBA test image filled by `pixel`.
fn image_with(width: u32, height: u32, pixel: impl Fn(u32, u32) -> [u8; 4]) -> RgbaImage {
    ImageBuffer::<Rgba<u8>, _>::from_fn(width, height, |x, y| Rgba(pixel(x, y)))
}

/// Deterministic pseudo-random byte so a PNG fixture cannot be compressed.
fn noise(x: u32, y: u32) -> u8 {
    let mut v = x.wrapping_mul(0x9E37_79B1) ^ y.wrapping_mul(0x85EB_CA77);
    v ^= v >> 13;
    v = v.wrapping_mul(0xC2B2_AE3D);
    v ^= v >> 16;
    v as u8
}

#[tokio::test]
async fn asset_thumbnail_scales_raster_image() {
    let vault = create_test_vault();
    let source = png_bytes(&image_with(64, 32, |x, y| [x as u8, y as u8, 128, 255]));

    let asset = vault
        .service
        .import_asset_bytes_checked(
            STANDARD.encode(&source),
            "thumb.png".into(),
            Some("image/png".into()),
            None,
        )
        .await
        .unwrap();

    let thumb = vault
        .service
        .asset_thumbnail(asset.vault_relative_path.clone(), 16)
        .await
        .unwrap();

    assert_eq!(thumb.mime, "image/png");
    assert_eq!((thumb.width, thumb.height), (16, 8));
    let decoded_bytes = STANDARD.decode(thumb.data_base64).unwrap();
    assert_eq!(thumb.size, decoded_bytes.len() as u64);
    let decoded = image::load_from_memory(&decoded_bytes).unwrap();
    assert_eq!((decoded.width(), decoded.height()), (16, 8));
}

#[tokio::test]
async fn asset_thumbnail_rejects_non_image() {
    let vault = create_test_vault();
    let asset = vault
        .service
        .import_asset_bytes_checked(
            STANDARD.encode(b"plain text"),
            "notes.txt".into(),
            None,
            None,
        )
        .await
        .unwrap();

    let err = vault
        .service
        .asset_thumbnail(asset.vault_relative_path, 64)
        .await
        .unwrap_err();
    assert_eq!(err, "not_an_image");
}

#[tokio::test]
async fn checked_import_compresses_oversized_image() {
    let vault = create_test_vault();
    let source = png_bytes(&image_with(2200, 2200, |x, y| {
        [noise(x, y), noise(x + 1, y), noise(x, y + 1), 255]
    }));
    assert!(
        source.len() > ASSET_MAX_BYTES,
        "fixture PNG must exceed the upload limit: {} bytes",
        source.len()
    );

    let asset = vault
        .service
        .import_asset_bytes_checked(
            STANDARD.encode(&source),
            "huge.png".into(),
            Some("image/png".into()),
            None,
        )
        .await
        .unwrap();

    assert_eq!(asset.mime, "image/jpeg");
    assert!(asset.size <= ASSET_MAX_BYTES as u64);
    assert!(vault.exists(&asset.vault_relative_path));
}

#[tokio::test]
async fn checked_import_rejects_oversized_non_image() {
    let vault = create_test_vault();
    let payload = vec![0u8; ASSET_MAX_BYTES + 1];

    let err = vault
        .service
        .import_asset_bytes_checked(STANDARD.encode(&payload), "huge.bin".into(), None, None)
        .await
        .unwrap_err();

    assert_eq!(err, "file_too_large");
    assert!(
        vault.service.list_asset_refs().await.unwrap().is_empty(),
        "a rejected upload must not register an asset"
    );
}

#[tokio::test]
async fn checked_path_import_dedupes_repeated_imports() {
    let vault = create_test_vault();
    let source = png_bytes(&image_with(8, 8, |x, y| [x as u8, y as u8, 0, 255]));
    std::fs::write(vault.vault_path.join("photo.png"), &source).unwrap();

    let first = vault
        .service
        .import_asset_path_checked("photo.png".into(), None)
        .await
        .unwrap();
    let second = vault
        .service
        .import_asset_path_checked("photo.png".into(), None)
        .await
        .unwrap();

    assert!(first.vault_relative_path.starts_with("assets/"));
    assert_eq!(first.vault_relative_path, second.vault_relative_path);
    assert_eq!(first.sha256, second.sha256);
}

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
