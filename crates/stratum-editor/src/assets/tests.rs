use pkm_block::Block;
use uuid::Uuid;

use super::*;
use crate::document::EditorDocument;
use crate::inline::SpanKind;
use crate::render::DecoratedSpan;

fn span(kind: SpanKind) -> DecoratedSpan {
    DecoratedSpan {
        text: String::new(),
        kind,
        syntax: false,
        hidden: false,
    }
}

fn thumbnail(width: u32, height: u32) -> AssetThumbnail {
    AssetThumbnail {
        png: Arc::new(vec![0u8]),
        width,
        height,
    }
}

#[test]
fn raster_extensions_are_images() {
    for path in [
        "assets/photo.png",
        "assets/photo.jpg",
        "assets/photo.jpeg",
        "assets/photo.gif",
        "assets/photo.webp",
        "assets/photo.bmp",
        "assets/photo.PNG",
    ] {
        assert_eq!(asset_kind(path), AssetKind::Image, "{path}");
    }
}

#[test]
fn other_extensions_and_missing_extensions_are_files() {
    for path in [
        "assets/doc.pdf",
        "assets/notes.txt",
        // SVG is not decodable by the raster thumbnail pipeline.
        "assets/diagram.svg",
        "assets/README",
        "",
    ] {
        assert_eq!(asset_kind(path), AssetKind::File, "{path}");
    }
}

#[test]
fn format_size_matches_the_react_table() {
    assert_eq!(format_size(0), "0 B");
    assert_eq!(format_size(512), "512 B");
    assert_eq!(format_size(1024), "1.0 KB");
    assert_eq!(format_size(1536), "1.5 KB");
    assert_eq!(format_size(2 * 1024 * 1024), "2.0 MB");
    assert_eq!(format_size(1024 * 1024 * 1024), "1.0 GB");
    // The unit index is capped at GB, matching `formatBytes`.
    assert_eq!(format_size(2048 * 1024 * 1024 * 1024), "2048.0 GB");
}

#[test]
fn asset_reference_wraps_the_path_as_an_embed() {
    assert_eq!(asset_reference("assets/photo.png"), "![[assets/photo.png]]");
    assert_eq!(asset_reference(""), "![[]]");
}

#[test]
fn display_name_is_the_basename() {
    assert_eq!(display_name("assets/photo.png"), "photo.png");
    assert_eq!(display_name("assets/nested/doc.pdf"), "doc.pdf");
    assert_eq!(display_name("photo.png"), "photo.png");
    assert_eq!(display_name(""), "");
}

#[test]
fn first_asset_span_returns_the_first_embed_in_order() {
    let spans = vec![
        span(SpanKind::Text),
        span(SpanKind::Asset {
            path: "assets/one.png".into(),
        }),
        span(SpanKind::Asset {
            path: "assets/two.pdf".into(),
        }),
    ];
    let found = first_asset_span(&spans).expect("asset span");
    assert_eq!(found.path, "assets/one.png");
    assert_eq!(found.kind, AssetKind::Image);
    assert_eq!(first_asset_span(&[span(SpanKind::Text)]), None);
}

#[test]
fn attachment_rows_preserve_order_and_labels() {
    let refs = vec![
        AssetRefInput {
            ref_path: "assets/pic.png".into(),
            mime: "image/png".into(),
            size: 1536,
        },
        AssetRefInput {
            ref_path: "assets/doc.pdf".into(),
            mime: "application/pdf".into(),
            size: 512,
        },
    ];
    let rows = attachment_rows(&refs);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].ref_path, "assets/pic.png");
    assert_eq!(rows[0].name, "pic.png");
    assert_eq!(rows[0].size_label, "1.5 KB");
    assert_eq!(rows[0].kind, AssetKind::Image);
    assert_eq!(rows[1].ref_path, "assets/doc.pdf");
    assert_eq!(rows[1].name, "doc.pdf");
    assert_eq!(rows[1].mime, "application/pdf");
    assert_eq!(rows[1].size_label, "512 B");
    assert_eq!(rows[1].kind, AssetKind::File);
}

#[test]
fn display_size_fits_portrait_images() {
    let (w, h) = thumbnail(100, 200).display_size(THUMBNAIL_BOX.0, THUMBNAIL_BOX.1);
    assert!((w - 90.0).abs() < 0.001);
    assert!((h - 180.0).abs() < 0.001);
}

#[test]
fn display_size_fits_landscape_images() {
    let (w, h) = thumbnail(400, 200).display_size(THUMBNAIL_BOX.0, THUMBNAIL_BOX.1);
    assert!((w - 320.0).abs() < 0.001);
    assert!((h - 160.0).abs() < 0.001);
}

#[test]
fn display_size_never_upscales() {
    let (w, h) = thumbnail(100, 100).display_size(THUMBNAIL_BOX.0, THUMBNAIL_BOX.1);
    assert!((w - 100.0).abs() < 0.001);
    assert!((h - 100.0).abs() < 0.001);
}

#[test]
fn display_size_guards_zero_dimensions() {
    assert_eq!(thumbnail(0, 100).display_size(320.0, 180.0), (0.0, 0.0));
    assert_eq!(thumbnail(100, 0).display_size(320.0, 180.0), (0.0, 0.0));
    assert_eq!(thumbnail(100, 100).display_size(0.0, 180.0), (0.0, 0.0));
    assert_eq!(thumbnail(100, 100).display_size(320.0, 0.0), (0.0, 0.0));
}

#[test]
fn cache_evicts_the_least_recently_used_entry() {
    let mut cache = AssetThumbnailCache::new(2);
    assert_eq!(cache.capacity(), 2);
    assert!(cache.is_empty());
    cache.insert("assets/a.png", thumbnail(10, 10));
    cache.insert("assets/b.png", thumbnail(10, 10));
    assert_eq!(cache.len(), 2);
    // Promote `a`, so `b` becomes the eviction candidate.
    assert!(cache.get("assets/a.png").is_some());
    cache.insert("assets/c.png", thumbnail(10, 10));
    assert_eq!(cache.len(), 2);
    assert!(cache.get("assets/a.png").is_some());
    assert!(cache.get("assets/b.png").is_none());
    assert!(cache.get("assets/c.png").is_some());
}

#[test]
fn cache_reinsert_does_not_grow_past_the_bound() {
    let mut cache = AssetThumbnailCache::new(1);
    cache.insert("assets/a.png", thumbnail(10, 10));
    cache.insert("assets/a.png", thumbnail(20, 20));
    assert_eq!(cache.len(), 1);
    let entry = cache.get("assets/a.png").expect("entry");
    assert_eq!(entry.width, 20);
}

#[test]
fn asset_refs_in_document_dedupes_in_document_order() {
    let blocks = vec![
        Block::new(Uuid::new_v4(), "![[assets/a.png]]".to_string()),
        Block::new(
            Uuid::new_v4(),
            "see ![[assets/b.pdf]] and [[assets/c.pdf]]".to_string(),
        ),
        Block::new(Uuid::new_v4(), "![[assets/a.png]]".to_string()),
        Block::new(Uuid::new_v4(), "plain text".to_string()),
    ];
    let doc = EditorDocument::from_blocks("pages/x.md", blocks).unwrap();
    assert_eq!(
        asset_refs_in_document(&doc),
        vec![
            "assets/a.png".to_string(),
            "assets/b.pdf".to_string(),
            "assets/c.pdf".to_string()
        ]
    );
}

#[test]
fn asset_row_height_uses_the_reserved_constants() {
    assert_eq!(asset_row_height(AssetKind::Image), ASSET_IMAGE_ROW_HEIGHT);
    assert_eq!(asset_row_height(AssetKind::File), ASSET_FILE_ROW_HEIGHT);
}
