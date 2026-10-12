use pkm_core::fs_util::MdCollector;
use pkm_core::PkmResult;
use std::path::Path;

/// Export the vault to a file (json or html).
pub(crate) fn cmd_export(vault: &Path, format: &str) -> PkmResult<()> {
    let notes = MdCollector::new().max_depth(8).collect(vault)?;
    match format {
        "json" => {
            let mut exports = Vec::new();
            for path in &notes {
                if let Ok(content) = std::fs::read_to_string(path) {
                    let parsed = pkm_markdown::parser::parse_raw(&content);
                    let rel = path
                        .strip_prefix(vault)
                        .unwrap_or(path)
                        .display()
                        .to_string();
                    exports.push(serde_json::json!({
                        "path": rel,
                        "title": parsed.frontmatter.title,
                        "tags": parsed.frontmatter.tags,
                        "body": parsed.body,
                        "links": parsed.links.iter().map(|l| l.target.clone()).collect::<Vec<_>>(),
                    }));
                }
            }
            let json = serde_json::to_string_pretty(&exports)?;
            let out_path = vault.join("export.json");
            std::fs::write(&out_path, &json)?;
            println!("✓ Exported {} notes to {}", notes.len(), out_path.display());
        }
        _ => {
            // Generate a simple HTML page with all notes
            let mut body = String::new();
            for path in &notes {
                if let Ok(content) = std::fs::read_to_string(path) {
                    let parsed = pkm_markdown::parser::parse_raw(&content);
                    let title = parsed.frontmatter.title.as_deref().unwrap_or("untitled");
                    body.push_str(&format!(
                        "<h1>{}</h1>\n<pre>{}</pre>\n<hr>\n",
                        title,
                        escape_html(&parsed.body)
                    ));
                }
            }
            let html = format!(
                "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>Stratum Export</title>\
                 <style>body{{max-width:800px;margin:0 auto;padding:20px;font-family:system-ui,sans-serif;line-height:1.6}}\
                 pre{{background:#f5f5f5;padding:12px;border-radius:8px;overflow-x:auto}}</style></head><body>\
                 <h1>Stratum Vault Export</h1><p>{} notes</p><hr>{}</body></html>",
                notes.len(), body
            );
            let out_path = vault.join("export.html");
            std::fs::write(&out_path, &html)?;
            println!("✓ Exported {} notes to {}", notes.len(), out_path.display());
        }
    }
    Ok(())
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
