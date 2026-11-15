//! Asset persistence: SQLite metadata records and `assets/` file operations.
//!
//! This submodule keeps the store-facing half of the asset API out of the
//! module root so no source file exceeds the repository limit. The free
//! functions are re-exported from [`super`] so every existing path
//! (`pkm_block::assets::register_asset_bytes`, …) keeps working.
//!
//! Assets (images, PDFs, audio, etc.) live as plain files in `<vault>/assets/`;
//! the SQLite `assets` table holds only a small metadata index. Writes are
//! atomic (temp file + rename) and deterministic
//! (`assets/<sanitized-stem>-<first-12-hex-of-sha256>.<ext>`), so identical
//! content dedupes to one file and one record.

use std::path::{Component, Path, PathBuf};

use pkm_core::PkmError;
use rusqlite::params;

use crate::store::{BlockStore, StoreResult};

use super::{compute_sha256_hex, mime_for, sanitize_stem, AssetRecord, AssetRef};

impl BlockStore {
    /// Insert an asset record, ignoring any existing row with the same `ref_path`.
    ///
    /// Uses `INSERT OR IGNORE` so re-registering a deduplicated asset is a no-op that
    /// never clobbers the existing record. Uniqueness is enforced by the UNIQUE index
    /// on `ref_path`.
    pub fn insert_asset_record(&self, asset: &AssetRecord) -> StoreResult<()> {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO assets
                 (id, ref_path, mime, size, sha256, created_at, modified_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    asset.id,
                    asset.ref_path,
                    asset.mime,
                    asset.size,
                    asset.sha256,
                    asset.created_at,
                    asset.modified_at
                ],
            )
            .map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?;
        Ok(())
    }

    /// Look up a single asset record by its `ref_path`.
    ///
    /// Returns `Ok(None)` when no asset with that ref path is registered.
    pub fn get_asset_record(&self, ref_path: &str) -> StoreResult<Option<AssetRecord>> {
        let result = self.conn.query_row(
            "SELECT id, ref_path, mime, size, sha256, created_at, modified_at
             FROM assets WHERE ref_path = ?1",
            params![ref_path],
            |row| {
                Ok(AssetRecord {
                    id: row.get(0)?,
                    ref_path: row.get(1)?,
                    mime: row.get(2)?,
                    size: row.get(3)?,
                    sha256: row.get(4)?,
                    created_at: row.get(5)?,
                    modified_at: row.get(6)?,
                })
            },
        );
        match result {
            Ok(rec) => Ok(Some(rec)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(PkmError::Internal(format!("SQLite error: {e}"))),
        }
    }

    /// List all registered asset records, ordered by `ref_path`.
    pub fn list_asset_records(&self) -> StoreResult<Vec<AssetRecord>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, ref_path, mime, size, sha256, created_at, modified_at
                 FROM assets ORDER BY ref_path",
            )
            .map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(AssetRecord {
                    id: row.get(0)?,
                    ref_path: row.get(1)?,
                    mime: row.get(2)?,
                    size: row.get(3)?,
                    sha256: row.get(4)?,
                    created_at: row.get(5)?,
                    modified_at: row.get(6)?,
                })
            })
            .map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?);
        }
        Ok(out)
    }

    /// Delete the asset record for `ref_path` (does not touch the file on disk).
    pub fn delete_asset_record(&self, ref_path: &str) -> StoreResult<()> {
        self.conn
            .execute("DELETE FROM assets WHERE ref_path = ?1", params![ref_path])
            .map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?;
        Ok(())
    }

    /// List the distinct asset ref paths referenced by a page.
    ///
    /// Returns the `target_page` values of every `links` row sourced by a block on the
    /// given page whose `link_type` is `asset_ref`, ordered by target path. This is the
    /// query the frontend uses to render an attachment list for a note.
    pub fn get_asset_refs_for_page(&self, page_path: &str) -> StoreResult<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT DISTINCT l.target_page
                 FROM links l
                 JOIN blocks b ON b.id = l.source_block
                 WHERE b.page_path = ?1 AND l.link_type = 'asset_ref'
                 ORDER BY l.target_page",
            )
            .map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?;
        let rows = stmt
            .query_map(params![page_path], |row| row.get::<_, String>(0))
            .map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?);
        }
        Ok(out)
    }
}

/// Register raw asset bytes into the vault's `assets/` directory.
///
/// # Determinism & collision handling
/// - The final name is `assets/<stem>-<first-12-hex>.ext`, where the stem is derived
///   from `dest_name` (if given) or `original_name`, and the hash is the SHA-256 of
///   `bytes`. Identical content therefore always maps to the same ref (dedupe).
/// - If a file already exists at the computed path with identical content, the existing
///   asset is returned (no new file, no new record).
/// - If it exists with *different* content (an extremely rare hash-prefix clash), the
///   suffix is bumped: `<stem>-<hash12>-2.ext`, `-3`, … until an unused name is found.
/// - If the (stripped) `dest_name` already conforms to the deterministic pattern
///   (`stem-hash12.ext`), it is respected as-is instead of being re-derived.
///
/// The write is atomic: content is written to a `.tmp-<uuid>` file in `assets/` and
/// then renamed into place, so readers never observe a partially written asset.
pub fn register_asset_bytes(
    store: &BlockStore,
    vault_path: &Path,
    bytes: &[u8],
    original_name: &str,
    mime: Option<&str>,
    dest_name: Option<&str>,
) -> StoreResult<AssetRef> {
    let hash = compute_sha256_hex(bytes);
    let hash12 = &hash[..12];

    let name_source = dest_name.unwrap_or(original_name);
    let (raw_stem, raw_ext) = split_name(name_source);
    // Never allow an empty stem - ".png" style dest names fall back to "asset".
    let stem = {
        let s = sanitize_stem(raw_stem);
        if s.chars().any(|c| c.is_ascii_alphanumeric()) {
            s
        } else {
            "asset".to_string()
        }
    };
    let ext = raw_ext.trim_start_matches('.').to_string();

    // Respect an already-canonical name, otherwise derive the deterministic name.
    let mut final_name = if matches_deterministic_pattern(name_source) {
        sanitize_filename(name_source)
    } else {
        format!("{stem}-{hash12}{}", ext_suffix(&ext))
    };

    let assets_dir = vault_path.join("assets");
    std::fs::create_dir_all(&assets_dir)
        .map_err(|e| PkmError::Internal(format!("create assets directory: {e}")))?;

    // Resolve collisions / dedupe by inspecting an existing file at the candidate path.
    let mut candidate_path = assets_dir.join(&final_name);
    let mut n = 2u32;
    loop {
        if !candidate_path.exists() {
            break;
        }
        let existing = std::fs::read(&candidate_path)
            .map_err(|e| PkmError::Internal(format!("read existing asset: {e}")))?;
        if compute_sha256_hex(&existing) == hash {
            // Identical content already present - dedupe to the existing asset.
            let ref_path = format!("assets/{final_name}");
            return existing_ref(store, &ref_path, mime, bytes.len(), &hash);
        }
        // Different content at this name (hash-prefix clash): bump `-2`, `-3`, …
        final_name = format!("{stem}-{hash12}-{n}{}", ext_suffix(&ext));
        candidate_path = assets_dir.join(&final_name);
        n += 1;
    }

    // Atomic write: temp file then rename into the final location.
    let tmp = assets_dir.join(format!(".tmp-{}", uuid::Uuid::new_v4()));
    std::fs::write(&tmp, bytes)
        .map_err(|e| PkmError::Internal(format!("write asset temp: {e}")))?;
    std::fs::rename(&tmp, &candidate_path)
        .map_err(|e| PkmError::Internal(format!("rename asset into place: {e}")))?;

    let ref_path = format!("assets/{final_name}");
    let record = AssetRecord {
        id: uuid::Uuid::new_v4().to_string(),
        ref_path: ref_path.clone(),
        mime: mime_for(&final_name, mime),
        size: bytes.len() as i64,
        sha256: hash.clone(),
        created_at: now_rfc3339(),
        modified_at: now_rfc3339(),
    };
    store.insert_asset_record(&record)?;
    Ok(AssetRef {
        vault_relative_path: ref_path,
        mime: record.mime,
        size: record.size as u64,
        sha256: record.sha256,
    })
}

/// Register an asset already present at an absolute path on disk.
///
/// Reads the file and delegates to [`register_asset_bytes`], deriving the original
/// name from the file's own name.
pub fn register_asset_path(
    store: &BlockStore,
    vault_path: &Path,
    abs_path: &Path,
    mime: Option<&str>,
    dest_name: Option<&str>,
) -> StoreResult<AssetRef> {
    let bytes =
        std::fs::read(abs_path).map_err(|e| PkmError::Internal(format!("read asset file: {e}")))?;
    let original_name = abs_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("file")
        .to_string();
    register_asset_bytes(store, vault_path, &bytes, &original_name, mime, dest_name)
}

/// Read an asset's bytes back from disk and its MIME type.
///
/// Validates that `ref_path` starts with `assets/` and resolves to a path contained
/// within `<vault>/assets/` (rejecting traversal). Returns the raw bytes and the MIME
/// type from the asset record, falling back to extension inference when no record exists.
pub fn read_asset(
    store: &BlockStore,
    vault_path: &Path,
    ref_path: &str,
) -> StoreResult<(Vec<u8>, String)> {
    let file_path = resolve_asset_path(vault_path, ref_path)?;
    let bytes = std::fs::read(&file_path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            PkmError::NotFound(format!("Asset not found: {ref_path}"))
        } else {
            PkmError::Internal(format!("read asset: {e}"))
        }
    })?;
    let mime = match store.get_asset_record(ref_path)? {
        Some(rec) => rec.mime,
        None => mime_for(ref_path, None),
    };
    Ok((bytes, mime))
}

/// Delete an asset: remove the file (if it exists) and its record.
///
/// Returns [`PkmError::NotFound`] when no asset record exists for `ref_path`. A missing
/// file on disk is tolerated (the record is still cleared).
pub fn delete_asset(store: &BlockStore, vault_path: &Path, ref_path: &str) -> StoreResult<()> {
    if store.get_asset_record(ref_path)?.is_none() {
        return Err(PkmError::NotFound(format!("Asset not found: {ref_path}")));
    }
    let file_path = resolve_asset_path(vault_path, ref_path)?;
    if file_path.exists() {
        std::fs::remove_file(&file_path)
            .map_err(|e| PkmError::Internal(format!("remove asset file: {e}")))?;
    }
    store.delete_asset_record(ref_path)?;
    Ok(())
}

/// List all assets registered in the vault, ordered by ref path.
pub fn list_assets(store: &BlockStore, _vault_path: &Path) -> StoreResult<Vec<AssetRef>> {
    let records = store.list_asset_records()?;
    let mut out: Vec<AssetRef> = records
        .into_iter()
        .map(|r| AssetRef {
            vault_relative_path: r.ref_path,
            mime: r.mime,
            size: r.size as u64,
            sha256: r.sha256,
        })
        .collect();
    out.sort_by(|a, b| a.vault_relative_path.cmp(&b.vault_relative_path));
    Ok(out)
}

/// Split a name into "(stem, extension)" without the leading dot on the extension.
fn split_name(name: &str) -> (&str, &str) {
    let stem = Path::new(name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(name);
    let ext = Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    (stem, ext)
}

/// Build the `.<ext>` suffix for a filename, or an empty string when there is no ext.
fn ext_suffix(ext: &str) -> String {
    if ext.is_empty() {
        String::new()
    } else {
        format!(".{ext}")
    }
}

/// Whether a name already conforms to the deterministic `stem-<12hex>.ext` pattern.
///
/// Used to respect a caller-provided canonical name instead of re-deriving it.
fn matches_deterministic_pattern(name: &str) -> bool {
    let lower = name.to_lowercase();
    let Some((_stem, rest)) = lower.rsplit_once('-') else {
        return false;
    };
    let hash_part = rest.split('.').next().unwrap_or(rest);
    hash_part.len() == 12 && hash_part.chars().all(|c| c.is_ascii_hexdigit())
}

/// Sanitize a full filename (stem + hash + ext), keeping `[a-z0-9._-]`.
fn sanitize_filename(name: &str) -> String {
    let mut out = String::new();
    for c in name.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
            out.push(c);
        } else {
            out.push('-');
        }
    }
    if out.is_empty() {
        "asset".to_string()
    } else {
        out
    }
}

/// Resolve and validate a `assets/...` ref path to an absolute filesystem path.
///
/// Rejects refs that don't start with `assets/` or that escape the `assets/` directory
/// (e.g. via `..`), guarding against path traversal.
fn resolve_asset_path(vault_path: &Path, ref_path: &str) -> StoreResult<PathBuf> {
    if !ref_path.starts_with("assets/") {
        return Err(PkmError::Validation(format!(
            "Asset ref must start with 'assets/': {ref_path}"
        )));
    }
    let assets_dir = vault_path.join("assets");
    let rel = ref_path.trim_start_matches("assets/");
    let full = assets_dir.join(rel);
    let norm_full = normalize_path(&full);
    let norm_assets = normalize_path(&assets_dir);
    if !norm_full.starts_with(&norm_assets) {
        return Err(PkmError::Validation(format!(
            "Asset path escapes the assets directory: {ref_path}"
        )));
    }
    Ok(norm_full)
}

/// Lexically normalize a path by resolving `.` and `..` components without touching
/// the filesystem, so containment checks work for paths that may not exist yet.
fn normalize_path(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// RFC 3339 timestamp for the current UTC time, matching how other tables store dates.
fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Return the existing asset identified by `ref_path`, creating its record if missing.
///
/// Used on the dedupe path: when the file already exists with identical content we do
/// not rewrite it, but the record must still be present for downstream queries.
fn existing_ref(
    store: &BlockStore,
    ref_path: &str,
    mime: Option<&str>,
    size: usize,
    hash: &str,
) -> StoreResult<AssetRef> {
    if let Some(rec) = store.get_asset_record(ref_path)? {
        return Ok(AssetRef {
            vault_relative_path: ref_path.to_string(),
            mime: rec.mime,
            size: rec.size as u64,
            sha256: rec.sha256,
        });
    }
    let record = AssetRecord {
        id: uuid::Uuid::new_v4().to_string(),
        ref_path: ref_path.to_string(),
        mime: mime_for(ref_path, mime),
        size: size as i64,
        sha256: hash.to_string(),
        created_at: now_rfc3339(),
        modified_at: now_rfc3339(),
    };
    store.insert_asset_record(&record)?;
    Ok(AssetRef {
        vault_relative_path: ref_path.to_string(),
        mime: record.mime.clone(),
        size: record.size as u64,
        sha256: record.sha256.clone(),
    })
}
