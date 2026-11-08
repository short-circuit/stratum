//! Headless tests for the cloud/folder backup service commands:
//! `run_backup`, `get_backup_status`, and the standalone `test_webdav` probe.

mod common;

use common::{create_test_vault, temp_dir};

/// A valid PROPFIND `<multistatus>` body with one collection response, matching
/// the shape `reqwest_dav` parses.
const MULTISTATUS: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<D:multistatus xmlns:D="DAV:">
  <D:response>
    <D:href>/</D:href>
    <D:propstat>
      <D:status>HTTP/1.1 200 OK</D:status>
      <D:prop>
        <D:getlastmodified>Wed, 10 Apr 2019 14:00:00 GMT</D:getlastmodified>
        <D:resourcetype><D:collection/></D:resourcetype>
        <D:getcontenttype>httpd/unix-directory</D:getcontenttype>
      </D:prop>
    </D:propstat>
  </D:response>
</D:multistatus>"#;

#[tokio::test]
async fn run_folder_backup_copies_files_and_records_status() {
    let vault = create_test_vault();
    let target = temp_dir();
    vault.write_file("pages/note.md", "# note\n");

    let mut config = vault.config();
    config.sync.backup.enabled = true;
    config.sync.backup.mode = pkm_core::BackupMode::Folder;
    config.sync.backup.folder.path = Some(target.path().to_string_lossy().to_string());
    vault.save_config(&config);

    let status = vault.service.run_backup().await.unwrap();

    assert_eq!(
        status.last_success,
        Some(true),
        "backup must succeed: {status:?}"
    );
    assert_eq!(status.mode, "folder");
    assert!(
        status.files_copied.unwrap_or(0) >= 1,
        "at least the note must be copied: {status:?}"
    );
    assert!(
        target.path().join("pages/note.md").exists(),
        "the mirrored note must exist at the backup target"
    );

    let read_back = vault.service.get_backup_status().await.unwrap();
    assert!(read_back.last_run.is_some(), "ledger must persist last_run");
    assert_eq!(read_back.last_success, Some(true));
}

#[tokio::test]
async fn get_backup_status_defaults_before_any_run() {
    let vault = create_test_vault();

    let status = vault.service.get_backup_status().await.unwrap();

    assert_eq!(status.mode, "folder");
    assert!(status.last_run.is_none());
    assert!(status.last_success.is_none());
}

#[tokio::test]
async fn run_backup_fails_cleanly_when_folder_target_is_missing() {
    let vault = create_test_vault();

    let mut config = vault.config();
    config.sync.backup.enabled = true;
    config.sync.backup.mode = pkm_core::BackupMode::Folder;
    // Deliberately no folder path.
    config.sync.backup.folder.path = None;
    vault.save_config(&config);

    let status = vault.service.run_backup().await.unwrap();

    assert_eq!(status.last_success, Some(false));
    assert!(status.last_error.is_some(), "failure must be reported");
}

#[tokio::test]
async fn test_webdav_returns_ok_against_a_propfind_capable_endpoint() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, ResponseTemplate};

    let server = wiremock::MockServer::start().await;
    Mock::given(method("PROPFIND"))
        .and(path("/"))
        .respond_with(
            ResponseTemplate::new(207)
                .insert_header("content-type", "application/xml")
                .set_body_string(MULTISTATUS),
        )
        .mount(&server)
        .await;

    let result =
        stratum_app::backup::test_webdav(server.uri(), Some("user".into()), Some("pass".into()))
            .await;

    assert_eq!(result.unwrap(), "ok");
}

#[tokio::test]
async fn test_webdav_surfaces_http_failure() {
    use wiremock::matchers::method;
    use wiremock::{Mock, ResponseTemplate};

    let server = wiremock::MockServer::start().await;
    Mock::given(method("PROPFIND"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;

    let err = stratum_app::backup::test_webdav(server.uri(), None, None)
        .await
        .unwrap_err();

    assert!(err.contains("401"), "error must mention the status: {err}");
}
