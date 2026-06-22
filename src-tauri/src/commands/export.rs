//! Static site generation and export commands.

use crate::commands::vault::AppState;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExportResult {
    pub output_dir: String,
    pub pages_exported: usize,
    pub assets_copied: usize,
}

#[tauri::command]
pub async fn export_html(
    output_dir: String,
    state: tauri::State<'_, AppState>,
) -> Result<ExportResult, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let vault = &state.vault_path;
    let output = std::path::PathBuf::from(&output_dir);

    std::fs::create_dir_all(&output).map_err(|e| e.to_string())?;

    let store = pkm_block::BlockStore::open(&state.db_path).map_err(|e| e.to_string())?;
    let pages = store.list_pages().map_err(|e| e.to_string())?;

    let mut exported = 0usize;

    for page_path in &pages {
        let full_path = vault.join(page_path);
        if !full_path.exists() {
            continue;
        }

        let content = std::fs::read_to_string(&full_path).map_err(|e| e.to_string())?;
        let html = markdown_to_html(&content);

        // Mirror directory structure
        let rel_dir = std::path::Path::new(page_path)
            .parent()
            .unwrap_or(std::path::Path::new(""));
        let slug = std::path::Path::new(page_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("index");

        let out_dir = output.join(rel_dir);
        std::fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;

        let html_path = out_dir.join(format!("{}.html", slug));
        let full_html = wrap_html(&html, &slug.replace('-', " "));
        std::fs::write(&html_path, &full_html).map_err(|e| e.to_string())?;
        exported += 1;
    }

    // Generate index.html linking to all pages
    let mut index = String::from(
        "<!DOCTYPE html>\n<html><head><meta charset=\"utf-8\"><title>Stratum Vault</title>",
    );
    index.push_str("<style>body{font-family:system-ui;max-width:800px;margin:0 auto;padding:2em;background:#fff;color:#111} a{color:#36c;text-decoration:none} a:hover{text-decoration:underline} ul{list-style:none;padding:0} li{margin:.5em 0} .meta{font-size:.85em;color:#666}</style>");
    index.push_str("</head><body><h1>Stratum Vault</h1><ul>");

    for page_path in &pages {
        let slug = std::path::Path::new(page_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("untitled");
        let name = slug.replace('-', " ");
        let rel_dir = std::path::Path::new(page_path)
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or("");
        let href = if rel_dir.is_empty() {
            format!("{}.html", slug)
        } else {
            format!("{}/{}.html", rel_dir, slug)
        };
        index.push_str(&format!(
            "<li><a href=\"{}\"><strong>{}</strong></a> <span class=\"meta\">{}</span></li>",
            href, name, page_path
        ));
    }

    index.push_str("</ul></body></html>");
    std::fs::write(output.join("index.html"), &index).map_err(|e| e.to_string())?;

    // Copy assets directory if it exists
    let assets_dir = vault.join("assets");
    let mut assets_copied = 0usize;
    if assets_dir.exists() {
        let output_assets = output.join("assets");
        copy_dir(&assets_dir, &output_assets).map_err(|e| e.to_string())?;
        assets_copied = std::fs::read_dir(&output_assets)
            .map(|d| d.count())
            .unwrap_or(0);
    }

    Ok(ExportResult {
        output_dir: output_dir,
        pages_exported: exported,
        assets_copied,
    })
}

#[tauri::command]
pub async fn export_json(
    output_dir: String,
    state: tauri::State<'_, AppState>,
) -> Result<ExportResult, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let vault = &state.vault_path;
    let output = std::path::PathBuf::from(&output_dir);
    std::fs::create_dir_all(&output).map_err(|e| e.to_string())?;

    let store = pkm_block::BlockStore::open(&state.db_path).map_err(|e| e.to_string())?;
    let pages = store.list_pages().map_err(|e| e.to_string())?;

    let mut exported = 0usize;
    for page_path in &pages {
        let full_path = vault.join(page_path);
        if !full_path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&full_path).map_err(|e| e.to_string())?;
        let (fm, body, blocks) = pkm_markdown::block_parser::parse_document(&content);

        let json_val = serde_json::json!({
            "path": page_path,
            "title": fm.title,
            "tags": fm.tags,
            "body": body,
            "blocks": blocks.iter().map(|b| serde_json::json!({
                "id": b.id.to_string(),
                "content": b.content,
                "marker": b.marker.map(|m| m.as_str()),
                "priority": b.priority.map(|p| p.as_str()),
            })).collect::<Vec<_>>(),
        });

        let json_path = output.join(page_path).with_extension("json");
        if let Some(parent) = json_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(
            &json_path,
            serde_json::to_string_pretty(&json_val).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        exported += 1;
    }

    Ok(ExportResult {
        output_dir,
        pages_exported: exported,
        assets_copied: 0,
    })
}

fn markdown_to_html(md: &str) -> String {
    let parser = pulldown_cmark::Parser::new_ext(md, pulldown_cmark::Options::all());
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, parser);
    html
}

fn wrap_html(body: &str, title: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<style>
body{{font-family:system-ui;max-width:800px;margin:0 auto;padding:2em;background:#fff;color:#111}}
pre,code{{background:#f5f5f5;border-radius:4px;padding:.2em .4em}}
pre{{padding:1em;overflow-x:auto}}
pre code{{padding:0;background:none}}
blockquote{{border-left:3px solid #ddd;margin-left:0;padding-left:1em;color:#666}}
img{{max-width:100%}}
a{{color:#36c}}
</style>
</head>
<body>
{body}
</body>
</html>"#,
        title = title,
        body = body,
    )
}

fn copy_dir(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    if !dst.exists() {
        std::fs::create_dir_all(dst)?;
    }
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir(&entry.path(), &dest)?;
        } else {
            std::fs::copy(entry.path(), &dest)?;
        }
    }
    Ok(())
}
