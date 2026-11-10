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
//!
//! # Module layout
//!
//! - This module owns the public types and pure helpers (classification, MIME,
//!   sanitization, hashing).
//! - [`store`] owns the [`crate::store::BlockStore`] asset-record methods and the
//!   file/record free functions, re-exported below so callers keep using
//!   `pkm_block::assets::register_asset_bytes` and friends.
//! - `tests` holds the module's test suite.

use std::path::Path;

use serde::{Deserialize, Serialize};

mod store;
#[cfg(test)]
mod tests;

pub use store::{delete_asset, list_assets, read_asset, register_asset_bytes, register_asset_path};

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

/// Whether a MIME type denotes an image format the app treats as an attachment image.
///
/// Matching is case-insensitive and trims surrounding whitespace. The list is the set of
/// raster/icon formats the shipped React uploader accepts as images. SVG is intentionally
/// excluded here: [`is_image_asset_path`] classifies SVG as an image by extension, but the
/// native thumbnail decoder in `stratum-app` can only decode raster formats.
pub fn is_image_mime(mime: &str) -> bool {
    let m = mime.trim().to_ascii_lowercase();
    matches!(
        m.as_str(),
        "image/png"
            | "image/jpeg"
            | "image/jpg"
            | "image/gif"
            | "image/webp"
            | "image/bmp"
            | "image/x-icon"
            | "image/vnd.microsoft.icon"
    )
}

/// Whether an asset ref path such as `assets/photo.png` denotes an image.
///
/// Classification is extension-based via [`mime_from_extension`], so it is case-insensitive
/// and works for both `assets/...` refs and plain file names. SVG counts as an image for
/// classification, but note that the `stratum-app` thumbnail helper only decodes raster
/// formats — callers should fall back to the original bytes for SVG.
pub fn is_image_asset_path(ref_path: &str) -> bool {
    let ext = Path::new(ref_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    matches!(mime_from_extension(ext), Some(m) if m.starts_with("image/"))
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
