//! Asset plumbing for [`ShellData`](crate::provider::ShellData) providers.
//!
//! This module holds everything the shell needs to turn asset references into
//! attachment rows and to drive the real [`AppProvider`] import/delete/thumbnail
//! flows. The pure row mapping lives in `stratum_editor::assets`; the helpers
//! here only infer a MIME type from a file name and build the default rows that
//! window-free providers return.
//!
//! MIME inference mirrors the subset of `pkm_block::assets::mime_from_extension`
//! the shell needs, because `stratum-ui` deliberately does not depend on
//! `pkm-block`. Provider errors are passed through unchanged so the panel can
//! translate the `file_too_large` sentinel.

use std::sync::Arc;

use stratum_app::asset::THUMBNAIL_MAX_EDGE;
use stratum_editor::{
    asset_kind, attachment_rows, display_name, format_size, AssetRefInput, AssetThumbnail,
    AttachmentRow,
};

use crate::provider::AppProvider;

/// Build an attachment row for a vault path without persisting anything.
///
/// The MIME is inferred from the extension and the size is unknown (`0`); this
/// is the default-provider row, used by providers that only accept the write.
pub fn attachment_row(ref_path: &str) -> AttachmentRow {
    attachment_row_with_mime(ref_path, 0, mime_for_extension(file_extension(ref_path)))
}

/// Build the default row for uploaded bytes: `assets/<basename>`.
pub(crate) fn attachment_row_for_bytes(name: &str, size: u64, mime: Option<&str>) -> AttachmentRow {
    let ref_path = format!("assets/{}", display_name(name));
    let mime = mime.unwrap_or_else(|| mime_for_extension(file_extension(name)));
    attachment_row_with_mime(&ref_path, size, mime)
}

/// Build a row from an explicit ref path, size and MIME.
pub(crate) fn attachment_row_with_mime(ref_path: &str, size: u64, mime: &str) -> AttachmentRow {
    AttachmentRow {
        ref_path: ref_path.to_string(),
        name: display_name(ref_path).to_string(),
        mime: mime.to_string(),
        size,
        size_label: format_size(size),
        kind: asset_kind(ref_path),
    }
}

/// Basename of a path without its extension, or `asset` when it has none.
pub(crate) fn file_stem(name: &str) -> &str {
    std::path::Path::new(name)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .unwrap_or("asset")
}

/// Extension of a path without the leading dot, or `bin` when it has none.
pub(crate) fn file_extension(name: &str) -> &str {
    std::path::Path::new(name)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("bin")
}

/// MIME type inferred from a file extension.
///
/// Mirrors the subset of `pkm_block::assets::mime_from_extension` the shell
/// needs; `stratum-ui` deliberately does not depend on `pkm-block`.
pub(crate) fn mime_for_extension(ext: &str) -> &'static str {
    match ext.trim_start_matches('.').to_ascii_lowercase().as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        "pdf" => "application/pdf",
        "md" => "text/markdown",
        "txt" => "text/plain",
        "json" => "application/json",
        "csv" => "text/csv",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "flac" => "audio/flac",
        "ogg" => "audio/ogg",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "zip" => "application/zip",
        "wasm" => "application/wasm",
        _ => "application/octet-stream",
    }
}

/// Build an attachment row from the service's asset reference fields.
///
/// The service already returns the canonical vault path, MIME and size, so the
/// row is built through the shared [`attachment_rows`] mapping to keep the
/// panel labels identical to the shipped app.
pub(crate) fn row_from_fields(ref_path: String, mime: String, size: u64) -> AttachmentRow {
    let input = AssetRefInput {
        ref_path,
        mime,
        size,
    };
    attachment_rows(std::slice::from_ref(&input))
        .pop()
        .expect("attachment_rows returns one row per input")
}

impl AppProvider {
    /// Read a decoded PNG thumbnail through the service layer.
    ///
    /// The service returns base64 PNG bytes; a decode failure or a non-image
    /// asset maps to `None` so the view keeps the chip fallback.
    pub(crate) fn read_thumbnail(&self, ref_path: &str) -> Option<AssetThumbnail> {
        let dto = self
            .runtime
            .block_on(
                self.service
                    .asset_thumbnail(ref_path.to_string(), THUMBNAIL_MAX_EDGE),
            )
            .ok()?;
        use base64::Engine;
        let png = base64::engine::general_purpose::STANDARD
            .decode(dto.data_base64.as_bytes())
            .ok()?;
        Some(AssetThumbnail {
            png: Arc::new(png),
            width: dto.width,
            height: dto.height,
        })
    }

    /// Read the attachment rows referenced by one page.
    pub(crate) fn read_attachments(&self, page_path: &str) -> Vec<AttachmentRow> {
        match self
            .runtime
            .block_on(self.service.get_page_asset_refs(page_path.to_string()))
        {
            Ok(references) => {
                let inputs: Vec<AssetRefInput> = references
                    .into_iter()
                    .map(|reference| AssetRefInput {
                        ref_path: reference.vault_relative_path,
                        mime: reference.mime,
                        size: reference.size,
                    })
                    .collect();
                attachment_rows(&inputs)
            }
            Err(_) => Vec::new(),
        }
    }

    /// Import raw bytes through the checked upload policy.
    ///
    /// The provider error string is passed through unchanged so the panel can
    /// translate the `file_too_large` sentinel.
    pub(crate) fn import_bytes(
        &self,
        bytes: Vec<u8>,
        name: &str,
        mime: Option<&str>,
    ) -> Result<AttachmentRow, String> {
        use base64::Engine;
        let data = base64::engine::general_purpose::STANDARD.encode(&bytes);
        let reference = self
            .runtime
            .block_on(self.service.import_asset_bytes_checked(
                data,
                name.to_string(),
                mime.map(str::to_string),
                None,
            ))?;
        Ok(row_from_fields(
            reference.vault_relative_path,
            reference.mime,
            reference.size,
        ))
    }

    /// Import a vault-relative path through the checked upload policy.
    pub(crate) fn import_path(&self, path: &str) -> Result<AttachmentRow, String> {
        let reference = self.runtime.block_on(
            self.service
                .import_asset_path_checked(path.to_string(), None),
        )?;
        Ok(row_from_fields(
            reference.vault_relative_path,
            reference.mime,
            reference.size,
        ))
    }

    /// Delete an attachment through the service layer.
    pub(crate) fn delete_ref(&self, ref_path: &str) -> Result<(), String> {
        self.runtime
            .block_on(self.service.delete_asset(ref_path.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mime_inference_is_case_insensitive_and_falls_back() {
        assert_eq!(mime_for_extension("PDF"), "application/pdf");
        assert_eq!(mime_for_extension(".PNG"), "image/png");
        assert_eq!(mime_for_extension("xyz"), "application/octet-stream");
        assert_eq!(file_stem("Report.final.pdf"), "Report.final");
        assert_eq!(file_extension("Report.final.pdf"), "pdf");
    }

    #[test]
    fn default_rows_classify_images_and_keep_the_basename() {
        let path_row = attachment_row("photos/a.png");
        assert_eq!(path_row.ref_path, "photos/a.png");
        assert_eq!(path_row.name, "a.png");
        assert_eq!(path_row.mime, "image/png");
        assert_eq!(path_row.size, 0);
        assert_eq!(path_row.kind, stratum_editor::AssetKind::Image);

        let bytes_row = attachment_row_for_bytes("paste.jpg", 3, None);
        assert_eq!(bytes_row.ref_path, "assets/paste.jpg");
        assert_eq!(bytes_row.mime, "image/jpeg");
        assert_eq!(bytes_row.size_label, "3 B");
    }
}
