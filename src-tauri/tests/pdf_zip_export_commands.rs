//! E10.F6 — Tauri command-layer + core tests for per-page PDF and whole-vault
//! ZIP export.
//!
//! The core functions (`export_pdf_core`, `export_zip_core`) are exercised
//! directly against a real temp vault. `export_zip` is additionally driven
//! through the real Tauri IPC dispatcher using the `tauri::test` mock-app
//! harness (same pattern as `query_export_commands.rs`). The Chromium-backed
//! PDF test runs only when a Chromium binary is available; otherwise it is
//! skipped with a note so CI without Chromium still passes.

mod common;

use app_lib::commands::export::pdf::{export_pdf_core, find_chromium};
use app_lib::commands::export::zip::export_zip_core;
use app_lib::commands::vault::{AppState, VaultState};
use serde_json::json;
use std::fs::File;
use std::io::Read;
use std::sync::Mutex;
use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri::webview::InvokeRequest;
use tauri::WebviewWindow;
use zip::ZipArchive;

/// Build a mock Tauri app whose `VaultState` is backed by `vault`, with the
/// PDF + ZIP export commands registered.
fn build_app(vault: &common::TestVault) -> tauri::App<tauri::test::MockRuntime> {
    let vs = VaultState::new(vault.vault_path.clone());
    mock_builder()
        .manage(Mutex::new(vs) as AppState)
        .invoke_handler(tauri::generate_handler![
            app_lib::commands::export::pdf::export_pdf,
            app_lib::commands::export::zip::export_zip,
        ])
        .build(mock_context(noop_assets()))
        .expect("app build")
}

/// Drive one command invocation through the real IPC dispatcher.
fn invoke(
    webview: &WebviewWindow<tauri::test::MockRuntime>,
    cmd: &str,
    body: serde_json::Value,
) -> Result<serde_json::Value, serde_json::Value> {
    let request = InvokeRequest {
        cmd: cmd.to_string(),
        callback: tauri::ipc::CallbackFn(0),
        error: tauri::ipc::CallbackFn(1),
        url: "tauri://localhost".parse().unwrap(),
        body: tauri::ipc::InvokeBody::Json(body),
        headers: Default::default(),
        invoke_key: tauri::test::INVOKE_KEY.to_string(),
    };
    tauri::test::get_ipc_response(webview, request).map(|b| {
        b.deserialize::<serde_json::Value>()
            .unwrap_or(serde_json::Value::Null)
    })
}

fn webview(app: &tauri::App<tauri::test::MockRuntime>) -> WebviewWindow<tauri::test::MockRuntime> {
    tauri::WebviewWindowBuilder::new(app, "main", Default::default())
        .build()
        .unwrap()
}

fn seed_pdf_vault(tv: &common::TestVault) {
    tv.create_md_file(
        "pages/project.md",
        "---\ntitle: Project\n---\nSome content $x^2$ and math.\n\n```mermaid\ngraph TD\nA-->B\n```\n",
    );
}

#[test]
fn export_pdf_core_creates_valid_pdf() {
    let tv = common::create_test_vault();
    seed_pdf_vault(&tv);

    let out = tv.vault_path.join("export-1.pdf");
    let res = export_pdf_core(&tv.vault_path, "pages/project.md", &out);

    if find_chromium().is_none() {
        eprintln!("skipping: no chromium");
        return;
    }

    let res = res.expect("pdf export must succeed");
    assert!(res.output_path.ends_with("export-1.pdf"));

    let meta = std::fs::metadata(&out).expect("pdf file exists");
    assert!(
        meta.len() > 1000,
        "pdf must be non-trivial, got {} bytes",
        meta.len()
    );
    let bytes = std::fs::read(&out).unwrap();
    assert_eq!(&bytes[..4], b"%PDF", "output must be a PDF");
}

#[test]
fn export_pdf_core_rejects_missing_page() {
    let tv = common::create_test_vault();
    let out = tv.vault_path.join("export-1.pdf");
    let err = export_pdf_core(&tv.vault_path, "pages/does-not-exist.md", &out).unwrap_err();
    assert!(
        err.contains("does-not-exist"),
        "error must name the missing page, got: {err}"
    );
}

#[test]
fn export_zip_core_contains_vault_files_and_manifest() {
    let tv = common::create_test_vault();

    // User data (included).
    tv.create_md_file("pages/a.md", "# A");
    tv.create_md_file("pages/sub/b.md", "# B");
    tv.create_md_file("templates/t.md", "# T");
    tv.create_md_file("assets/img.png", "fake-png-bytes");

    // Excluded: .pkm / .git anywhere, and root export artifacts.
    tv.create_md_file(".pkm/x.db", "db");
    tv.create_md_file(".git/HEAD", "ref");
    tv.create_md_file("export.html", "<html></html>");

    let out = tv.vault_path.join("export.zip");
    let res = export_zip_core(&tv.vault_path, &out).expect("zip export must succeed");

    let file = File::open(&out).unwrap();
    let mut archive = ZipArchive::new(file).unwrap();
    let names: Vec<String> = (0..archive.len())
        .map(|i| archive.name_for_index(i).unwrap().unwrap().into_owned())
        .collect();

    for expected in [
        "manifest.json",
        "pages/a.md",
        "pages/sub/b.md",
        "assets/img.png",
        "templates/t.md",
    ] {
        assert!(
            names.contains(&expected.to_string()),
            "missing {expected} in {names:?}"
        );
    }
    assert!(!names.contains(&".pkm/x.db".to_string()), ".pkm excluded");
    assert!(!names.contains(&".git/HEAD".to_string()), ".git excluded");
    assert!(
        !names.contains(&"export.html".to_string()),
        "root export.html excluded"
    );

    let mut manifest_str = String::new();
    archive
        .by_name("manifest.json")
        .unwrap()
        .read_to_string(&mut manifest_str)
        .unwrap();
    let manifest: serde_json::Value = serde_json::from_str(&manifest_str).unwrap();
    assert_eq!(manifest["format"], "stratum-vault-zip");
    assert_eq!(manifest["version"], "1");
    assert_eq!(manifest["page_count"], 3, "three .md files");
    chrono::DateTime::parse_from_rfc3339(manifest["exported_at"].as_str().unwrap())
        .expect("exported_at must be valid RFC3339");

    assert_eq!(
        res.files_packed,
        archive.len(),
        "files_packed must equal zip entry count"
    );
    assert_eq!(res.page_count, 3);
}

#[test]
fn export_zip_core_roundtrip_contents() {
    let tv = common::create_test_vault();
    let original = b"# A\n\n- hello\n- world\n";
    tv.create_md_file("pages/a.md", std::str::from_utf8(original).unwrap());

    let out = tv.vault_path.join("export.zip");
    export_zip_core(&tv.vault_path, &out).unwrap();

    let file = File::open(&out).unwrap();
    let mut archive = ZipArchive::new(file).unwrap();
    let mut entry = archive.by_name("pages/a.md").unwrap();
    let mut buf = Vec::new();
    entry.read_to_end(&mut buf).unwrap();
    assert_eq!(
        buf, original,
        "round-tripped bytes must match the source file exactly"
    );
}

#[test]
fn export_zip_ipc_returns_output_path() {
    let tv = common::create_test_vault();
    tv.create_md_file("pages/a.md", "# A");
    let app = build_app(&tv);
    let wv = webview(&app);

    let resp = invoke(&wv, "export_zip", json!({})).expect("export_zip must resolve");
    let output_path = resp["output_path"]
        .as_str()
        .expect("output_path must be a string");
    assert!(
        output_path.ends_with("export.zip"),
        "output_path must point at export.zip, got: {output_path}"
    );
    assert!(
        std::path::Path::new(output_path).exists(),
        "zip file must exist on disk"
    );
}

#[test]
fn export_zip_ipc_honors_custom_output_path() {
    let tv = common::create_test_vault();
    tv.create_md_file("pages/a.md", "# A");
    let app = build_app(&tv);
    let wv = webview(&app);

    // Point at a non-existent nested directory so parent-dir creation is
    // exercised end-to-end and the chosen path is provably honored.
    let custom = tv.vault_path.join("nested/backup/export.zip");
    let custom_str = custom.to_string_lossy().to_string();

    let resp = invoke(&wv, "export_zip", json!({ "outputPath": custom_str }))
        .expect("export_zip must resolve");
    let output_path = resp["output_path"]
        .as_str()
        .expect("output_path must be a string");
    assert_eq!(
        output_path, custom_str,
        "output_path must equal the chosen path, got: {output_path}"
    );
    assert!(
        std::path::Path::new(output_path).exists(),
        "zip file must exist at the chosen path"
    );
}
