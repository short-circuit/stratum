//! Integration tests for the `pkm-import` crate.
//!
//! These verify the end-to-end importer behavior against the committed fixture
//! vaults in `tests/fixtures/`, covering the spec's acceptance scenarios:
//! format detection, preview counts and link rewriting, on-disk writes and
//! structural conversion, idempotent re-imports, dry-run behavior, conflict
//! handling, generic passthrough, and the guarantee that the source vault is
//! never mutated.

use pkm_import::convert::content_hash;
use pkm_import::{detect_format, import_vault, preview_import, ImportFormat, ImportOptions};
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn opts(dest: &Path, format: ImportFormat, dry_run: bool) -> ImportOptions {
    ImportOptions {
        destination: dest.to_path_buf(),
        format,
        dry_run,
        subdir: None,
    }
}

/// Recursively list all files under `dir` as (relative path, byte size).
fn snapshot_files(dir: &Path) -> Vec<(String, u64)> {
    let mut out = Vec::new();
    fn walk(dir: &Path, base: &Path, out: &mut Vec<(String, u64)>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, base, out);
                } else if let Ok(meta) = std::fs::metadata(&path) {
                    if let Ok(rel) = path.strip_prefix(base) {
                        out.push((rel.to_string_lossy().to_string(), meta.len()));
                    }
                }
            }
        }
    }
    walk(dir, dir, &mut out);
    out.sort();
    out
}

fn read(dest: &Path, rel: &str) -> String {
    std::fs::read_to_string(dest.join(rel)).expect("written file should exist")
}

#[test]
fn t1_detect_format_per_heuristics() {
    assert_eq!(
        detect_format(&fixture("obsidian-sample")),
        ImportFormat::Obsidian
    );
    assert_eq!(
        detect_format(&fixture("logseq-sample")),
        ImportFormat::Logseq
    );
    assert_eq!(
        detect_format(&fixture("generic-sample")),
        ImportFormat::Generic
    );
}

#[test]
fn t2_obsidian_preview_counts_and_rewrites() {
    let dest = tempfile::tempdir().unwrap();
    let src = fixture("obsidian-sample");
    let p = preview_import(&src, &opts(dest.path(), ImportFormat::Auto, true)).unwrap();
    assert_eq!(p.format, ImportFormat::Obsidian);
    assert_eq!(p.total_pages, 3);
    assert_eq!(p.total_assets, 1);
    for page in &p.pages {
        assert!(
            page.rel_path.starts_with("imported/obsidian-sample/"),
            "unexpected rel_path {}",
            page.rel_path
        );
    }

    // Content rewriting is observable only after a real write.
    let r = import_vault(&src, &opts(dest.path(), ImportFormat::Auto, false)).unwrap();
    assert_eq!(r.pages_imported, 3);
    let welcome = read(dest.path(), "imported/obsidian-sample/Welcome.md");
    assert!(welcome.contains("[[Projects]]"), "wiki-link not preserved");
    assert!(
        welcome.contains("![[imported/obsidian-sample/assets/logo.png]]"),
        "embed not rewritten"
    );
}

#[test]
fn t3_obsidian_import_writes_structured_blocks_and_leaves_source_alone() {
    let dest = tempfile::tempdir().unwrap();
    let src = fixture("obsidian-sample");
    let before = snapshot_files(&src);

    let r = import_vault(&src, &opts(dest.path(), ImportFormat::Auto, false)).unwrap();
    assert_eq!(r.pages_imported, 3);
    assert_eq!(r.assets_imported, 1);

    // Project page: heading_level 1 and callout property.
    let projects = read(dest.path(), "imported/obsidian-sample/Projects.md");
    let (_, _, blocks) = pkm_markdown::block_parser::parse_document(&projects);
    assert!(
        blocks.iter().any(|b| b.meta.heading_level == Some(1)),
        "Projects page should have a heading_level 1 block"
    );
    assert!(
        blocks
            .iter()
            .any(|b| b.properties.get("callout").map(String::as_str) == Some("tip")),
        "Projects page should have a callout block with callout=tip"
    );

    // Reading page: TODO and DONE markers.
    let reading = read(dest.path(), "imported/obsidian-sample/Reading.md");
    let (_, _, blocks) = pkm_markdown::block_parser::parse_document(&reading);
    assert!(
        blocks
            .iter()
            .any(|b| b.marker == Some(pkm_block::TaskMarker::Todo)),
        "Reading page should have a TODO marker"
    );
    assert!(
        blocks
            .iter()
            .any(|b| b.marker == Some(pkm_block::TaskMarker::Done)),
        "Reading page should have a DONE marker"
    );

    // Source must be untouched.
    assert_eq!(snapshot_files(&src), before);
}

#[test]
fn t4_reimport_is_idempotent() {
    let dest = tempfile::tempdir().unwrap();
    let src = fixture("obsidian-sample");
    let first = import_vault(&src, &opts(dest.path(), ImportFormat::Auto, false)).unwrap();
    assert_eq!(first.pages_imported, 3);
    assert_eq!(first.assets_imported, 1);

    let second = import_vault(&src, &opts(dest.path(), ImportFormat::Auto, false)).unwrap();
    assert_eq!(second.pages_imported, 0);
    assert_eq!(second.assets_imported, 0);
    assert_eq!(second.duplicates_skipped, 3);
    assert_eq!(second.conflicts_skipped, 0);
}

#[test]
fn t5_dry_run_writes_nothing() {
    let dest = tempfile::tempdir().unwrap();
    let src = fixture("obsidian-sample");
    let r = import_vault(&src, &opts(dest.path(), ImportFormat::Auto, true)).unwrap();
    assert!(r.imported_paths.is_empty());
    assert_eq!(std::fs::read_dir(dest.path()).unwrap().count(), 0);
}

#[test]
fn t6_conflict_is_never_overwritten() {
    let dest = tempfile::tempdir().unwrap();
    let src = fixture("obsidian-sample");
    let target = dest.path().join("imported/obsidian-sample/Welcome.md");
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    std::fs::write(&target, "# A totally different note\n").unwrap();

    let r = import_vault(&src, &opts(dest.path(), ImportFormat::Auto, false)).unwrap();
    assert_eq!(r.conflicts_skipped, 1);
    // The pre-existing file is left untouched (not overwritten).
    assert_eq!(
        std::fs::read_to_string(&target).unwrap(),
        "# A totally different note\n"
    );
    // The conflict is surfaced in the preview's conflicts list.
    let p = preview_import(&src, &opts(dest.path(), ImportFormat::Auto, true)).unwrap();
    assert!(p.conflicts.iter().any(|c| c.ends_with("Welcome.md")));
}

#[test]
fn t7_logseq_conversion_and_tags() {
    let dest = tempfile::tempdir().unwrap();
    let src = fixture("logseq-sample");
    let p = preview_import(&src, &opts(dest.path(), ImportFormat::Auto, true)).unwrap();
    assert_eq!(p.total_pages, 2);

    import_vault(&src, &opts(dest.path(), ImportFormat::Auto, false)).unwrap();

    let notes = read(dest.path(), "imported/logseq-sample/pages/reading-notes.md");
    let (_, _, blocks) = pkm_markdown::block_parser::parse_document(&notes);
    assert!(
        blocks.iter().any(
            |b| b.content.contains("fix typo") && b.marker == Some(pkm_block::TaskMarker::Todo)
        ),
        "reading-notes should have a TODO-marked 'fix typo' block"
    );
    assert!(
        blocks
            .iter()
            .any(|b| b.properties.get("key").map(String::as_str) == Some("value")),
        "reading-notes should preserve key=value property"
    );
    assert!(
        blocks
            .iter()
            .any(|b| b.properties.get("other-key").map(String::as_str) == Some("b")),
        "reading-notes should preserve other-key=b property"
    );
    assert!(
        blocks.iter().any(|b| b
            .content
            .contains("((00000000-0000-0000-0000-000000000001))")),
        "reading-notes should preserve block ref"
    );

    let journal = read(dest.path(), "imported/logseq-sample/journals/2026-01-01.md");
    let (fm, _, _) = pkm_markdown::block_parser::parse_document(&journal);
    assert!(
        fm.tags.iter().any(|t| t == "alpha"),
        "journal frontmatter should carry the alpha tag"
    );
}

#[test]
fn t8_generic_is_byte_identical_and_preserves_subfolders() {
    let dest = tempfile::tempdir().unwrap();
    let src = fixture("generic-sample");
    import_vault(&src, &opts(dest.path(), ImportFormat::Generic, false)).unwrap();

    for (rel, _) in snapshot_files(&src) {
        let written =
            std::fs::read(dest.path().join("imported/generic-sample").join(&rel)).unwrap();
        let source = std::fs::read(src.join(&rel)).unwrap();
        assert_eq!(
            written, source,
            "generic file {} should be byte-identical",
            rel
        );
    }
}

#[test]
fn t9_import_never_mutates_source() {
    let dests = vec![
        tempfile::tempdir().unwrap(),
        tempfile::tempdir().unwrap(),
        tempfile::tempdir().unwrap(),
    ];
    let fixtures = ["obsidian-sample", "logseq-sample", "generic-sample"];
    for (i, name) in fixtures.iter().enumerate() {
        let src = fixture(name);
        let before = snapshot_files(&src);
        let before_hashes: Vec<(String, u64)> = before
            .iter()
            .map(|(rel, _)| {
                (
                    rel.clone(),
                    content_hash(&std::fs::read(src.join(rel)).unwrap()),
                )
            })
            .collect();
        import_vault(&src, &opts(dests[i].path(), ImportFormat::Auto, false)).unwrap();
        let after: Vec<(String, u64)> = snapshot_files(&src)
            .iter()
            .map(|(rel, _)| {
                (
                    rel.clone(),
                    content_hash(&std::fs::read(src.join(rel)).unwrap()),
                )
            })
            .collect();
        assert_eq!(before_hashes, after, "source {} mutated by import", name);
    }
}
