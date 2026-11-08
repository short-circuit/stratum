//! Catalog install helper: download → sha256 verify → stage → install.
//!
//! Separated from the command method so tests can drive it without a host.

use super::PluginManager;

/// Download + verify + stage + install from a catalog URL.
///
/// `catalog_id` is the catalog entry id, used as the staged file name: a
/// downloaded module without an embedded `stratum:manifest` falls back to the
/// file stem as identity, so naming the stage `<catalog_id>.wasm` keeps that
/// fallback aligned with the catalog (and filesystem-safe).
pub async fn install_url(
    manager: &PluginManager,
    url: &str,
    sha256: Option<&str>,
    catalog_id: &str,
) -> Result<super::PluginInfo, String> {
    let bytes = reqwest::Client::new()
        .get(url)
        .send()
        .await
        .map_err(|e| format!("plugin_download_error: {e}"))?
        .error_for_status()
        .map_err(|e| format!("plugin_download_error: {e}"))?
        .bytes()
        .await
        .map_err(|e| format!("plugin_download_error: {e}"))?;

    if let Some(expected) = sha256 {
        if !expected.is_empty() {
            let actual = {
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(&bytes);
                let digest = hasher.finalize();
                let mut hex = String::with_capacity(digest.len() * 2);
                for b in digest {
                    use std::fmt::Write;
                    let _ = write!(hex, "{b:02x}");
                }
                hex
            };
            if !actual.eq_ignore_ascii_case(expected.trim()) {
                return Err(format!(
                    "plugin_checksum_error: expected {expected}, got {actual}"
                ));
            }
        }
    }

    // Stage in a temp dir named after the catalog id (<catalog_id>.wasm). The
    // install copies it into the vault plugin directory before this function
    // returns, so the temp dir is removed on drop.
    let stage_dir = tempfile::tempdir().map_err(|e| format!("plugin_staging_error: {e}"))?;
    let staged_wasm = stage_dir.path().join(format!("{catalog_id}.wasm"));
    std::fs::write(&staged_wasm, &bytes).map_err(|e| format!("plugin_staging_error: {e}"))?;

    manager.install(&staged_wasm)
}
