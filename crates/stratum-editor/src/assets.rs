//! Headless attachment model for the native block editor.
//!
//! This module classifies asset embeds, formats the attachment rows the shipped
//! React `AttachmentPanel` shows, keeps a bounded thumbnail cache and reserves
//! deterministic row heights for the virtual layout. It performs no I/O and
//! owns no UI types: the phase-B view decodes bytes elsewhere and fills
//! [`crate::render::DecoratedRow::thumbnail`] lazily.
//!
//! The behavioural reference is the shipped React app:
//! `src/components/AttachmentPanel.tsx` (row labels and `formatBytes`) and
//! `src/lib/assetUpload.ts` (an upload is an image when its MIME starts with
//! `image/`).
//!
//! # Classification
//!
//! [`AssetKind::Image`] covers the raster formats the native decoder can turn
//! into a thumbnail (`png`, `jpg`, `jpeg`, `gif`, `webp`, `bmp`). SVG is a
//! [`AssetKind::File`] row: the raster pipeline does not decode it, so it is
//! listed with the basename, MIME and size instead of an image preview.
//!
//! # Thumbnail cache
//!
//! [`AssetThumbnailCache`] is a bounded LRU keyed by asset ref path. The
//! default bound is [`DEFAULT_THUMBNAIL_CAPACITY`] thumbnails; entries are
//! evicted least-recently-used first, and [`AssetThumbnailCache::get`] promotes
//! the entry it returns.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::document::EditorDocument;
use crate::inline::{parse_inline, SpanKind};
use crate::render::DecoratedSpan;

/// Default number of decoded thumbnails kept in an [`AssetThumbnailCache`].
pub const DEFAULT_THUMBNAIL_CAPACITY: usize = 32;

/// Reserved row height for an image embed before its thumbnail is decoded.
pub const ASSET_IMAGE_ROW_HEIGHT: f32 = 244.0;

/// Reserved row height for a non-image attachment row.
pub const ASSET_FILE_ROW_HEIGHT: f32 = 48.0;

/// Maximum thumbnail box (width, height) used when laying out an image row.
pub const THUMBNAIL_BOX: (f32, f32) = (320.0, 180.0);

/// How an asset reference is rendered in the editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetKind {
    /// A raster image the thumbnail pipeline can decode.
    Image,
    /// Any other attachment (PDF, audio, SVG, plain files).
    File,
}

/// Classify an asset ref path by its extension.
///
/// Only raster extensions (`png`, `jpg`, `jpeg`, `gif`, `webp`, `bmp`) are
/// images; matching is case-insensitive. SVG is deliberately a
/// [`AssetKind::File`] because the native thumbnail decoder only handles raster
/// formats — an SVG embed falls back to a file row.
pub fn asset_kind(ref_path: &str) -> AssetKind {
    let ext = std::path::Path::new(ref_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    match ext.to_ascii_lowercase().as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" => AssetKind::Image,
        _ => AssetKind::File,
    }
}

/// One asset reference found in a decorated row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetSpan {
    /// Vault-relative asset path (the `![[…]]` inner text).
    pub path: String,
    /// Rendering class of the referenced asset.
    pub kind: AssetKind,
}

/// Find the first asset embed span in `spans`, in order.
///
/// Returns `None` when the row carries no `![[…]]` asset embed.
pub fn first_asset_span(spans: &[DecoratedSpan]) -> Option<AssetSpan> {
    spans.iter().find_map(|span| match &span.kind {
        SpanKind::Asset { path } => Some(AssetSpan {
            path: path.clone(),
            kind: asset_kind(path),
        }),
        _ => None,
    })
}

/// The markdown embed reference for an asset path.
///
/// Mirrors React `importOne`, which inserts `![[<vault_relative_path>]]`.
pub fn asset_reference(ref_path: &str) -> String {
    format!("![[{ref_path}]]")
}

/// The basename of an asset ref path (the text after the last `/`).
pub fn display_name(ref_path: &str) -> &str {
    ref_path.rsplit('/').next().unwrap_or(ref_path)
}

/// Format a byte size exactly like `formatBytes` in `AttachmentPanel.tsx`.
///
/// Units are `B`, `KB`, `MB` and `GB`; the unit index is
/// `min(floor(log(bytes) / log(1024)), 3)` and the value uses zero decimals for
/// bytes and one decimal otherwise. The rounding matches JavaScript's
/// `toFixed`, which rounds halfway cases up. `format_size(0)` is `"0 B"`.
pub fn format_size(bytes: u64) -> String {
    if bytes == 0 {
        return "0 B".to_string();
    }
    let units = ["B", "KB", "MB", "GB"];
    let mut divisor: u128 = 1;
    let mut unit = 0usize;
    while unit < 3 && bytes as u128 >= divisor * 1024 {
        divisor *= 1024;
        unit += 1;
    }
    if unit == 0 {
        // `toFixed(0)` of an integer below 1024 is the integer itself.
        return format!("{bytes} B");
    }
    // Exact integer arithmetic: tenths = floor(bytes * 10 / divisor + 0.5),
    // i.e. round-half-up like `Number.prototype.toFixed(1)`.
    let tenths = ((bytes as u128) * 10 + divisor / 2) / divisor;
    format!("{}.{:01} {}", tenths / 10, tenths % 10, units[unit])
}

/// Input record for [`attachment_rows`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetRefInput {
    /// Vault-relative asset path.
    pub ref_path: String,
    /// MIME type recorded for the asset.
    pub mime: String,
    /// Size of the asset in bytes.
    pub size: u64,
}

/// One attachment list row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachmentRow {
    /// Vault-relative asset path.
    pub ref_path: String,
    /// Basename shown as the row title.
    pub name: String,
    /// MIME chip text.
    pub mime: String,
    /// Raw size in bytes.
    pub size: u64,
    /// Human-readable size label.
    pub size_label: String,
    /// Rendering class of the asset.
    pub kind: AssetKind,
}

/// Build attachment rows in input order.
///
/// Mirrors the shipped panel: each row shows the basename, the recorded MIME,
/// and the `formatBytes` size label.
pub fn attachment_rows(refs: &[AssetRefInput]) -> Vec<AttachmentRow> {
    refs.iter()
        .map(|input| AttachmentRow {
            ref_path: input.ref_path.clone(),
            name: display_name(&input.ref_path).to_string(),
            mime: input.mime.clone(),
            size: input.size,
            size_label: format_size(input.size),
            kind: asset_kind(&input.ref_path),
        })
        .collect()
}

/// A decoded thumbnail for an image asset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetThumbnail {
    /// PNG bytes of the decoded thumbnail.
    pub png: Arc<Vec<u8>>,
    /// Intrinsic image width in pixels.
    pub width: u32,
    /// Intrinsic image height in pixels.
    pub height: u32,
}

impl AssetThumbnail {
    /// Display size that fits the thumbnail inside `max_w` × `max_h`.
    ///
    /// The aspect ratio is preserved and the result never exceeds the
    /// intrinsic size (no upscaling). Zero-sized images or boxes yield
    /// `(0.0, 0.0)`.
    pub fn display_size(&self, max_w: f32, max_h: f32) -> (f32, f32) {
        let width = self.width as f32;
        let height = self.height as f32;
        if width <= 0.0 || height <= 0.0 || max_w <= 0.0 || max_h <= 0.0 {
            return (0.0, 0.0);
        }
        let scale = (max_w / width).min(max_h / height).min(1.0);
        (width * scale, height * scale)
    }
}

/// A bounded LRU cache of decoded asset thumbnails.
///
/// The key is the asset ref path. `get` marks the returned entry as recently
/// used; `insert` evicts least-recently-used entries once the bound is reached.
/// The cache is safe to share across the single UI thread (`Mutex` guards the
/// recency list).
#[derive(Debug)]
pub struct AssetThumbnailCache {
    entries: HashMap<String, Arc<AssetThumbnail>>,
    order: Mutex<Vec<String>>,
    capacity: usize,
}

impl Default for AssetThumbnailCache {
    fn default() -> Self {
        Self::new(DEFAULT_THUMBNAIL_CAPACITY)
    }
}

impl AssetThumbnailCache {
    /// Create a cache bounded to `capacity` thumbnails.
    ///
    /// A capacity of zero is clamped to one so the cache always makes progress.
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: HashMap::new(),
            order: Mutex::new(Vec::new()),
            capacity: capacity.max(1),
        }
    }

    /// Look up a thumbnail and mark it as recently used.
    pub fn get(&self, ref_path: &str) -> Option<Arc<AssetThumbnail>> {
        let thumbnail = self.entries.get(ref_path)?.clone();
        let mut order = self
            .order
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(position) = order.iter().position(|entry| entry == ref_path) {
            let entry = order.remove(position);
            order.push(entry);
        }
        Some(thumbnail)
    }

    /// Insert a thumbnail, evicting the least-recently-used entry if needed.
    pub fn insert(&mut self, ref_path: &str, thumbnail: AssetThumbnail) {
        let mut order = self
            .order
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        // Re-inserting an existing key must not leave a stale recency slot.
        if self.entries.remove(ref_path).is_some() {
            if let Some(position) = order.iter().position(|entry| entry == ref_path) {
                order.remove(position);
            }
        }
        while self.entries.len() >= self.capacity {
            let Some(oldest) = order.first().cloned() else {
                break;
            };
            order.remove(0);
            self.entries.remove(&oldest);
        }
        order.push(ref_path.to_string());
        self.entries
            .insert(ref_path.to_string(), Arc::new(thumbnail));
    }

    /// Number of cached thumbnails.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// The configured bound.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Reserved height for a row that embeds an asset of `kind`.
pub fn asset_row_height(kind: AssetKind) -> f32 {
    match kind {
        AssetKind::Image => ASSET_IMAGE_ROW_HEIGHT,
        AssetKind::File => ASSET_FILE_ROW_HEIGHT,
    }
}

/// Every asset path referenced by the document's blocks.
///
/// Both reference forms count — the `![[assets/…]]` embed and the plain
/// `[[assets/…]]` file link — matching the `asset_ref` rows the shipped app
/// stores in its links table. Paths are returned in document order and
/// deduplicated, keeping the first occurrence. This is the fallback attachment
/// list when the provider has no asset records for the page.
pub fn asset_refs_in_document(doc: &EditorDocument) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut push = |path: String| {
        if !out.iter().any(|existing| existing == &path) {
            out.push(path);
        }
    };
    for block in doc.blocks() {
        for span in parse_inline(&block.content) {
            match span.kind {
                SpanKind::Asset { path } => push(path),
                SpanKind::WikiLink { target, .. }
                    if pkm_markdown::linker::is_asset_link_target(&target) =>
                {
                    push(target);
                }
                _ => {}
            }
        }
    }
    out
}

#[cfg(test)]
mod tests;
