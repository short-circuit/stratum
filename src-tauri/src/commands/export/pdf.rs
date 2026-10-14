//! Per-page PDF export.
//!
//! Renders a single vault page to a PDF by shelling out to a headless
//! Chromium binary. LaTeX is pre-rendered to MathML with `latex2mathml`,
//! Mermaid fenced blocks are rendered by the embedded Mermaid runtime, and
//! wiki-image embeds are rewritten to standard Markdown before rendering.
//! The core function [`export_pdf_core`] is pure (no Tauri) and testable; the
//! `export_pdf` command is the thin Tauri glue.

use crate::commands::vault::AppState;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Embedded Mermaid runtime, compiled in from `node_modules` so the exported
/// HTML can render diagrams offline without a network fetch. `node_modules` is
/// guaranteed present for Tauri builds because `beforeBuildCommand` runs
/// `npm run build` first.
const MERMAID_JS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../node_modules/mermaid/dist/mermaid.min.js"
));

/// Result of a successful per-page PDF export, carrying the output file path.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PdfExportResult {
    pub output_path: String,
    pub page_path: String,
}

/// Core PDF export — plain function so it is testable without a Tauri runtime.
///
/// Reads `vault.join(page_path)`, renders its body to a self-contained HTML
/// document in a temp dir (alongside a copy of the vault `assets/` dir), and
/// shells out to Chromium to print it to `output`. The temp dir is always
/// cleaned up, even if rendering fails.
pub fn export_pdf_core(
    vault: &Path,
    page_path: &str,
    output: &Path,
) -> Result<PdfExportResult, String> {
    let full_path = vault.join(page_path);
    if !full_path.is_file() {
        return Err(format!("page not found: {}", full_path.display()));
    }

    let content = std::fs::read_to_string(&full_path).map_err(|e| e.to_string())?;
    let parsed = pkm_markdown::parser::parse_raw(&content);
    let title = parsed
        .frontmatter
        .title
        .clone()
        .or_else(|| page_stem(page_path))
        .unwrap_or_else(|| "Untitled".to_string());

    let rewritten = rewrite_wiki_images(&parsed.body);
    let html = super::markdown_to_html(&rewritten);
    let html = postprocess_math(&html);
    let html = convert_mermaid_blocks(&html);
    let doc = build_pdf_document(&html, &title);

    let temp = std::env::temp_dir().join(format!("stratum-pdf-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&temp).map_err(|e| e.to_string())?;

    let slug = page_stem(page_path).unwrap_or_else(|| "page".to_string());
    let html_path = temp.join(format!("{}.html", slug));
    std::fs::write(&html_path, &doc).map_err(|e| e.to_string())?;

    let assets_dir = vault.join("assets");
    if assets_dir.exists() {
        super::copy_dir(&assets_dir, &temp.join("assets")).map_err(|e| e.to_string())?;
    }

    let result = render_pdf(&html_path, output);
    let _ = std::fs::remove_dir_all(&temp);
    result?;

    Ok(PdfExportResult {
        output_path: output.to_string_lossy().to_string(),
        page_path: page_path.to_string(),
    })
}

/// Thin Tauri command: exports `page_path` (vault-relative) to the chosen
/// `output_path` when provided and non-empty (parent dir created if missing),
/// otherwise to `<vault>/export-<slug>.pdf`.
#[tauri::command]
pub async fn export_pdf(
    page_path: String,
    output_path: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<PdfExportResult, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let vault = state.vault_path.clone();

    let full_path = vault.join(&page_path);
    if !full_path.is_file() {
        return Err(format!("page not found: {}", full_path.display()));
    }

    let slug = page_stem(&page_path).unwrap_or_else(|| "page".to_string());
    let output = super::resolve_output_path(&vault, output_path, format!("export-{}.pdf", slug))?;
    export_pdf_core(&vault, &page_path, &output)
}

fn page_stem(page_path: &str) -> Option<String> {
    Path::new(page_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
}

/// Rewrite wiki-image embeds (`![[assets/X.png]]`) to standard Markdown image
/// syntax (`![X](assets/X.png)`) so the asset path resolves relative to the
/// exported HTML's temp dir copy. Only used in the PDF pipeline; the HTML
/// export core is left untouched.
fn rewrite_wiki_images(md: &str) -> String {
    let mut out = String::with_capacity(md.len());
    let mut rest = md;
    const OPEN: &str = "![[";
    const CLOSE: &str = "]]";
    loop {
        match rest.find(OPEN) {
            None => {
                out.push_str(rest);
                break;
            }
            Some(i) => {
                out.push_str(&rest[..i]);
                let after = &rest[i + OPEN.len()..];
                match after.find(CLOSE) {
                    Some(j) => {
                        let inner = &after[..j];
                        let name = Path::new(inner)
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("image");
                        out.push_str(&format!("![{}]({})", name, inner));
                        rest = &after[j + CLOSE.len()..];
                    }
                    None => {
                        out.push_str(&rest[i..]);
                        break;
                    }
                }
            }
        }
    }
    out
}

/// Run `latex2mathml::replace` on every non-code (`<pre>`) chunk of the HTML.
/// A chunk whose math fails to render is kept unchanged so a stray literal `$`
/// never aborts the export.
fn postprocess_math(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    let mut in_pre = false;
    loop {
        let marker = if in_pre { "</pre>" } else { "<pre>" };
        match rest.find(marker) {
            Some(i) => {
                if in_pre {
                    let end = i + marker.len();
                    out.push_str(&rest[..end]);
                    rest = &rest[end..];
                    in_pre = false;
                } else {
                    out.push_str(&on_chunk(&rest[..i]));
                    out.push_str(marker);
                    rest = &rest[i + marker.len()..];
                    in_pre = true;
                }
            }
            None => {
                if in_pre {
                    out.push_str(rest);
                } else {
                    out.push_str(&on_chunk(rest));
                }
                break;
            }
        }
    }
    out
}

fn on_chunk(chunk: &str) -> String {
    match latex2mathml::replace(chunk) {
        Ok(rendered) => rendered,
        Err(_) => chunk.to_string(),
    }
}

/// Convert pulldown-rendered Mermaid fenced blocks
/// (`<pre><code class="language-mermaid">…</code></pre>`) into
/// `<pre class="mermaid">…</pre>` so the embedded Mermaid runtime can render
/// them. The inner text stays HTML-escaped — the browser decodes it into
/// `textContent`.
fn convert_mermaid_blocks(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    const OPEN: &str = "<pre><code class=\"language-mermaid\">";
    const CLOSE: &str = "</code></pre>";
    loop {
        match rest.find(OPEN) {
            None => {
                out.push_str(rest);
                break;
            }
            Some(i) => {
                out.push_str(&rest[..i]);
                let after = &rest[i + OPEN.len()..];
                match after.find(CLOSE) {
                    Some(j) => {
                        out.push_str("<pre class=\"mermaid\">");
                        out.push_str(&after[..j]);
                        out.push_str("</pre>");
                        rest = &after[j + CLOSE.len()..];
                    }
                    None => {
                        out.push_str(&rest[i..]);
                        break;
                    }
                }
            }
        }
    }
    out
}

/// Build the self-contained PDF HTML document: reuse `wrap_html` for the shell,
/// then inject the math/mermaid CSS and the Mermaid bootstrap script.
fn build_pdf_document(body: &str, title: &str) -> String {
    let mut doc = super::wrap_html(body, title);
    doc = doc.replace(
        "</style>",
        " math { font-size: 1.05em; } .mermaid { background:#f8f8f8; border:1px solid #ddd; padding:1em; }</style>",
    );
    let script = format!(
        "<script>{}</script>\n<script>mermaid.initialize({{startOnLoad:false, theme:'default'}}); mermaid.run({{querySelector:'.mermaid'}});</script>\n",
        MERMAID_JS
    );
    doc = doc.replace("</body>", &format!("{}\n</body>", script));
    doc
}

/// Print `html_path` to `output` using headless Chromium. On failure the
/// captured stderr is included in the error string.
fn render_pdf(html_path: &Path, output: &Path) -> Result<(), String> {
    let chrom = find_chromium().ok_or_else(|| {
        "chromium not found; install chromium or set STRATUM_CHROMIUM".to_string()
    })?;

    let status = std::process::Command::new(&chrom)
        .arg("--headless=new")
        .arg("--no-sandbox")
        .arg("--disable-gpu")
        .arg("--no-pdf-header-footer")
        .arg("--virtual-time-budget=15000")
        .arg(format!("--print-to-pdf={}", output.display()))
        .arg(html_path)
        .output()
        .map_err(|e| format!("failed to run {}: {}", chrom.display(), e))?;

    if !status.status.success() {
        return Err(format!(
            "chromium failed ({}): {}",
            chrom.display(),
            String::from_utf8_lossy(&status.stderr)
        ));
    }
    Ok(())
}

/// Resolve a usable Chromium binary for headless PDF printing. Returns the
/// first candidate that exists on disk.
pub fn find_chromium() -> Option<PathBuf> {
    chromium_candidates().into_iter().find(|p| p.is_file())
}

/// All candidate Chromium paths, in priority order: `STRATUM_CHROMIUM` env,
/// PATH lookup, common system locations, then Playwright's browser cache.
pub fn chromium_candidates() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();

    if let Ok(env) = std::env::var("STRATUM_CHROMIUM") {
        if !env.is_empty() {
            out.push(PathBuf::from(env));
        }
    }

    if let Ok(path) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path) {
            for name in CHROMIUM_NAMES {
                let p = dir.join(name);
                if p.is_file() {
                    out.push(p);
                }
            }
        }
    }

    for p in SYSTEM_CHROMIUM_PATHS {
        out.push(PathBuf::from(p));
    }

    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        collect_playwright_chromium(&home.join(".cache/ms-playwright"), &mut out);
    }

    let mut seen = std::collections::HashSet::new();
    out.retain(|p| seen.insert(p.clone()));
    out
}

const CHROMIUM_NAMES: [&str; 4] = [
    "chromium",
    "chromium-browser",
    "google-chrome",
    "google-chrome-stable",
];
const SYSTEM_CHROMIUM_PATHS: [&str; 3] = [
    "/usr/bin/chromium",
    "/usr/bin/google-chrome",
    "/usr/bin/google-chrome-stable",
];

/// Scan Playwright's cache for installed Chromium/headless-shell binaries.
fn collect_playwright_chromium(cache: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(cache) else {
        return;
    };
    for entry in entries.flatten() {
        let dir = entry.path();
        let dir_name = dir.file_name().and_then(|s| s.to_str()).unwrap_or_default();
        if !dir_name.starts_with("chromium-") && !dir_name.starts_with("chromium_headless_shell-") {
            continue;
        }
        let Ok(subs) = std::fs::read_dir(&dir) else {
            continue;
        };
        for sub in subs.flatten() {
            let sub_path = sub.path();
            let sub_name = sub_path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            if sub_name.starts_with("chrome-linux") {
                for bin in ["chrome", "headless_shell"] {
                    let b = sub_path.join(bin);
                    if b.is_file() {
                        out.push(b);
                    }
                }
            }
            if sub_name.starts_with("chrome-headless-shell") {
                let b = sub_path.join("chrome-headless-shell-linux64/chrome-headless-shell");
                if b.is_file() {
                    out.push(b);
                }
            }
        }
    }
}
