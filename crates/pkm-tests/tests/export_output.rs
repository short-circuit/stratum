mod common;

use common::create_test_vault;
use std::fs;

#[test]
fn test_export_html_creates_files() {
    let tv = create_test_vault();

    // Create a page
    tv.add_page("pages/test.md");
    tv.add_block("pages/test.md", "# Exported Page");
    tv.add_block("pages/test.md", "Content for export.");

    // Export to a temp dir
    let export_dir = tv.vault_path.join("export");
    fs::create_dir_all(&export_dir).unwrap();

    // Build basic HTML export
    let pages = tv.store.list_pages().unwrap();
    for page_path in &pages {
        let blocks = tv.store.get_blocks_by_page(page_path).unwrap();
        let slug = std::path::Path::new(page_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("page");

        let html = format!("<!DOCTYPE html><html><head><title>{}</title></head><body>{}</body></html>",
            slug,
            blocks.iter().map(|b| format!("<p>{}</p>", b.content)).collect::<Vec<_>>().join("\n")
        );

        let out_path = export_dir.join(format!("{}.html", slug));
        fs::write(&out_path, &html).unwrap();
        assert!(out_path.exists());
    }

    // Verify export produced output
    let export_count = fs::read_dir(&export_dir).unwrap().count();
    assert_eq!(export_count, 1, "should have exported 1 HTML file");
}

#[test]
fn test_export_json_creates_files() {
    let tv = create_test_vault();

    tv.add_page("pages/test.md");
    tv.add_block("pages/test.md", "JSON export test");

    let export_dir = tv.vault_path.join("export-json");
    fs::create_dir_all(&export_dir).unwrap();

    // Build simple JSON export
    let pages = tv.store.list_pages().unwrap();
    let json = serde_json::json!({
        "exported_at": chrono::Utc::now().to_rfc3339(),
        "pages": pages.iter().map(|p| {
            let blocks = tv.store.get_blocks_by_page(p).unwrap();
            serde_json::json!({
                "path": p,
                "blocks": blocks.iter().map(|b| serde_json::json!({
                    "id": b.id.to_string(),
                    "content": b.content,
                })).collect::<Vec<_>>(),
            })
        }).collect::<Vec<_>>(),
    });

    let out_path = export_dir.join("stratum-export.json");
    fs::write(&out_path, serde_json::to_string_pretty(&json).unwrap()).unwrap();
    assert!(out_path.exists());

    // Verify JSON is valid
    let content = fs::read_to_string(&out_path).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(parsed.get("pages").is_some());
}
