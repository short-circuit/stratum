use std::path::PathBuf;

use crate::store::BlockStore;
use pkm_core::PkmError;
use tempfile::TempDir;

use super::*;

/// Create a temp vault with a BlockStore whose DB lives under `.pkm/`.
fn setup() -> (TempDir, BlockStore, PathBuf) {
    let tmp = TempDir::new().unwrap();
    let vault = tmp.path().to_path_buf();
    std::fs::create_dir_all(vault.join(".pkm")).unwrap();
    let db = vault.join(".pkm").join("blocks.db");
    let store = BlockStore::open(&db).unwrap();
    (tmp, store, vault)
}

#[test]
fn test_register_and_read_round_trip() {
    let (_tmp, store, vault) = setup();
    let bytes = b"hello asset".to_vec();
    let asset = register_asset_bytes(&store, &vault, &bytes, "foo.png", None, None).unwrap();
    // Deterministic ref mirrors the hash prefix.
    assert_eq!(
        asset.vault_relative_path,
        format!("assets/foo-{}.png", &asset.sha256[..12])
    );
    assert!(vault.join(&asset.vault_relative_path).exists());
    let rec = store
        .get_asset_record(&asset.vault_relative_path)
        .unwrap()
        .unwrap();
    assert_eq!(rec.mime, "image/png");
    assert_eq!(rec.size, bytes.len() as i64);
    // read_asset returns identical bytes + mime.
    let (read, mime) = read_asset(&store, &vault, &asset.vault_relative_path).unwrap();
    assert_eq!(read, bytes);
    assert_eq!(mime, "image/png");
}

#[test]
fn test_dedupe_same_bytes_same_name() {
    let (_tmp, store, vault) = setup();
    let bytes = b"dup content".to_vec();
    let a = register_asset_bytes(&store, &vault, &bytes, "a.png", None, None).unwrap();
    let b = register_asset_bytes(&store, &vault, &bytes, "a.png", None, None).unwrap();
    // Same ref, only one record and one file.
    assert_eq!(a.vault_relative_path, b.vault_relative_path);
    assert_eq!(store.list_asset_records().unwrap().len(), 1);
    let files: Vec<_> = std::fs::read_dir(vault.join("assets"))
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| !e.file_name().to_string_lossy().starts_with(".tmp-"))
        .collect();
    assert_eq!(files.len(), 1);
}

#[test]
fn test_collision_different_bytes_same_stem() {
    let (_tmp, store, vault) = setup();
    let a = register_asset_bytes(&store, &vault, b"content-one", "a.png", None, None).unwrap();
    let b = register_asset_bytes(&store, &vault, b"content-two", "a.png", None, None).unwrap();
    // Distinct hashes => distinct refs, both files exist, both under assets/.
    assert_ne!(a.vault_relative_path, b.vault_relative_path);
    assert!(a.vault_relative_path.starts_with("assets/a-"));
    assert!(b.vault_relative_path.starts_with("assets/a-"));
    assert!(a.vault_relative_path.starts_with("assets/"));
    assert!(b.vault_relative_path.starts_with("assets/"));
    assert!(vault.join(&a.vault_relative_path).exists());
    assert!(vault.join(&b.vault_relative_path).exists());
}

#[test]
fn test_deterministic_ref_path() {
    let (_tmp, store, vault) = setup();
    let bytes = b"deterministic".to_vec();
    let a = register_asset_bytes(&store, &vault, &bytes, "det.png", None, None).unwrap();
    // Registering the same bytes/name yields an identical ref (hash-based).
    let b = register_asset_bytes(&store, &vault, &bytes, "det.png", None, None).unwrap();
    assert_eq!(a.vault_relative_path, b.vault_relative_path);
    assert_eq!(
        a.vault_relative_path,
        format!("assets/det-{}.png", &a.sha256[..12])
    );
}

#[test]
fn test_is_image_mime() {
    // Positive: every accepted raster/icon MIME, including the legacy jpg alias.
    for mime in [
        "image/png",
        "image/jpeg",
        "image/jpg",
        "image/gif",
        "image/webp",
        "image/bmp",
        "image/x-icon",
        "image/vnd.microsoft.icon",
    ] {
        assert!(is_image_mime(mime), "expected image: {mime}");
    }
    // Case-insensitive and whitespace-trimmed.
    assert!(is_image_mime("  IMAGE/PNG "));
    assert!(is_image_mime("Image/Jpeg"));
    // Negative: non-image MIME, SVG (raster-only helper) and empty input.
    assert!(!is_image_mime("application/pdf"));
    assert!(!is_image_mime("audio/mpeg"));
    assert!(!is_image_mime("image/svg+xml"));
    assert!(!is_image_mime("image/avif"));
    assert!(!is_image_mime(""));
}

#[test]
fn test_is_image_asset_path() {
    // Positive: raster extensions, uppercase extension, nested asset path.
    assert!(is_image_asset_path("assets/photo.png"));
    assert!(is_image_asset_path("assets/photo.PNG"));
    assert!(is_image_asset_path("assets/photo.jpeg"));
    assert!(is_image_asset_path("assets/photo.webp"));
    assert!(is_image_asset_path("assets/sub/icon.ico"));
    // SVG is classified as an image by path even though it is not raster-decodable.
    assert!(is_image_asset_path("assets/diagram.svg"));
    // Negative: non-image extension, unknown extension, no extension.
    assert!(!is_image_asset_path("assets/doc.pdf"));
    assert!(!is_image_asset_path("assets/audio.mp3"));
    assert!(!is_image_asset_path("assets/unknown.xyz"));
    assert!(!is_image_asset_path("assets/README"));
    assert!(!is_image_asset_path(""));
}

#[test]
fn test_mime_inference() {
    assert_eq!(mime_from_extension("png"), Some("image/png"));
    assert_eq!(mime_from_extension(".PNG"), Some("image/png"));
    assert_eq!(mime_from_extension("xyz"), None);
    assert_eq!(mime_for("a.png", None), "image/png");
    assert_eq!(mime_for("a.unknown", None), "application/octet-stream");
    // Provided MIME wins.
    assert_eq!(mime_for("a.png", Some("text/plain")), "text/plain");
    // Empty provided MIME falls back to inference.
    assert_eq!(mime_for("a.png", Some("")), "image/png");
}

#[test]
fn test_delete_asset() {
    let (_tmp, store, vault) = setup();
    let asset = register_asset_bytes(&store, &vault, b"delete me", "d.png", None, None).unwrap();
    assert!(vault.join(&asset.vault_relative_path).exists());
    delete_asset(&store, &vault, &asset.vault_relative_path).unwrap();
    assert!(!vault.join(&asset.vault_relative_path).exists());
    assert!(store
        .get_asset_record(&asset.vault_relative_path)
        .unwrap()
        .is_none());
    // Deleting a nonexistent ref is an error.
    let err = delete_asset(&store, &vault, "assets/nope.png").unwrap_err();
    assert!(matches!(err, PkmError::NotFound(_)));
    // Reading a nonexistent ref is an error.
    let err = read_asset(&store, &vault, "assets/nope.png").unwrap_err();
    assert!(matches!(err, PkmError::NotFound(_)));
}

#[test]
fn test_path_validation() {
    let (_tmp, store, vault) = setup();
    let asset = register_asset_bytes(&store, &vault, b"x", "v.png", None, None).unwrap();
    assert!(read_asset(&store, &vault, &asset.vault_relative_path).is_ok());
    // Non-asset prefix rejected.
    let err = read_asset(&store, &vault, "pages/x.png").unwrap_err();
    assert!(matches!(err, PkmError::Validation(_)));
    // `..` escaping the assets dir rejected.
    let err = read_asset(&store, &vault, "assets/../x.png").unwrap_err();
    assert!(matches!(err, PkmError::Validation(_)));
    // Nested path under assets/ is valid (file missing -> NotFound, not Validation).
    let err = read_asset(&store, &vault, "assets/sub/x.png").unwrap_err();
    assert!(matches!(err, PkmError::NotFound(_)));
}

#[test]
fn test_get_asset_refs_for_page() {
    let (_tmp, store, _vault) = setup();
    let id = uuid::Uuid::new_v4();
    store
        .insert_block(
            &crate::block::Block::new(id, "see [[assets/foo.png]]".into()),
            "pages/p.md",
        )
        .unwrap();
    store
        .insert_link(id, "asset_ref", Some("assets/foo.png"), None)
        .unwrap();
    store
        .insert_link(id, "page_ref", Some("pages/other.md"), None)
        .unwrap();
    let refs = store.get_asset_refs_for_page("pages/p.md").unwrap();
    assert_eq!(refs, vec!["assets/foo.png".to_string()]);
    // A page with no assets yields an empty list.
    let empty = store.get_asset_refs_for_page("pages/q.md").unwrap();
    assert!(empty.is_empty());
}

#[test]
fn test_sanitize_stem() {
    assert_eq!(sanitize_stem("Hello World"), "hello-world");
    assert_eq!(sanitize_stem("  Foo__Bar  "), "foo__bar");
    assert_eq!(sanitize_stem("a/b\\c"), "a-b-c");
    assert_eq!(sanitize_stem("!!!"), "asset");
    assert_eq!(sanitize_stem(""), "asset");
    // Trailing dash collapsed.
    assert_eq!(sanitize_stem("name-"), "name");
}
