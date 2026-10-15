//! Whole-vault ZIP export.
//!
//! Packs a portable vault archive (plain `.md` files, `assets/`, `templates/`,
//! plus a `manifest.json`) for backup or transfer. The `.pkm/` and `.git/`
//! directories are excluded everywhere, along with the vault-root export
//! artifacts, so the archive contains only user data. The core function
//! [`export_zip_core`] is pure (no Tauri) and testable; `export_zip` is the
//! thin Tauri glue.

use crate::commands::vault::AppState;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Result of a successful whole-vault ZIP export.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ZipExportResult {
    pub output_path: String,
    pub files_packed: usize,
    pub page_count: usize,
}

/// Core ZIP export — plain function so it is testable without a Tauri runtime.
///
/// Walks `vault` recursively and writes every user file to `output` as a
/// Deflated zip entry, preserving vault-relative paths with `/` separators.
/// The first entry is always `manifest.json` describing the archive.
pub fn export_zip_core(vault: &Path, output: &Path) -> Result<ZipExportResult, String> {
    let mut file_paths: Vec<PathBuf> = Vec::new();
    collect_files(vault, vault, &mut file_paths)?;

    let page_count = file_paths
        .iter()
        .filter(|p| p.extension().map(|e| e == "md").unwrap_or(false))
        .count();
    let out_count = file_paths.len();

    let file = std::fs::File::create(output).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let manifest = serde_json::json!({
        "format": "stratum-vault-zip",
        "version": "1",
        "exported_at": chrono::Utc::now().to_rfc3339(),
        "page_count": page_count,
        "file_count": out_count,
    });
    zip.start_file("manifest.json", options)
        .map_err(|e| e.to_string())?;
    let manifest_bytes = serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?;
    zip.write_all(&manifest_bytes).map_err(|e| e.to_string())?;

    for path in &file_paths {
        let rel = path.strip_prefix(vault).map_err(|e| e.to_string())?;
        let name = rel.to_string_lossy().replace('\\', "/");
        zip.start_file(name, options).map_err(|e| e.to_string())?;
        let data = std::fs::read(path).map_err(|e| e.to_string())?;
        zip.write_all(&data).map_err(|e| e.to_string())?;
    }

    zip.finish().map_err(|e| e.to_string())?;

    Ok(ZipExportResult {
        output_path: output.to_string_lossy().to_string(),
        // This counts every entry written to the archive, including the
        // manifest, so it matches the total number of readable zip entries.
        files_packed: out_count + 1,
        page_count,
    })
}

/// Thin Tauri command: exports the whole vault to the chosen `output_path`
/// when provided and non-empty (parent dir created if missing), otherwise to
/// `<vault>/export.zip`.
#[tauri::command]
pub async fn export_zip(
    output_path: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<ZipExportResult, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let vault = state.vault_path.clone();
    let output = super::resolve_output_path(&vault, output_path, "export.zip".to_string())?;
    export_zip_core(&vault, &output)
}

/// Recursively collect vault files to include in the archive. Directories
/// named `.pkm`/`.git` are skipped anywhere, and the vault-root export
/// artifacts (`export.html`, `export.json`, `export.zip`, `export.pdf`) are
/// skipped so an archive never packages its own previous output.
fn collect_files(vault: &Path, dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let ty = entry.file_type().map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().to_string();

        if ty.is_dir() {
            if name == ".pkm" || name == ".git" {
                continue;
            }
            collect_files(vault, &path, out)?;
        } else if ty.is_file() && !is_root_artifact(vault, &path) {
            out.push(path);
        }
    }
    Ok(())
}

fn is_root_artifact(vault: &Path, path: &Path) -> bool {
    let Ok(rel) = path.strip_prefix(vault) else {
        return false;
    };
    let at_root = rel
        .parent()
        .map(|p| p.as_os_str().is_empty())
        .unwrap_or(true);
    if !at_root {
        return false;
    }
    matches!(
        rel.file_name().and_then(|s| s.to_str()).unwrap_or_default(),
        "export.html" | "export.json" | "export.zip" | "export.pdf"
    )
}
