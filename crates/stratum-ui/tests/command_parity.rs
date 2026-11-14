//! Differential parity between the shipped IPC command surface and the native
//! shell.
//!
//! The web frontend was a thin adapter over [`stratum_app::StratumService`], so
//! parity is structural: every IPC command in `docs/development/service-layer.md`
//! must resolve to a public service method that still exists in
//! `crates/stratum-app/src`. This test parses that shipped inventory and checks
//! the implementation, which is what makes "no unimplemented feature" checkable
//! after the web shell is deleted.
//!
//! The second test drives the same fixture vault through the service and through
//! the native provider seam and asserts the two views agree row for row, so the
//! native path cannot drift from the service contract.

use std::fs;
use std::path::{Path, PathBuf};

use stratum_ui::provider::provider_from_env;

/// The shipped command inventory size (the table lists 128 rows: 126 IPC
/// commands plus the two platform-only pickers).
const SHIPPED_COMMANDS: usize = 128;

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

/// Extract the backticked identifier from a table cell.
fn cell_ident(cell: &str) -> Option<String> {
    let start = cell.find('`')? + 1;
    let end = cell[start..].find('`')? + start;
    let raw = &cell[start..end];
    // Rows carry annotations such as `init_vault_at` (path resolved by adapter);
    // the identifier itself is the first token.
    Some(raw.split_whitespace().next()?.to_string())
}

#[test]
fn every_shipped_command_resolves_to_a_service_method() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace = manifest_dir.join("../..");
    let doc = workspace.join("docs/development/service-layer.md");
    let text = fs::read_to_string(&doc).expect("service-layer doc is readable");

    let mut rows = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("| `") {
            continue;
        }
        let cells: Vec<&str> = trimmed.split('|').collect();
        if cells.len() < 4 {
            continue;
        }
        let Some(command) = cell_ident(cells[1]) else {
            continue;
        };
        let Some(method) = cell_ident(cells[2]) else {
            continue;
        };
        rows.push((command, method));
    }

    assert_eq!(
        rows.len(),
        SHIPPED_COMMANDS,
        "the shipped command inventory changed; update docs/development/service-layer.md \
         and this gate together"
    );

    let mut sources = Vec::new();
    rust_sources(&workspace.join("crates/stratum-app/src"), &mut sources);
    let combined: String = sources
        .iter()
        .filter_map(|path| fs::read_to_string(path).ok())
        .collect::<Vec<_>>()
        .join("\n");

    let mut missing = Vec::new();
    for (command, method) in &rows {
        // The method cell is `module::fn` (or `fn` with an annotation); only the
        // final segment is the Rust function name.
        let function = method.rsplit("::").next().unwrap_or(method);
        let needle = format!("fn {function}(");
        if !combined.contains(&needle) {
            missing.push(format!("{command} -> {method}"));
        }
    }
    assert!(
        missing.is_empty(),
        "{} commands have no service method: {:?}",
        missing.len(),
        missing
    );
}

#[test]
fn the_native_provider_returns_the_same_kanban_data_as_the_service() {
    let dir = tempfile::tempdir().expect("temp vault");
    let service = stratum_app::StratumService::new(dir.path().to_path_buf());
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let created = runtime
        .block_on(service.create_kanban_block("Parity card".to_string(), "TODO".to_string()))
        .expect("task block is created");
    let service_rows = runtime
        .block_on(service.get_kanban_blocks())
        .expect("service reads the board");

    let provider = provider_from_env(dir.path().to_path_buf(), false);
    let native_rows = provider
        .kanban_cards()
        .expect("native seam reads the board");

    assert_eq!(service_rows.blocks.len(), native_rows.len());
    let service_row = service_rows
        .blocks
        .iter()
        .find(|row| row.id == created.id)
        .expect("the created card is on the board");
    let native_row = native_rows
        .iter()
        .find(|row| row.id == created.id)
        .expect("the created card is on the native board");
    assert_eq!(service_row.content, native_row.content);
    assert_eq!(service_row.marker, native_row.marker);
    assert_eq!(service_row.page_path, native_row.page_path);
}
