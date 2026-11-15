//! Embedded image files and data-URL helpers for `.excalidraw` documents.
//!
//! Excalidraw stores pasted images in the document's `files` map as
//! `{ fileId: { id, mimeType, dataURL, created } }` where `dataURL` is a
//! base64 `data:` URL. These helpers keep the map shape canonical and provide
//! the encode/decode used by attachments and exports.

use base64::Engine;
use pkm_core::{PkmError, PkmResult};
use serde_json::{json, Map, Value};

/// An empty `files` map.
pub fn empty_files() -> Value {
    json!({})
}

/// Extract the mime type from a base64 data URL (`data:image/png;base64,...`).
pub fn data_url_mime(data_url: &str) -> Option<&str> {
    let rest = data_url.strip_prefix("data:")?;
    let (meta, _) = rest.split_once(',')?;
    let mime = meta.strip_suffix(";base64")?;
    if mime.is_empty() {
        None
    } else {
        Some(mime)
    }
}

/// Decode a base64 data URL into `(mime, bytes)`.
///
/// # Errors
///
/// Returns [`PkmError::Canvas`] when the URL is not a base64 data URL or the
/// payload is not valid base64.
pub fn decode_data_url(data_url: &str) -> PkmResult<(String, Vec<u8>)> {
    let mime = data_url_mime(data_url)
        .ok_or_else(|| PkmError::Canvas("not a base64 data URL".to_string()))?;
    let (_, payload) = data_url
        .split_once(',')
        .ok_or_else(|| PkmError::Canvas("data URL without payload".to_string()))?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(payload)
        .map_err(|e| PkmError::Canvas(format!("invalid base64 payload: {e}")))?;
    Ok((mime.to_string(), bytes))
}

/// Read one `files` entry by id.
pub fn file_entry<'a>(files: &'a Value, file_id: &str) -> Option<&'a Value> {
    files.as_object()?.get(file_id)
}

/// Insert or replace one embedded file entry.
///
/// The `created` timestamp is `0` so fixtures and tests stay deterministic;
/// callers that care about the timestamp can overwrite it afterwards.
///
/// # Errors
///
/// Returns [`PkmError::Canvas`] when `files` is not a JSON object.
pub fn set_file(files: &mut Value, file_id: &str, data_url: &str, mime: &str) -> PkmResult<()> {
    if !files.is_object() {
        *files = empty_files();
    }
    let map = files
        .as_object_mut()
        .ok_or_else(|| PkmError::Canvas("files is not an object".to_string()))?;
    let mut entry = Map::new();
    entry.insert("id".into(), Value::String(file_id.to_string()));
    entry.insert("mimeType".into(), Value::String(mime.to_string()));
    entry.insert("dataURL".into(), Value::String(data_url.to_string()));
    entry.insert("created".into(), Value::from(0_u64));
    map.insert(file_id.to_string(), Value::Object(entry));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_url_round_trip() {
        let url = "data:image/png;base64,iVBORw0KGgo=";
        assert_eq!(data_url_mime(url), Some("image/png"));
        let (mime, bytes) = decode_data_url(url).unwrap();
        assert_eq!(mime, "image/png");
        assert_eq!(bytes, vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
    }

    #[test]
    fn set_file_builds_canonical_entry() {
        let mut files = empty_files();
        set_file(&mut files, "f1", "data:image/png;base64,AA==", "image/png").unwrap();
        let entry = file_entry(&files, "f1").unwrap();
        assert_eq!(entry["id"], json!("f1"));
        assert_eq!(entry["mimeType"], json!("image/png"));
    }
}
