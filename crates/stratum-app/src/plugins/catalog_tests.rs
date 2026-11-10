//! Tests for the plugin/theme catalog parsing, caching and offline fallback.
//!
//! The network is never touched: [`FakeFetch`] returns fixed bytes or an
//! error, and all disk work happens in a `tempfile` directory.

use std::future::Future;

use super::*;

const SHA: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn plugin_catalog(entries: &str) -> String {
    format!(r#"{{"schema":"stratum-plugins-catalog/v1","plugins":[{entries}]}}"#)
}

fn theme_catalog(entries: &str) -> String {
    format!(r#"{{"schema":"stratum-theme-catalog/v1","themes":[{entries}]}}"#)
}

fn plugin_entry(id: &str) -> String {
    format!(
        r#"{{"id":"{id}","name":"Plugin {id}","version":"1.0.0","download_url":"https://example.test/{id}.wasm","sha256":"{SHA}"}}"#
    )
}

fn theme_entry(id: &str) -> String {
    format!(
        r##"{{"id":"{id}","name":"Theme {id}","version":"1.0.0","dark_mode":true,"primary_color":"#112233","secondary_color":"#445566","font_size":16.0,"download_url":"https://example.test/{id}.json","sha256":"{SHA}"}}"##
    )
}

/// Deterministic fetch fake: returns `bytes` or `error`.
struct FakeFetch {
    bytes: Vec<u8>,
    error: Option<String>,
}

impl FakeFetch {
    fn ok(bytes: Vec<u8>) -> Self {
        Self { bytes, error: None }
    }

    fn failing() -> Self {
        Self {
            bytes: Vec::new(),
            error: Some("catalog_network_error".to_string()),
        }
    }
}

impl CatalogFetch for FakeFetch {
    async fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
        match self.error.clone() {
            Some(error) => Err(error),
            None => Ok(self.bytes.clone()),
        }
    }
}

#[test]
fn parses_a_valid_plugin_catalog() {
    let entries = parse_plugin_catalog(plugin_catalog(&plugin_entry("alpha")).as_bytes())
        .expect("valid catalog");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].id, "alpha");
    assert_eq!(entries[0].sha256, SHA);
}

#[test]
fn parses_a_valid_theme_catalog() {
    let entries = parse_theme_catalog(theme_catalog(&theme_entry("midnight")).as_bytes())
        .expect("valid catalog");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].id, "midnight");
    assert!(entries[0].dark_mode);
}

#[test]
fn rejects_a_wrong_schema() {
    let wrong = r#"{"schema":"stratum-plugins-catalog/v2","plugins":[]}"#;
    assert_eq!(
        parse_plugin_catalog(wrong.as_bytes()),
        Err(CATALOG_SCHEMA_INVALID.to_string())
    );
}

#[test]
fn rejects_non_object_and_empty_entries() {
    assert_eq!(
        parse_plugin_catalog(plugin_catalog("42").as_bytes()),
        Err(CATALOG_SCHEMA_INVALID.to_string())
    );
    let empty_id = r#"{"id":"","name":"n","version":"1","download_url":"u","sha256":"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"}"#;
    assert_eq!(
        parse_plugin_catalog(plugin_catalog(empty_id).as_bytes()),
        Err(CATALOG_SCHEMA_INVALID.to_string())
    );
}

#[test]
fn rejects_a_malformed_sha256() {
    let bad = r#"{"id":"a","name":"n","version":"1","download_url":"u","sha256":"abc"}"#;
    assert_eq!(
        parse_plugin_catalog(plugin_catalog(bad).as_bytes()),
        Err(CATALOG_SCHEMA_INVALID.to_string())
    );
}

#[test]
fn cache_round_trips_and_corrupt_cache_is_absent() {
    let dir = tempfile::tempdir().expect("tempdir");
    let payload = plugin_catalog(&plugin_entry("alpha")).into_bytes();
    write_cache(
        dir.path(),
        CatalogKind::Plugins,
        &payload,
        "2026-11-10T12:00:00Z",
    )
    .expect("write cache");
    let (cached, fetched_at) = read_cache(dir.path(), CatalogKind::Plugins).expect("read cache");
    assert_eq!(fetched_at, "2026-11-10T12:00:00Z");
    assert_eq!(
        parse_plugin_catalog(&cached).expect("cached catalog").len(),
        1
    );

    std::fs::write(dir.path().join(".pkm/catalog/themes.json"), b"not json")
        .expect("write corrupt cache");
    assert_eq!(read_cache(dir.path(), CatalogKind::Themes), None);
}

#[test]
fn successful_fetch_is_cached_and_marked_online() {
    let dir = tempfile::tempdir().expect("tempdir");
    let fetch = FakeFetch::ok(plugin_catalog(&plugin_entry("alpha")).into_bytes());
    let snapshot = block_on(load_catalog(
        dir.path(),
        CatalogKind::Plugins,
        &fetch,
        parse_plugin_catalog,
    ));
    let snapshot = snapshot.expect("online snapshot");
    assert!(!snapshot.offline);
    assert!(snapshot.error.is_none());
    assert_eq!(snapshot.entries.len(), 1);
    assert!(read_cache(dir.path(), CatalogKind::Plugins).is_some());
}

#[test]
fn failed_fetch_falls_back_to_the_cache() {
    let dir = tempfile::tempdir().expect("tempdir");
    let payload = plugin_catalog(&plugin_entry("alpha")).into_bytes();
    write_cache(
        dir.path(),
        CatalogKind::Plugins,
        &payload,
        "2026-11-10T12:00:00Z",
    )
    .expect("write cache");

    let snapshot = block_on(load_catalog(
        dir.path(),
        CatalogKind::Plugins,
        &FakeFetch::failing(),
        parse_plugin_catalog,
    ))
    .expect("offline snapshot");
    assert!(snapshot.offline);
    assert_eq!(snapshot.error, Some(CATALOG_UNAVAILABLE.to_string()));
    assert_eq!(snapshot.entries.len(), 1);
}

#[test]
fn missing_cache_and_failed_fetch_is_an_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let result = block_on(load_catalog(
        dir.path(),
        CatalogKind::Themes,
        &FakeFetch::failing(),
        parse_theme_catalog,
    ));
    assert_eq!(result, Err(CATALOG_UNAVAILABLE.to_string()));
}

/// Block on a future with a current-thread runtime, matching the crate tests.
fn block_on<F: Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime")
        .block_on(future)
}
