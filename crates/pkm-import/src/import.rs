//! Import orchestration: preview and execution.
//!
//! [`preview_import`] computes everything an import would do and returns it as a
//! preview, writing nothing. [`import_vault`] runs the same planning pass and
//! then writes only the items classified `new` (never overwriting a `conflict`
//! or re-copying a `duplicate`). Neither function ever mutates the source
//! directory. All imported content lands under a single `imported/<name>/`
//! subdirectory so the operation is trivially reversible.
//!
//! Idempotency is achieved by hashing (via [`crate::convert::content_hash`]) the
//! content that would be written and comparing it to the on-disk destination
//! file, so re-importing the same vault yields zero new files and everything is
//! reported as a duplicate.

use crate::convert::{content_hash, finalize_output, resolve_format, resolve_subdir};
use crate::report::{
    ImportAsset, ImportFormat, ImportOptions, ImportPreview, ImportPreviewItem, ImportReport,
};
use crate::{generic, logseq, obsidian};
use pkm_core::fs_util::MdCollector;
use pkm_core::{Frontmatter, PkmError, PkmResult};
use std::path::Path;

/// What to write for a single planned file.
enum WriteContent {
    Text(String),
    Bytes(Vec<u8>),
}

/// One file the import plans to create (or skip) at the destination.
struct PlannedWrite {
    source_rel: String,
    rel_path: String,
    status: String,
    size_bytes: u64,
    is_asset: bool,
    title: String,
    content: WriteContent,
}

/// Full internal plan produced by [`plan_import`].
struct ImportPlan {
    preview: ImportPreview,
    writes: Vec<PlannedWrite>,
    errors: Vec<String>,
}

/// Preview what importing `source_path` would do, without writing anything.
pub fn preview_import(source_path: &Path, opts: &ImportOptions) -> PkmResult<ImportPreview> {
    Ok(plan_import(source_path, opts)?.preview)
}

/// Import `source_path` into `opts.destination`, returning a report.
///
/// When `opts.dry_run` is true nothing is written and the report's write counts
/// are zero. The source is never modified, and nothing outside the chosen
/// `subdir` is touched.
pub fn import_vault(source_path: &Path, opts: &ImportOptions) -> PkmResult<ImportReport> {
    let plan = plan_import(source_path, opts)?;

    let mut pages_imported = 0usize;
    let mut assets_imported = 0usize;
    let mut imported_paths: Vec<String> = Vec::new();

    if !opts.dry_run {
        for w in &plan.writes {
            if w.status != "new" {
                continue;
            }
            let dest = opts.destination.join(&w.rel_path);
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent).map_err(PkmError::Io)?;
            }
            match &w.content {
                WriteContent::Text(s) => std::fs::write(&dest, s).map_err(PkmError::Io)?,
                WriteContent::Bytes(b) => std::fs::write(&dest, b).map_err(PkmError::Io)?,
            }
            if w.is_asset {
                assets_imported += 1;
            } else {
                pages_imported += 1;
            }
            imported_paths.push(w.rel_path.clone());
        }
    }

    Ok(ImportReport {
        source_path: plan.preview.source_path.clone(),
        source_name: plan.preview.source_name.clone(),
        format: plan.preview.format,
        pages_imported,
        assets_imported,
        duplicates_skipped: plan.preview.duplicates.len(),
        conflicts_skipped: plan.preview.conflicts.len(),
        dry_run: opts.dry_run,
        errors: plan.errors,
        imported_paths,
    })
}

/// Shared planning pass used by both the preview and the import.
///
/// Scans the source, converts each page per the effective format, and classifies
/// every target as `new` / `duplicate` / `conflict` against the destination.
fn plan_import(source_path: &Path, opts: &ImportOptions) -> PkmResult<ImportPlan> {
    if !source_path.is_dir() {
        return Err(PkmError::NotFound(format!(
            "Source folder not found: {}",
            source_path.display()
        )));
    }

    let format = resolve_format(opts.format, source_path);
    let source_name = source_path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "import".to_string());
    let subdir = resolve_subdir(&opts.subdir, &source_name);

    let mut errors: Vec<String> = Vec::new();
    let mut writes: Vec<PlannedWrite> = Vec::new();
    let mut duplicates: Vec<String> = Vec::new();
    let mut conflicts: Vec<String> = Vec::new();

    // Pages.
    for rel in collect_page_rels(source_path, format)? {
        match plan_page(source_path, format, &subdir, &opts.destination, &rel) {
            Ok(w) => {
                record_skip_status(&w, &mut duplicates, &mut conflicts);
                writes.push(w);
            }
            Err(e) => errors.push(format!("{rel}: {e}")),
        }
    }

    // Assets (non-markdown files under `assets/`). Asset duplicate/conflict
    // status is surfaced through each asset's `status` field and `assets_imported`
    // (0 on re-import); the page-oriented `duplicates`/`conflicts` lists track
    // notes only, matching the report counts the frontend and e2e harness rely on.
    for asset_res in collect_assets(source_path, &subdir) {
        match asset_res {
            Ok((source_rel, rel_path, bytes)) => {
                let status = status_for(&opts.destination, &rel_path, content_hash(&bytes));
                let size = bytes.len() as u64;
                let w = PlannedWrite {
                    source_rel,
                    rel_path: rel_path.clone(),
                    status,
                    size_bytes: size,
                    is_asset: true,
                    title: String::new(),
                    content: WriteContent::Bytes(bytes),
                };
                writes.push(w);
            }
            Err(e) => errors.push(e.to_string()),
        }
    }

    let pages: Vec<ImportPreviewItem> = writes
        .iter()
        .filter(|w| !w.is_asset)
        .map(|w| ImportPreviewItem {
            source_rel: w.source_rel.clone(),
            rel_path: w.rel_path.clone(),
            title: w.title.clone(),
            status: w.status.clone(),
            size_bytes: w.size_bytes,
        })
        .collect();
    let assets: Vec<ImportAsset> = writes
        .iter()
        .filter(|w| w.is_asset)
        .map(|w| ImportAsset {
            source_rel: w.source_rel.clone(),
            rel_path: w.rel_path.clone(),
            status: w.status.clone(),
            size_bytes: w.size_bytes,
        })
        .collect();

    let preview = ImportPreview {
        source_path: source_path.to_string_lossy().to_string(),
        source_name,
        format,
        total_pages: pages.len(),
        total_assets: assets.len(),
        pages,
        assets,
        duplicates,
        conflicts,
    };

    Ok(ImportPlan {
        preview,
        writes,
        errors,
    })
}

/// Convert one source page into its planned destination write.
fn plan_page(
    source_path: &Path,
    format: ImportFormat,
    subdir: &str,
    destination: &Path,
    source_rel: &str,
) -> PkmResult<PlannedWrite> {
    let full = source_path.join(source_rel);
    let source_content = std::fs::read_to_string(&full).map_err(PkmError::Io)?;
    let (frontmatter, output) = convert_page(format, &source_content, source_path, subdir)?;

    let rel_path = format!("{subdir}/{source_rel}");
    let status = page_status(destination, &rel_path, &output);
    let title = frontmatter
        .title
        .clone()
        .unwrap_or_else(|| slug_from_rel(&rel_path));
    let size_bytes = output.len() as u64;

    Ok(PlannedWrite {
        source_rel: source_rel.to_string(),
        rel_path,
        status,
        size_bytes,
        is_asset: false,
        title,
        content: WriteContent::Text(output),
    })
}

/// Convert source content to (frontmatter, final file content) per the format.
fn convert_page(
    format: ImportFormat,
    source_content: &str,
    source_root: &Path,
    subdir: &str,
) -> PkmResult<(Frontmatter, String)> {
    match format {
        ImportFormat::Generic => {
            let (fm, _) = pkm_markdown::parser::parse_frontmatter(source_content);
            Ok((fm, generic::convert(source_content)))
        }
        ImportFormat::Obsidian => {
            let c = obsidian::convert(source_content, source_root, subdir)?;
            let (_, _, blocks) = pkm_markdown::block_parser::parse_document(&c.body);
            let output = finalize_output(&c, &blocks)?;
            Ok((c.frontmatter, output))
        }
        ImportFormat::Logseq => {
            let c = logseq::convert(source_content, source_root, subdir)?;
            let (_, _, blocks) = pkm_markdown::block_parser::parse_document(&c.body);
            let output = finalize_output(&c, &blocks)?;
            Ok((c.frontmatter, output))
        }
        ImportFormat::Auto => unreachable!("format resolved before conversion"),
    }
}

/// Collect the source-relative `.md` paths for the given format.
fn collect_page_rels(source_path: &Path, format: ImportFormat) -> PkmResult<Vec<String>> {
    match format {
        ImportFormat::Logseq => {
            let mut rels = Vec::new();
            for sub in ["pages", "journals"] {
                let dir = source_path.join(sub);
                if dir.is_dir() {
                    let files = MdCollector::new().collect_relative(&dir, source_path)?;
                    rels.extend(files);
                }
            }
            rels.sort();
            Ok(rels)
        }
        _ => MdCollector::new()
            .skip_hidden_dirs(true)
            .collect_relative(source_path, source_path),
    }
}

/// Enumerate non-markdown files under `<source>/assets/**` into
/// `(source_rel, target_rel, bytes)` tuples.
fn collect_assets(source_path: &Path, subdir: &str) -> Vec<PkmResult<(String, String, Vec<u8>)>> {
    let assets_dir = source_path.join("assets");
    let mut out = Vec::new();
    if assets_dir.is_dir() {
        collect_assets_recursive(&assets_dir, &assets_dir, subdir, &mut out);
    }
    out
}

fn collect_assets_recursive(
    dir: &Path,
    assets_dir: &Path,
    subdir: &str,
    out: &mut Vec<PkmResult<(String, String, Vec<u8>)>>,
) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            out.push(Err(PkmError::Io(e)));
            return;
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_assets_recursive(&path, assets_dir, subdir, out);
        } else if path.is_file() {
            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
            if ext == "md" {
                continue;
            }
            if let Ok(rel) = path.strip_prefix(assets_dir) {
                let rel_str = rel.to_string_lossy().to_string();
                let source_rel = format!("assets/{rel_str}");
                let target_rel = format!("{subdir}/assets/{rel_str}");
                match std::fs::read(&path) {
                    Ok(bytes) => out.push(Ok((source_rel, target_rel, bytes))),
                    Err(e) => out.push(Err(PkmError::Io(e))),
                }
            }
        }
    }
}

/// Classify a planned write as a duplicate or conflict and, if so, record its
/// destination path in the corresponding list.
fn record_skip_status(w: &PlannedWrite, duplicates: &mut Vec<String>, conflicts: &mut Vec<String>) {
    match w.status.as_str() {
        "duplicate" => duplicates.push(w.rel_path.clone()),
        "conflict" => conflicts.push(w.rel_path.clone()),
        _ => {}
    }
}

/// Classify whether a page should be `new` / `duplicate` / `conflict`.
///
/// The converted page content includes per-block `.id:` UUIDs, which are
/// regenerated on every conversion, so a naive byte hash would never match a
/// previously imported file and re-imports would report spurious conflicts.
/// Both the prospective output and the on-disk destination are normalized by
/// stripping `.id:` lines before hashing, making the comparison idempotent.
fn page_status(destination: &Path, rel_path: &str, output: &str) -> String {
    let target = destination.join(rel_path);
    if !target.exists() {
        return "new".to_string();
    }
    match std::fs::read_to_string(&target) {
        Ok(existing) => {
            let out_hash = content_hash(normalize_for_dedup(output).as_bytes());
            let existing_hash = content_hash(normalize_for_dedup(&existing).as_bytes());
            if out_hash == existing_hash {
                "duplicate".to_string()
            } else {
                "conflict".to_string()
            }
        }
        Err(_) => "conflict".to_string(),
    }
}

/// Remove auto-generated block `.id:` lines so content hashing is stable across
/// conversions (the UUIDs are not part of the user's content).
fn normalize_for_dedup(content: &str) -> String {
    content
        .lines()
        .filter(|l| !l.trim_start().starts_with(".id:"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Compare the given content hash against the on-disk destination file.
///
/// Used for binary assets, which must be copied byte-for-byte, so a raw content
/// hash is sufficient (there is no regenerated ID to normalize away). Returns
/// `new` when no destination file exists, `duplicate` when the destination
/// already holds byte-identical content, and `conflict` otherwise.
fn status_for(destination: &Path, rel_path: &str, hash: u64) -> String {
    let target = destination.join(rel_path);
    if !target.exists() {
        return "new".to_string();
    }
    match std::fs::read(&target) {
        Ok(bytes) if content_hash(&bytes) == hash => "duplicate".to_string(),
        _ => "conflict".to_string(),
    }
}

/// Derive a slug from a destination-relative path (file stem).
fn slug_from_rel(rel_path: &str) -> String {
    Path::new(rel_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("untitled")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn dry_run_writes_nothing() {
        let src = TempDir::new().unwrap();
        std::fs::write(src.path().join("note.md"), "Hello").unwrap();
        let dest = TempDir::new().unwrap();
        let opts = ImportOptions {
            destination: dest.path().to_path_buf(),
            format: ImportFormat::Generic,
            dry_run: true,
            subdir: None,
        };
        let report = import_vault(src.path(), &opts).unwrap();
        assert_eq!(report.pages_imported, 0);
        assert!(report.imported_paths.is_empty());
        assert_eq!(std::fs::read_dir(dest.path()).unwrap().count(), 0);
    }

    #[test]
    fn source_not_found_is_error() {
        let opts = ImportOptions::default();
        let result = preview_import(Path::new("/nonexistent/path"), &opts);
        assert!(matches!(result, Err(PkmError::NotFound(_))));
    }

    #[test]
    fn import_then_reimport_is_idempotent() {
        let src = TempDir::new().unwrap();
        std::fs::write(src.path().join("a.md"), "A note.").unwrap();
        let dest = TempDir::new().unwrap();
        let opts = ImportOptions {
            destination: dest.path().to_path_buf(),
            format: ImportFormat::Generic,
            dry_run: false,
            subdir: None,
        };
        let r1 = import_vault(src.path(), &opts).unwrap();
        assert_eq!(r1.pages_imported, 1);
        let r2 = import_vault(src.path(), &opts).unwrap();
        assert_eq!(r2.pages_imported, 0);
        assert_eq!(r2.duplicates_skipped, 1);
    }
}
