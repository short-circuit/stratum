//! File normalization commands (normalize_file, normalize_all_files).
//!
//! Both re-serialize .md files through the block parser to enforce consistent
//! indentation, block syntax, and frontmatter.

use super::resolve_safe_path;
use crate::commands::vault::{AppState, IndexingGuard};
use pkm_core::fs_util::MdCollector;
use tauri::Emitter;
use tracing::info;

/// Normalize a single .md file by parsing it back through the block parser and
/// re-serializing. This ensures consistent indentation, block syntax, and
/// frontmatter. Other frontmatter fields beyond `title` are preserved verbatim
/// by extracting the raw frontmatter block if it existed.
#[tauri::command]
pub async fn normalize_file(path: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut state = state.lock().map_err(|e| e.to_string())?;
    let full_path = resolve_safe_path(&state.vault_path, &path)?;
    let content = std::fs::read_to_string(&full_path).map_err(|e| e.to_string())?;

    let (_fm, _body, blocks) = pkm_markdown::block_parser::parse_document(&content);
    let serialized = pkm_markdown::block_parser::serialize_blocks(&blocks);

    // Preserve original frontmatter YAML if it existed (including tags, created, etc.)
    let final_md = if content.trim_start().starts_with("---") {
        let yaml_str = serde_yaml::to_string(&_fm).unwrap_or_default();
        format!("---\n{}---\n\n{}", yaml_str, serialized)
    } else {
        serialized
    };

    std::fs::write(&full_path, &final_md).map_err(|e| e.to_string())?;
    // Mark this as our own save so the file watcher can skip it
    state.watcher_last_save = std::time::SystemTime::now();
    Ok(())
}

/// Normalize all .md files in the vault. Progress is reported via
/// "reindex-progress" Tauri events.
#[tauri::command]
pub async fn normalize_all_files(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<usize, String> {
    let mut state = state.lock().map_err(|e| e.to_string())?;
    // IndexingGuard clears the flag even when an early `?` returns, so an error
    // cannot leave the watcher permanently disabled.
    let _guard = IndexingGuard::new(&state)?;
    let md_files = MdCollector::new()
        .include_extensionless(true)
        .skip_dirs(vec![".pkm", "templates", ".git"])
        .collect_relative(&state.vault_path, &state.vault_path)
        .map_err(|e| e.to_string())?;

    let total = md_files.len();
    let mut count = 0;
    for (i, rel) in md_files.iter().enumerate() {
        let _ = app.emit(
            "reindex-progress",
            crate::commands::ProgressEventPayload {
                message: format!("Normalizing {}/{}", i + 1, total),
                percent: if total > 0 {
                    (i as f32 + 1.0) / total as f32
                } else {
                    1.0
                },
            },
        );

        let full_path = state.vault_path.join(rel);
        match std::fs::read_to_string(&full_path) {
            Ok(content) => {
                let (_fm, _body, blocks) = pkm_markdown::block_parser::parse_document(&content);
                let serialized = pkm_markdown::block_parser::serialize_blocks(&blocks);
                let final_md = if content.trim_start().starts_with("---") {
                    let yaml_str = serde_yaml::to_string(&_fm).unwrap_or_default();
                    format!("---\n{}---\n\n{}", yaml_str, serialized)
                } else {
                    serialized
                };
                let _ = std::fs::write(&full_path, &final_md);
                // Mark this as our own save so the file watcher can skip it
                state.watcher_last_save = std::time::SystemTime::now();
                count += 1;
            }
            Err(e) => {
                tracing::warn!("normalize_all_files: could not read {}: {}", rel, e);
            }
        }
    }

    let _ = app.emit(
        "reindex-progress",
        crate::commands::ProgressEventPayload {
            message: format!("Normalized {} files from filesystem", count),
            percent: 1.0,
        },
    );

    info!("Normalized {} files", count);
    // Invalidate graph cache so fresh data is served
    crate::commands::graph::invalidate_graph_cache();
    Ok(count)
}
