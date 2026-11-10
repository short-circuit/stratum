//! Asset / attachment management service methods.
//!
//! Thin service layer exposing the `pkm_block::assets` free functions. Each
//! method locks the shared vault state to clone the vault/db paths, then runs
//! the store work on a blocking pool so an async host event loop is never
//! stalled. Asset bytes are transported as base64; the file on disk remains the
//! single source of truth.
//!
//! Thumbnail generation and upload compression live in the pure [`image`]
//! submodule. The `image` crate (MIT/Apache-2.0, already present transitively)
//! is the decoder used for both.

use crate::state::StratumService;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};

pub mod image;

pub use image::{
    compress_image_for_upload, prepare_upload, thumbnail_for_image, ThumbnailImage,
    ASSET_MAX_BYTES, IMAGE_COMPRESS_MAX_EDGE, IMAGE_JPEG_QUALITY, THUMBNAIL_MAX_EDGE,
};

/// Thumbnail payload returned to the caller.
///
/// `data_base64` is an encoded PNG thumbnail; `width`/`height` are its scaled
/// pixel dimensions, `mime` is always `image/png`, and `size` is the byte length
/// of the PNG payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetThumbnailDto {
    pub data_base64: String,
    pub width: u32,
    pub height: u32,
    pub mime: String,
    pub size: u64,
}

/// Payload for reading an asset's bytes back to the caller.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetBytesDto {
    pub data_base64: String,
    pub mime: String,
    pub size: u64,
}

impl StratumService {
    /// Import an asset from an existing file path.
    ///
    /// The path is resolved inside the vault via `vault_path.join(path)` and the
    /// file's bytes are registered as an asset. `dest_name` optionally overrides
    /// the default deterministic name.
    pub async fn import_asset_path(
        &self,
        path: String,
        dest_name: Option<String>,
    ) -> Result<pkm_block::assets::AssetRef, String> {
        let vault_path = {
            let s = self.lock()?;
            s.vault_path.clone()
        };
        let db_path = {
            let s = self.lock()?;
            s.db_path.clone()
        };

        tokio::task::spawn_blocking(move || {
            let store = pkm_block::BlockStore::open(&db_path)?;
            let abs = vault_path.join(&path);
            pkm_block::assets::register_asset_path(
                &store,
                &vault_path,
                &abs,
                None,
                dest_name.as_deref(),
            )
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
    }

    /// Import an asset from base64-encoded bytes (e.g. a clipboard paste or drag-drop).
    ///
    /// `original_name` provides a fallback name; if empty it defaults to `paste.png`.
    pub async fn import_asset_bytes(
        &self,
        data_base64: String,
        original_name: String,
        mime: Option<String>,
        dest_name: Option<String>,
    ) -> Result<pkm_block::assets::AssetRef, String> {
        let vault_path = {
            let s = self.lock()?;
            s.vault_path.clone()
        };
        let db_path = {
            let s = self.lock()?;
            s.db_path.clone()
        };

        let bytes = STANDARD
            .decode(data_base64)
            .map_err(|e| format!("Invalid base64 data: {e}"))?;
        let original_name = if original_name.trim().is_empty() {
            "paste.png".to_string()
        } else {
            original_name
        };

        tokio::task::spawn_blocking(move || {
            let store = pkm_block::BlockStore::open(&db_path)?;
            pkm_block::assets::register_asset_bytes(
                &store,
                &vault_path,
                &bytes,
                &original_name,
                mime.as_deref(),
                dest_name.as_deref(),
            )
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
    }

    /// Import an asset from base64 bytes, enforcing the shared upload policy.
    ///
    /// Decodes the payload, derives the MIME from `original_name`/`mime`, runs
    /// [`prepare_upload`] (oversized images are compressed to JPEG, oversized
    /// non-images fail with `"file_too_large"`), then registers the resulting
    /// bytes. An empty `original_name` falls back to `paste.png`, matching the
    /// existing [`Self::import_asset_bytes`] behaviour.
    pub async fn import_asset_bytes_checked(
        &self,
        data_base64: String,
        original_name: String,
        mime: Option<String>,
        dest_name: Option<String>,
    ) -> Result<pkm_block::assets::AssetRef, String> {
        let vault_path = {
            let s = self.lock()?;
            s.vault_path.clone()
        };
        let db_path = {
            let s = self.lock()?;
            s.db_path.clone()
        };

        tokio::task::spawn_blocking(move || {
            let store = pkm_block::BlockStore::open(&db_path).map_err(|e| e.to_string())?;
            let bytes = STANDARD
                .decode(data_base64)
                .map_err(|e| format!("Invalid base64 data: {e}"))?;
            let original_name = if original_name.trim().is_empty() {
                "paste.png".to_string()
            } else {
                original_name
            };
            let inferred_mime = pkm_block::assets::mime_for(&original_name, mime.as_deref());
            let (bytes, mime) = prepare_upload(bytes, &inferred_mime)?;
            pkm_block::assets::register_asset_bytes(
                &store,
                &vault_path,
                &bytes,
                &original_name,
                Some(&mime),
                dest_name.as_deref(),
            )
            .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    /// Import an asset from a path inside the vault, enforcing the upload policy.
    ///
    /// This is the drag-and-drop entry point: it reads the file directly from
    /// disk (never base64-round-trips large payloads), infers the MIME from the
    /// file name, runs [`prepare_upload`], then registers the bytes.
    pub async fn import_asset_path_checked(
        &self,
        path: String,
        dest_name: Option<String>,
    ) -> Result<pkm_block::assets::AssetRef, String> {
        let vault_path = {
            let s = self.lock()?;
            s.vault_path.clone()
        };
        let db_path = {
            let s = self.lock()?;
            s.db_path.clone()
        };

        tokio::task::spawn_blocking(move || {
            let store = pkm_block::BlockStore::open(&db_path).map_err(|e| e.to_string())?;
            let abs = vault_path.join(&path);
            let bytes = std::fs::read(&abs).map_err(|e| format!("read asset file: {e}"))?;
            let original_name = abs
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("file")
                .to_string();
            let inferred_mime = pkm_block::assets::mime_for(&original_name, None);
            let (bytes, mime) = prepare_upload(bytes, &inferred_mime)?;
            pkm_block::assets::register_asset_bytes(
                &store,
                &vault_path,
                &bytes,
                &original_name,
                Some(&mime),
                dest_name.as_deref(),
            )
            .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    /// Generate a PNG thumbnail for a stored raster image asset.
    ///
    /// Reads the asset through `pkm_block::assets::read_asset`, refuses
    /// non-raster MIME types with the exact sentinel `"not_an_image"`, then
    /// delegates to [`thumbnail_for_image`]. SVG is rejected here because the
    /// native decoder only handles raster formats.
    pub async fn asset_thumbnail(
        &self,
        ref_path: String,
        max_edge: u32,
    ) -> Result<AssetThumbnailDto, String> {
        let vault_path = {
            let s = self.lock()?;
            s.vault_path.clone()
        };
        let db_path = {
            let s = self.lock()?;
            s.db_path.clone()
        };

        tokio::task::spawn_blocking(move || {
            let store = pkm_block::BlockStore::open(&db_path).map_err(|e| e.to_string())?;
            let (bytes, mime) = pkm_block::assets::read_asset(&store, &vault_path, &ref_path)
                .map_err(|e| e.to_string())?;
            if !pkm_block::assets::is_image_mime(&mime) {
                return Err("not_an_image".to_string());
            }
            let thumb = thumbnail_for_image(&bytes, max_edge)?;
            let size = thumb.png.len() as u64;
            Ok(AssetThumbnailDto {
                data_base64: STANDARD.encode(&thumb.png),
                width: thumb.width,
                height: thumb.height,
                mime: "image/png".to_string(),
                size,
            })
        })
        .await
        .map_err(|e| e.to_string())?
    }

    /// Read an asset's bytes (as base64), MIME, and size.
    pub async fn read_asset(&self, ref_path: String) -> Result<AssetBytesDto, String> {
        let vault_path = {
            let s = self.lock()?;
            s.vault_path.clone()
        };
        let db_path = {
            let s = self.lock()?;
            s.db_path.clone()
        };

        let (data, mime) = tokio::task::spawn_blocking(move || {
            let store = pkm_block::BlockStore::open(&db_path)?;
            let (bytes, mime) = pkm_block::assets::read_asset(&store, &vault_path, &ref_path)?;
            Ok::<_, pkm_core::PkmError>((bytes, mime))
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

        let size = data.len() as u64;
        Ok(AssetBytesDto {
            data_base64: STANDARD.encode(data),
            mime,
            size,
        })
    }

    /// Delete an asset (removes the file and its index record).
    pub async fn delete_asset(&self, ref_path: String) -> Result<(), String> {
        let vault_path = {
            let s = self.lock()?;
            s.vault_path.clone()
        };
        let db_path = {
            let s = self.lock()?;
            s.db_path.clone()
        };

        tokio::task::spawn_blocking(move || {
            let store = pkm_block::BlockStore::open(&db_path)?;
            pkm_block::assets::delete_asset(&store, &vault_path, &ref_path)
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
    }

    /// List every asset registered in the vault, ordered by ref path.
    pub async fn list_asset_refs(&self) -> Result<Vec<pkm_block::assets::AssetRef>, String> {
        let vault_path = {
            let s = self.lock()?;
            s.vault_path.clone()
        };
        let db_path = {
            let s = self.lock()?;
            s.db_path.clone()
        };

        tokio::task::spawn_blocking(move || {
            let store = pkm_block::BlockStore::open(&db_path)?;
            pkm_block::assets::list_assets(&store, &vault_path)
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
    }

    /// List the assets referenced by a specific page, resolving each to its record.
    pub async fn get_page_asset_refs(
        &self,
        page_path: String,
    ) -> Result<Vec<pkm_block::assets::AssetRef>, String> {
        let db_path = {
            let s = self.lock()?;
            s.db_path.clone()
        };

        tokio::task::spawn_blocking(move || {
            let store = pkm_block::BlockStore::open(&db_path)?;
            let refs = store.get_asset_refs_for_page(&page_path)?;
            let mut out = Vec::new();
            for rp in refs {
                if let Some(rec) = store.get_asset_record(&rp)? {
                    out.push(pkm_block::assets::AssetRef {
                        vault_relative_path: rec.ref_path,
                        mime: rec.mime,
                        size: rec.size as u64,
                        sha256: rec.sha256,
                    });
                }
            }
            Ok::<_, pkm_core::PkmError>(out)
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
    }
}
