//! Tests for theme installation, listing, activation and uninstall.
//!
//! Downloads are served by a fake [`CatalogFetch`] so the tests never touch
//! the network; every vault lives in a `tempfile` directory.

use std::future::Future;

use super::*;

/// A theme document with the four required fields.
const THEME_DOC: &str = r##"{"dark_mode":false,"primary_color":"#010203","secondary_color":"#040506","font_size":15.0}"##;

/// Deterministic fetch fake returning fixed bytes or an error.
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
            error: Some("theme_download_error".to_string()),
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

fn entry_for(doc: &[u8]) -> ThemeCatalogEntry {
    ThemeCatalogEntry {
        id: "midnight".to_string(),
        name: "Midnight".to_string(),
        version: "1.0.0".to_string(),
        description: "Test theme".to_string(),
        author: None,
        dark_mode: false,
        primary_color: "#010203".to_string(),
        secondary_color: "#040506".to_string(),
        font_size: 15.0,
        repo: String::new(),
        source: None,
        sha256: sha256_hex(doc),
        download_url: "https://example.test/midnight.json".to_string(),
    }
}

fn block_on<F: Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime")
        .block_on(future)
}

#[test]
fn install_list_apply_uninstall_round_trips() {
    let dir = tempfile::tempdir().expect("tempdir");
    let doc = THEME_DOC.as_bytes();
    let installed = block_on(install_theme_from_catalog(
        dir.path(),
        entry_for(doc),
        &FakeFetch::ok(doc.to_vec()),
    ))
    .expect("install");
    assert!(installed.active);
    assert_eq!(installed.values.font_size, 15.0);

    let listed = list_installed_themes(dir.path());
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, "midnight");
    assert!(listed[0].active);
    assert_eq!(active_theme_id(dir.path()), Some("midnight".to_string()));

    let values = apply_theme(dir.path(), "midnight").expect("apply");
    assert!(!values.dark_mode);
    assert_eq!(values.primary_color, "#010203");

    uninstall_theme(dir.path(), "midnight").expect("uninstall");
    assert!(list_installed_themes(dir.path()).is_empty());
    assert_eq!(active_theme_id(dir.path()), None);
}

#[test]
fn uninstalling_the_active_theme_falls_back_to_built_in() {
    let dir = tempfile::tempdir().expect("tempdir");
    let doc = THEME_DOC.as_bytes();
    block_on(install_theme_from_catalog(
        dir.path(),
        entry_for(doc),
        &FakeFetch::ok(doc.to_vec()),
    ))
    .expect("install");

    uninstall_theme(dir.path(), "midnight").expect("uninstall");
    let values = apply_theme(dir.path(), BUILTIN_THEME_ID).expect("built-in");
    assert_eq!(values, builtin_values());
    assert!(values.dark_mode);
    assert_eq!(values.primary_color, "#f97316");
    assert_eq!(values.secondary_color, "#6b7280");
    assert_eq!(values.font_size, 16.0);
}

#[test]
fn checksum_mismatch_is_rejected() {
    let dir = tempfile::tempdir().expect("tempdir");
    let doc = THEME_DOC.as_bytes();
    let mut entry = entry_for(doc);
    entry.sha256 = "0".repeat(64);
    let error = block_on(install_theme_from_catalog(
        dir.path(),
        entry,
        &FakeFetch::ok(doc.to_vec()),
    ))
    .expect_err("checksum mismatch");
    assert_eq!(error, THEME_CHECKSUM_MISMATCH);
    assert!(list_installed_themes(dir.path()).is_empty());
}

#[test]
fn malformed_theme_document_is_rejected() {
    let dir = tempfile::tempdir().expect("tempdir");
    let doc = br#"{"dark_mode":true}"#;
    let error = block_on(install_theme_from_catalog(
        dir.path(),
        entry_for(doc),
        &FakeFetch::ok(doc.to_vec()),
    ))
    .expect_err("schema invalid");
    assert_eq!(error, THEME_SCHEMA_INVALID);
}

#[test]
fn download_failure_is_reported() {
    let dir = tempfile::tempdir().expect("tempdir");
    let doc = THEME_DOC.as_bytes();
    let error = block_on(install_theme_from_catalog(
        dir.path(),
        entry_for(doc),
        &FakeFetch::failing(),
    ))
    .expect_err("download error");
    assert_eq!(error, THEME_DOWNLOAD_ERROR);
}

#[test]
fn service_methods_see_the_installed_theme() {
    let dir = tempfile::tempdir().expect("tempdir");
    let doc = THEME_DOC.as_bytes();
    block_on(install_theme_from_catalog(
        dir.path(),
        entry_for(doc),
        &FakeFetch::ok(doc.to_vec()),
    ))
    .expect("install");

    let service = StratumService::new(dir.path().to_path_buf());
    let listed = block_on(service.list_installed_themes()).expect("list");
    assert_eq!(listed.len(), 1);
    assert!(listed[0].active);

    let values = block_on(service.apply_theme("midnight".to_string())).expect("apply");
    assert_eq!(values.font_size, 15.0);

    block_on(service.uninstall_theme("midnight".to_string())).expect("uninstall");
    let listed = block_on(service.list_installed_themes()).expect("list");
    assert!(listed.is_empty());
}
