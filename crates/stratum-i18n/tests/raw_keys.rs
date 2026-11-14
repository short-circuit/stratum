//! Raw-key scan gate.
//!
//! Every user-facing string in the native shell goes through the catalogs. This
//! gate walks the shell sources, extracts each literal key passed to
//! `t_global`/`tr_global`, and fails when a key is absent from the English
//! catalog (and therefore from every catalog, which the parity gate keeps in
//! lock-step). A missing key would otherwise render as the raw key string.

use std::fs;
use std::path::{Path, PathBuf};

use stratum_i18n::{key_set, Locale};

/// Every `.rs` file under `dir`, recursively.
fn rust_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_sources(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

/// Extract the first string literal from `rest` (the text after `t_global(`).
fn first_literal(rest: &str) -> Option<String> {
    let rest = rest.trim_start();
    let quote = rest.chars().next()?;
    if quote != '"' {
        return None;
    }
    let body = &rest[1..];
    let end = body.find('"')?;
    Some(body[..end].to_string())
}

/// Collect every statically known catalog key used by `source`.
fn used_keys(source: &str) -> Vec<String> {
    let mut keys = Vec::new();
    for call in ["t_global(", "tr_global("] {
        let mut cursor = 0;
        while let Some(index) = source[cursor..].find(call) {
            let start = cursor + index + call.len();
            if let Some(key) = first_literal(&source[start..]) {
                keys.push(key);
            }
            cursor = start;
        }
    }
    keys
}

#[test]
fn every_key_used_in_the_shell_exists_in_the_catalog() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let shell_src = manifest_dir.join("../stratum-ui/src");
    let mut sources = Vec::new();
    rust_sources(&shell_src, &mut sources);
    assert!(
        sources.len() > 20,
        "expected the shell sources under {}, found {} files",
        shell_src.display(),
        sources.len()
    );

    let english = key_set(Locale::En).expect("English catalog loads");
    let mut missing = Vec::new();
    let mut total = 0;
    for path in sources {
        let Ok(source) = fs::read_to_string(&path) else {
            continue;
        };
        for key in used_keys(&source) {
            total += 1;
            if !english.contains(&key) {
                missing.push(format!("{}: {key}", path.display()));
            }
        }
    }
    missing.sort();
    missing.dedup();
    assert!(total > 100, "raw-key scan found only {total} keys");
    assert!(
        missing.is_empty(),
        "{} shell keys are missing from the catalog: {:?}",
        missing.len(),
        missing
    );
}
