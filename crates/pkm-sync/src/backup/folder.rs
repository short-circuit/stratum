//! Local-folder differential backup mirror (E10.F3).
//!
//! Copies vault files into a destination directory, per file, tracking what
//! has been written in a `manifest.json` at the destination root. Files whose
//! size and mtime are unchanged are skipped; files present in the previous
//! manifest but missing locally are deleted at the destination. Files at the
//! destination that were never in the manifest are left untouched, so the
//! destination may safely contain unrelated data.

use crate::backup::{collect_files, load_manifest, write_manifest, BackupResult, FileMeta};
use chrono::Utc;
use pkm_core::PkmError;
use std::collections::BTreeMap;
use std::fs::File;
use std::path::Path;
use std::time::UNIX_EPOCH;

/// Backup the vault into `dest` as a differential mirror.
///
/// Returns a [`BackupResult`] describing the run. On error the result has
/// `success` false and a message in [`BackupResult::error`]; the returned
/// `Result` itself only carries `PkmError` for unrecoverable failures such as
/// being unable to create the destination directory.
pub fn backup_to_folder(
    vault: &Path,
    dest: &Path,
    exclude: &[String],
) -> Result<BackupResult, PkmError> {
    let started_at = Utc::now();
    match backup_folder_core(vault, dest, exclude) {
        Ok((copied, unchanged, deleted)) => Ok(BackupResult {
            success: true,
            target: "folder".to_string(),
            files_copied: copied,
            files_unchanged: unchanged,
            files_deleted: deleted,
            started_at,
            finished_at: Utc::now(),
            error: None,
        }),
        Err(e) => Ok(BackupResult {
            success: false,
            target: "folder".to_string(),
            files_copied: 0,
            files_unchanged: 0,
            files_deleted: 0,
            started_at,
            finished_at: Utc::now(),
            error: Some(e.to_string()),
        }),
    }
}

fn backup_folder_core(
    vault: &Path,
    dest: &Path,
    exclude: &[String],
) -> Result<(usize, usize, usize), PkmError> {
    std::fs::create_dir_all(dest).map_err(PkmError::Io)?;
    let old_manifest = load_manifest(dest);

    let files = collect_files(vault, exclude, Some(dest))?;

    let mut new_set: BTreeMap<String, FileMeta> = BTreeMap::new();
    for (_, rel, size, mtime) in &files {
        new_set.insert(
            rel.clone(),
            FileMeta {
                size: *size,
                mtime: *mtime,
            },
        );
    }

    let mut copied = 0;
    let mut unchanged = 0;

    for (abs, rel, size, mtime) in &files {
        let target = dest.join(rel);
        if destination_matches(&target, *size, *mtime) {
            unchanged += 1;
            continue;
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(PkmError::Io)?;
        }
        // Copy to a `.part` sibling first, then rename into place so a
        // partially-written file is never visible under its final name. We
        // carry the source mtime across so a subsequent run sees the file as
        // unchanged (copy alone would set a fresh timestamp and force a
        // re-copy on every run, breaking idempotency).
        let file_name = target
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let part = target.with_file_name(format!("{}.part", file_name));
        std::fs::copy(abs, &part).map_err(PkmError::Io)?;
        let source_meta = std::fs::metadata(abs).map_err(PkmError::Io)?;
        if let Ok(modified) = source_meta.modified() {
            let _ = File::open(&part).and_then(|f| f.set_modified(modified));
        }
        std::fs::rename(&part, &target).map_err(PkmError::Io)?;
        copied += 1;
    }

    // Delete propagation: only files that were recorded in the previous
    // manifest but no longer exist locally. Files present at the destination
    // that were never in the manifest are left untouched.
    let mut deleted = 0;
    if let Some(old) = old_manifest {
        for rel in old.files.keys() {
            if !new_set.contains_key(rel) {
                let target = dest.join(rel);
                if target.is_file() {
                    std::fs::remove_file(&target).map_err(PkmError::Io)?;
                    // Best-effort cleanup of now-empty parent directories.
                    if let Some(parent) = target.parent() {
                        let _ = std::fs::remove_dir(parent);
                    }
                    deleted += 1;
                }
            }
        }
    }

    write_manifest(dest, &new_set)?;
    Ok((copied, unchanged, deleted))
}

/// Whether `target` already matches the source `size`/`mtime`, so it can be
/// skipped without a copy.
fn destination_matches(target: &Path, size: u64, mtime: u64) -> bool {
    let Ok(meta) = std::fs::metadata(target) else {
        return false;
    };
    if !meta.is_file() || meta.len() != size {
        return false;
    }
    match meta.modified() {
        Ok(m) => m
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() == mtime)
            .unwrap_or(false),
        Err(_) => false,
    }
}
