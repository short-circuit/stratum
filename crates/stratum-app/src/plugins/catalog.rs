//! Plugin and theme catalogs: schema validation, disk cache and service
//! methods.
//!
//! The shipped React app keeps these catalogs in `localStorage`
//! (`src/lib/pluginCatalog.ts`). The headless service keeps the same payloads
//! under `<vault>/.pkm/catalog/` so the native shell works offline after the
//! first successful fetch. Network access is behind [`CatalogFetch`] so tests
//! never touch the network.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::state::StratumService;

/// Stable error returned when a catalog payload fails schema validation.
///
/// Hosts map this string to `plugins.catalog_schema_invalid`.
pub const CATALOG_SCHEMA_INVALID: &str = "catalog_schema_invalid";
/// Stable error returned when neither the network nor the disk cache works.
///
/// Hosts map this string to `plugins.catalog_error` / `plugins.catalog_offline`.
pub const CATALOG_UNAVAILABLE: &str = "catalog_unavailable";

/// Community plugin catalog URL.
pub const PLUGIN_CATALOG_URL: &str =
    "https://raw.githubusercontent.com/short-circuit/stratum-plugins/main/plugins/index.json";
/// Community theme catalog URL.
pub const THEME_CATALOG_URL: &str =
    "https://raw.githubusercontent.com/short-circuit/stratum-plugins/main/themes/index.json";

/// Which catalog a cache entry belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogKind {
    /// Plugin catalog (`plugins/index.json`).
    Plugins,
    /// Theme catalog (`themes/index.json`).
    Themes,
}

impl CatalogKind {
    /// The remote URL for this catalog.
    pub fn url(self) -> &'static str {
        match self {
            Self::Plugins => PLUGIN_CATALOG_URL,
            Self::Themes => THEME_CATALOG_URL,
        }
    }

    /// The cache file name under `<vault>/.pkm/catalog/`.
    pub fn file_name(self) -> &'static str {
        match self {
            Self::Plugins => "plugins.json",
            Self::Themes => "themes.json",
        }
    }
}

/// One entry in the community plugin catalog.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PluginCatalogEntry {
    /// Catalog schema version for the entry.
    #[serde(default)]
    pub schema_version: u32,
    /// Stable plugin id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Plugin version.
    pub version: String,
    /// Short description.
    #[serde(default)]
    pub description: String,
    /// Optional author name.
    #[serde(default)]
    pub author: Option<String>,
    /// WASM entry file name inside the plugin directory.
    #[serde(default)]
    pub entry: String,
    /// Declared permissions.
    #[serde(default)]
    pub permissions: Vec<String>,
    /// Hook enablement map from the catalog.
    #[serde(default)]
    pub hooks: Option<BTreeMap<String, bool>>,
    /// Source repository URL.
    #[serde(default)]
    pub repo: String,
    /// Optional source file reference.
    #[serde(default)]
    pub source: Option<String>,
    /// Lowercase hex sha256 digest of the download.
    #[serde(default)]
    pub sha256: String,
    /// Download URL for the WASM module.
    #[serde(default)]
    pub download_url: String,
}

/// One entry in the community theme catalog.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThemeCatalogEntry {
    /// Stable theme id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Theme version.
    pub version: String,
    /// Short description.
    #[serde(default)]
    pub description: String,
    /// Optional author name.
    #[serde(default)]
    pub author: Option<String>,
    /// Whether the theme is dark.
    #[serde(default)]
    pub dark_mode: bool,
    /// Primary color.
    #[serde(default)]
    pub primary_color: String,
    /// Secondary color.
    #[serde(default)]
    pub secondary_color: String,
    /// Base font size.
    #[serde(default)]
    pub font_size: f64,
    /// Source repository URL.
    #[serde(default)]
    pub repo: String,
    /// Optional source file reference.
    #[serde(default)]
    pub source: Option<String>,
    /// Lowercase hex sha256 digest of the download.
    #[serde(default)]
    pub sha256: String,
    /// Download URL for the theme document.
    #[serde(default)]
    pub download_url: String,
}

/// A catalog snapshot returned to hosts.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct CatalogSnapshot<T> {
    /// Validated catalog entries.
    pub entries: Vec<T>,
    /// True when the entries came from the disk cache because the network
    /// fetch failed.
    pub offline: bool,
    /// ISO timestamp of the last successful remote fetch.
    pub fetched_at: Option<String>,
    /// Stable error message when the network fetch failed.
    pub error: Option<String>,
}

/// Network fetch abstraction so tests can inject a fake catalog.
///
/// `async fn` in the trait is intentional: callers are generic over
/// `CatalogFetch`, so the missing `Send` guarantee on the returned future is
/// not observable. The lint is allowed for that reason.
#[allow(async_fn_in_trait)]
pub trait CatalogFetch: Send + Sync {
    /// Download `url` and return its raw bytes.
    async fn fetch(&self, url: &str) -> Result<Vec<u8>, String>;
}

/// Production fetch implementation using `reqwest` with a 10 s timeout.
#[derive(Debug, Clone, Copy, Default)]
pub struct ReqwestFetch;

impl CatalogFetch for ReqwestFetch {
    async fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|_| "catalog_network_error".to_string())?;
        let response = client
            .get(url)
            .send()
            .await
            .map_err(|_| "catalog_network_error".to_string())?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!("catalog_http_{}", status.as_u16()));
        }
        response
            .bytes()
            .await
            .map(|bytes| bytes.to_vec())
            .map_err(|_| "catalog_network_error".to_string())
    }
}

/// True when `value` is exactly 64 lowercase/uppercase hex characters.
fn is_hex64(value: &str) -> bool {
    value.len() == 64 && value.chars().all(|c| c.is_ascii_hexdigit())
}

/// Reject an entry unless the required string fields are non-empty.
fn require_strings(entry: &Value, fields: &[&str]) -> bool {
    fields.iter().all(|field| {
        entry
            .get(*field)
            .and_then(Value::as_str)
            .is_some_and(|value| !value.trim().is_empty())
    })
}

/// Reject an entry unless its `sha256` is a 64-character hex digest.
fn require_sha256(entry: &Value) -> bool {
    entry
        .get("sha256")
        .and_then(Value::as_str)
        .is_some_and(is_hex64)
}

/// Parse and validate a plugin catalog document.
///
/// Returns [`CATALOG_SCHEMA_INVALID`] for a wrong/missing schema, a non-array
/// `plugins` field, a non-object entry, an empty required string, or a
/// malformed `sha256`.
pub fn parse_plugin_catalog(bytes: &[u8]) -> Result<Vec<PluginCatalogEntry>, String> {
    let document: Value =
        serde_json::from_slice(bytes).map_err(|_| CATALOG_SCHEMA_INVALID.to_string())?;
    if document.get("schema").and_then(Value::as_str) != Some("stratum-plugins-catalog/v1") {
        return Err(CATALOG_SCHEMA_INVALID.to_string());
    }
    let entries = document
        .get("plugins")
        .and_then(Value::as_array)
        .ok_or_else(|| CATALOG_SCHEMA_INVALID.to_string())?;

    let mut parsed = Vec::with_capacity(entries.len());
    for entry in entries {
        if !entry.is_object()
            || !require_strings(entry, &["id", "name", "version", "download_url"])
            || !require_sha256(entry)
        {
            return Err(CATALOG_SCHEMA_INVALID.to_string());
        }
        let typed: PluginCatalogEntry = serde_json::from_value(entry.clone())
            .map_err(|_| CATALOG_SCHEMA_INVALID.to_string())?;
        parsed.push(typed);
    }
    Ok(parsed)
}

/// Parse and validate a theme catalog document.
///
/// Applies the same rules as [`parse_plugin_catalog`] with the
/// `stratum-theme-catalog/v1` schema and a `themes` array.
pub fn parse_theme_catalog(bytes: &[u8]) -> Result<Vec<ThemeCatalogEntry>, String> {
    let document: Value =
        serde_json::from_slice(bytes).map_err(|_| CATALOG_SCHEMA_INVALID.to_string())?;
    if document.get("schema").and_then(Value::as_str) != Some("stratum-theme-catalog/v1") {
        return Err(CATALOG_SCHEMA_INVALID.to_string());
    }
    let entries = document
        .get("themes")
        .and_then(Value::as_array)
        .ok_or_else(|| CATALOG_SCHEMA_INVALID.to_string())?;

    let mut parsed = Vec::with_capacity(entries.len());
    for entry in entries {
        if !entry.is_object()
            || !require_strings(entry, &["id", "name", "version", "download_url"])
            || !require_sha256(entry)
        {
            return Err(CATALOG_SCHEMA_INVALID.to_string());
        }
        let typed: ThemeCatalogEntry = serde_json::from_value(entry.clone())
            .map_err(|_| CATALOG_SCHEMA_INVALID.to_string())?;
        parsed.push(typed);
    }
    Ok(parsed)
}

/// Disk-cache record shape (`{"payload": …, "fetched_at": "…"}`).
#[derive(Debug, Serialize, Deserialize)]
struct CacheRecord {
    payload: Value,
    fetched_at: String,
}

/// Path of the catalog cache directory for `vault`.
fn cache_dir(vault: &Path) -> PathBuf {
    vault.join(".pkm").join("catalog")
}

/// Read a catalog cache entry, treating a missing or corrupt file as absent.
///
/// Returns the raw catalog payload bytes and the recorded fetch timestamp.
pub fn read_cache(vault: &Path, kind: CatalogKind) -> Option<(Vec<u8>, String)> {
    let path = cache_dir(vault).join(kind.file_name());
    let raw = std::fs::read(&path).ok()?;
    let record: CacheRecord = serde_json::from_slice(&raw).ok()?;
    let payload = serde_json::to_vec(&record.payload).ok()?;
    Some((payload, record.fetched_at))
}

/// Write a catalog payload to the disk cache.
///
/// The payload must be valid JSON; the wrapper stores the parsed document so
/// corrupt caches cannot be produced by this function.
pub fn write_cache(
    vault: &Path,
    kind: CatalogKind,
    payload: &[u8],
    fetched_at: &str,
) -> Result<(), String> {
    let document: Value =
        serde_json::from_slice(payload).map_err(|_| CATALOG_SCHEMA_INVALID.to_string())?;
    let record = CacheRecord {
        payload: document,
        fetched_at: fetched_at.to_string(),
    };
    let dir = cache_dir(vault);
    std::fs::create_dir_all(&dir).map_err(|e| format!("catalog_cache_write_error: {e}"))?;
    let encoded = serde_json::to_vec(&record).map_err(|e| e.to_string())?;
    std::fs::write(dir.join(kind.file_name()), encoded)
        .map_err(|e| format!("catalog_cache_write_error: {e}"))
}

/// Return a cached snapshot, or [`CATALOG_UNAVAILABLE`] when the cache is
/// missing or invalid.
fn cached_snapshot<T: Serialize>(
    vault: &Path,
    kind: CatalogKind,
    parse: fn(&[u8]) -> Result<Vec<T>, String>,
) -> Result<CatalogSnapshot<T>, String> {
    let Some((payload, fetched_at)) = read_cache(vault, kind) else {
        return Err(CATALOG_UNAVAILABLE.to_string());
    };
    let entries = parse(&payload).map_err(|_| CATALOG_UNAVAILABLE.to_string())?;
    Ok(CatalogSnapshot {
        entries,
        offline: true,
        fetched_at: Some(fetched_at),
        error: Some(CATALOG_UNAVAILABLE.to_string()),
    })
}

/// Fetch, validate and cache a catalog, falling back to the disk cache.
///
/// On a network or validation failure the cached entries are returned with
/// `offline: true`. When neither works the result is
/// [`CATALOG_UNAVAILABLE`].
pub async fn load_catalog<T, F>(
    vault: &Path,
    kind: CatalogKind,
    fetch: &F,
    parse: fn(&[u8]) -> Result<Vec<T>, String>,
) -> Result<CatalogSnapshot<T>, String>
where
    T: Serialize,
    F: CatalogFetch,
{
    match fetch.fetch(kind.url()).await {
        Ok(bytes) => match parse(&bytes) {
            Ok(entries) => {
                let fetched_at = Utc::now().to_rfc3339();
                let _ = write_cache(vault, kind, &bytes, &fetched_at);
                Ok(CatalogSnapshot {
                    entries,
                    offline: false,
                    fetched_at: Some(fetched_at),
                    error: None,
                })
            }
            Err(_) => cached_snapshot(vault, kind, parse),
        },
        Err(_) => cached_snapshot(vault, kind, parse),
    }
}

impl StratumService {
    /// Fetch the community plugin catalog, caching it on success.
    ///
    /// Returns the cached catalog with `offline: true` when the network fails,
    /// and [`CATALOG_UNAVAILABLE`] when there is no usable cache either.
    pub async fn plugins_catalog(&self) -> Result<CatalogSnapshot<PluginCatalogEntry>, String> {
        load_catalog(
            &self.vault_path(),
            CatalogKind::Plugins,
            &ReqwestFetch,
            parse_plugin_catalog,
        )
        .await
    }

    /// Fetch the community theme catalog, caching it on success.
    ///
    /// Same offline/fallback contract as [`Self::plugins_catalog`].
    pub async fn themes_catalog(&self) -> Result<CatalogSnapshot<ThemeCatalogEntry>, String> {
        load_catalog(
            &self.vault_path(),
            CatalogKind::Themes,
            &ReqwestFetch,
            parse_theme_catalog,
        )
        .await
    }
}

#[cfg(test)]
#[path = "catalog_tests.rs"]
mod tests;
