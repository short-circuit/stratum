//! Asset / attachment storage for notes.
//!
//! Assets (images, PDFs, audio, etc.) live as plain files in `<vault>/assets/` — the
//! file on disk is the single source of truth. The SQLite `assets` table holds only a
//! small metadata index (ref path, mime, size, sha256, timestamps); no binaries are
//! stored in the database.
//!
//! Markdown references take two forms, both of which map to an asset row in the
//! `links` table with `link_type = 'asset_ref'`:
//! - `![[assets/foo.png]]` — inline embed
//! - `[[assets/foo.pdf]]` — file link
//!
//! Asset names are deterministic: `assets/<sanitized-stem>-<first-12-hex-of-sha256>.<ext>`.
//! This means identical content always yields an identical ref (natural dedupe), and
//! never overwrites an existing file.

use crate::store::{BlockStore, StoreResult};
use pkm_core::PkmError;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

/// Public metadata describing a stored asset.
///
/// Returned to callers (and serialized across the Tauri boundary) whenever an asset
/// is imported, read, or listed. `vault_relative_path` is the path relative to the
/// vault root, always prefixed with `assets/`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssetRef {
    /// Vault-relative path, e.g. `assets/foo-0123456789ab.png`.
    pub vault_relative_path: String,
    /// MIME type of the asset.
    pub mime: String,
    /// Size of the asset in bytes.
    pub size: u64,
    /// Lowercase hex SHA-256 digest of the asset content.
    pub sha256: String,
}

/// Internal persistent record mirroring a row in the `assets` SQLite table.
///
/// Not serialized to the frontend directly; convert to [`AssetRef`] for external use.
#[derive(Debug, Clone, PartialEq)]
pub struct AssetRecord {
    pub id: String,
    pub ref_path: String,
    pub mime: String,
    pub size: i64,
    pub sha256: String,
    pub created_at: String,
    pub modified_at: String,
}

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

/// Sanitize a file stem so it is safe as part of a deterministic asset filename.
///
/// Lowercases the input, keeps only `[a-z0-9._-]`, replaces every other character with
/// `-`, collapses consecutive dashes, caps the result at 64 characters, and falls back
/// to `"asset"` when nothing usable remains.
pub fn sanitize_stem(name: &str) -> String {
    let mut replaced = String::new();
    for c in name.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
            replaced.push(c);
        } else {
            replaced.push('-');
        }
    }

    // Collapse consecutive dashes introduced by replacement so stems stay tidy.
    let mut collapsed = String::new();
    let mut prev_dash = false;
    for c in replaced.chars() {
        if c == '-' {
            if !prev_dash {
                collapsed.push(c);
            }
            prev_dash = true;
        } else {
            collapsed.push(c);
            prev_dash = false;
        }
    }

    let trimmed = collapsed.trim_matches('-');
    if trimmed.is_empty() {
        return "asset".to_string();
    }
    trimmed.chars().take(64).collect()
}

/// Map a file extension to a MIME type.
///
/// Returns `None` for unknown extensions so callers can fall back to
/// `application/octet-stream`. The extension match is case-insensitive.
pub fn mime_from_extension(ext: &str) -> Option<&'static str> {
    let e = ext.trim_start_matches('.').to_lowercase();
    match e.as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        "avif" => Some("image/avif"),
        "svg" => Some("image/svg+xml"),
        "bmp" => Some("image/bmp"),
        "ico" => Some("image/x-icon"),
        "pdf" => Some("application/pdf"),
        "md" => Some("text/markdown"),
        "txt" => Some("text/plain"),
        "json" => Some("application/json"),
        "csv" => Some("text/csv"),
        "html" => Some("text/html"),
        "mp3" => Some("audio/mpeg"),
        "wav" => Some("audio/wav"),
        "flac" => Some("audio/flac"),
        "ogg" | "oga" => Some("audio/ogg"),
        "m4a" => Some("audio/mp4"),
        "mp4" => Some("video/mp4"),
        "webm" => Some("video/webm"),
        "mov" => Some("video/quicktime"),
        "zip" => Some("application/zip"),
        "7z" => Some("application/x-7z-compressed"),
        "gz" => Some("application/gzip"),
        "tar" => Some("application/x-tar"),
        "wasm" => Some("application/wasm"),
        "woff" => Some("font/woff"),
        "woff2" => Some("font/woff2"),
        "ttf" => Some("font/ttf"),
        "otf" => Some("font/otf"),
        "xlsx" => Some("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"),
        "docx" => Some("application/vnd.openxmlformats-officedocument.wordprocessingml.document"),
        _ => None,
    }
}

/// Determine the MIME type for an asset name.
///
/// A non-empty `provided` MIME always wins; otherwise the extension map is consulted;
/// otherwise the result is `application/octet-stream`.
pub fn mime_for(name: &str, provided: Option<&str>) -> String {
    if let Some(p) = provided {
        let p = p.trim();
        if !p.is_empty() {
            return p.to_string();
        }
    }
    let ext = Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    mime_from_extension(ext)
        .map(|s| s.to_string())
        .unwrap_or_else(|| "application/octet-stream".to_string())
}

/// Compute the lowercase hex SHA-256 digest of a byte slice.
///
/// Uses the `sha2` crate (already a workspace dependency) and formats each byte with
/// `{:02x}` — no separate hex crate is pulled in.
pub fn compute_sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    digest.iter().map(|b| format!("{b:02x}")).collect()
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Create a temp vault with a BlockStore whose DB lives under `.pkm/`.
    fn setup() -> (TempDir, BlockStore, PathBuf) {
        let tmp = TempDir::new().unwrap();
        let vault = tmp.path().to_path_buf();
        std::fs::create_dir_all(vault.join(".pkm")).unwrap();
        let db = vault.join(".pkm").join("blocks.db");
        let store = BlockStore::open(&db).unwrap();
        (tmp, store, vault)
    }

    #[test]
    fn test_register_and_read_round_trip() {
        let (_tmp, store, vault) = setup();
        let bytes = b"hello asset".to_vec();
        let asset = register_asset_bytes(&store, &vault, &bytes, "foo.png", None, None).unwrap();
        // Deterministic ref mirrors the hash prefix.
        assert_eq!(
            asset.vault_relative_path,
            format!("assets/foo-{}.png", &asset.sha256[..12])
        );
        assert!(vault.join(&asset.vault_relative_path).exists());
        let rec = store
            .get_asset_record(&asset.vault_relative_path)
            .unwrap()
            .unwrap();
        assert_eq!(rec.mime, "image/png");
        assert_eq!(rec.size, bytes.len() as i64);
        // read_asset returns identical bytes + mime.
        let (read, mime) = read_asset(&store, &vault, &asset.vault_relative_path).unwrap();
        assert_eq!(read, bytes);
        assert_eq!(mime, "image/png");
    }

    #[test]
    fn test_dedupe_same_bytes_same_name() {
        let (_tmp, store, vault) = setup();
        let bytes = b"dup content".to_vec();
        let a = register_asset_bytes(&store, &vault, &bytes, "a.png", None, None).unwrap();
        let b = register_asset_bytes(&store, &vault, &bytes, "a.png", None, None).unwrap();
        // Same ref, only one record and one file.
        assert_eq!(a.vault_relative_path, b.vault_relative_path);
        assert_eq!(store.list_asset_records().unwrap().len(), 1);
        let files: Vec<_> = std::fs::read_dir(vault.join("assets"))
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| !e.file_name().to_string_lossy().starts_with(".tmp-"))
            .collect();
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn test_collision_different_bytes_same_stem() {
        let (_tmp, store, vault) = setup();
        let a = register_asset_bytes(&store, &vault, b"content-one", "a.png", None, None).unwrap();
        let b = register_asset_bytes(&store, &vault, b"content-two", "a.png", None, None).unwrap();
        // Distinct hashes => distinct refs, both files exist, both under assets/.
        assert_ne!(a.vault_relative_path, b.vault_relative_path);
        assert!(a.vault_relative_path.starts_with("assets/a-"));
        assert!(b.vault_relative_path.starts_with("assets/a-"));
        assert!(a.vault_relative_path.starts_with("assets/"));
        assert!(b.vault_relative_path.starts_with("assets/"));
        assert!(vault.join(&a.vault_relative_path).exists());
        assert!(vault.join(&b.vault_relative_path).exists());
    }

    #[test]
    fn test_deterministic_ref_path() {
        let (_tmp, store, vault) = setup();
        let bytes = b"deterministic".to_vec();
        let a = register_asset_bytes(&store, &vault, &bytes, "det.png", None, None).unwrap();
        // Registering the same bytes/name yields an identical ref (hash-based).
        let b = register_asset_bytes(&store, &vault, &bytes, "det.png", None, None).unwrap();
        assert_eq!(a.vault_relative_path, b.vault_relative_path);
        assert_eq!(
            a.vault_relative_path,
            format!("assets/det-{}.png", &a.sha256[..12])
        );
    }

    #[test]
    fn test_mime_inference() {
        assert_eq!(mime_from_extension("png"), Some("image/png"));
        assert_eq!(mime_from_extension(".PNG"), Some("image/png"));
        assert_eq!(mime_from_extension("xyz"), None);
        assert_eq!(mime_for("a.png", None), "image/png");
        assert_eq!(mime_for("a.unknown", None), "application/octet-stream");
        // Provided MIME wins.
        assert_eq!(mime_for("a.png", Some("text/plain")), "text/plain");
        // Empty provided MIME falls back to inference.
        assert_eq!(mime_for("a.png", Some("")), "image/png");
    }

    #[test]
    fn test_delete_asset() {
        let (_tmp, store, vault) = setup();
        let asset =
            register_asset_bytes(&store, &vault, b"delete me", "d.png", None, None).unwrap();
        assert!(vault.join(&asset.vault_relative_path).exists());
        delete_asset(&store, &vault, &asset.vault_relative_path).unwrap();
        assert!(!vault.join(&asset.vault_relative_path).exists());
        assert!(store
            .get_asset_record(&asset.vault_relative_path)
            .unwrap()
            .is_none());
        // Deleting a nonexistent ref is an error.
        let err = delete_asset(&store, &vault, "assets/nope.png").unwrap_err();
        assert!(matches!(err, PkmError::NotFound(_)));
        // Reading a nonexistent ref is an error.
        let err = read_asset(&store, &vault, "assets/nope.png").unwrap_err();
        assert!(matches!(err, PkmError::NotFound(_)));
    }

    #[test]
    fn test_path_validation() {
        let (_tmp, store, vault) = setup();
        let asset = register_asset_bytes(&store, &vault, b"x", "v.png", None, None).unwrap();
        assert!(read_asset(&store, &vault, &asset.vault_relative_path).is_ok());
        // Non-asset prefix rejected.
        let err = read_asset(&store, &vault, "pages/x.png").unwrap_err();
        assert!(matches!(err, PkmError::Validation(_)));
        // `..` escaping the assets dir rejected.
        let err = read_asset(&store, &vault, "assets/../x.png").unwrap_err();
        assert!(matches!(err, PkmError::Validation(_)));
        // Nested path under assets/ is valid (file missing -> NotFound, not Validation).
        let err = read_asset(&store, &vault, "assets/sub/x.png").unwrap_err();
        assert!(matches!(err, PkmError::NotFound(_)));
    }

    #[test]
    fn test_get_asset_refs_for_page() {
        let (_tmp, store, _vault) = setup();
        let id = uuid::Uuid::new_v4();
        store
            .insert_block(
                &crate::block::Block::new(id, "see [[assets/foo.png]]".into()),
                "pages/p.md",
            )
            .unwrap();
        store
            .insert_link(id, "asset_ref", Some("assets/foo.png"), None)
            .unwrap();
        store
            .insert_link(id, "page_ref", Some("pages/other.md"), None)
            .unwrap();
        let refs = store.get_asset_refs_for_page("pages/p.md").unwrap();
        assert_eq!(refs, vec!["assets/foo.png".to_string()]);
        // A page with no assets yields an empty list.
        let empty = store.get_asset_refs_for_page("pages/q.md").unwrap();
        assert!(empty.is_empty());
    }

    #[test]
    fn test_sanitize_stem() {
        assert_eq!(sanitize_stem("Hello World"), "hello-world");
        assert_eq!(sanitize_stem("  Foo__Bar  "), "foo__bar");
        assert_eq!(sanitize_stem("a/b\\c"), "a-b-c");
        assert_eq!(sanitize_stem("!!!"), "asset");
        assert_eq!(sanitize_stem(""), "asset");
        // Trailing dash collapsed.
        assert_eq!(sanitize_stem("name-"), "name");
    }
}
