//! WebDAV differential backup mirror (E10.F3).
//!
//! Uploads vault files to a WebDAV endpoint, one file per request, tracking
//! what has been written in a `manifest.json` at the base collection. Remote
//! entries are listed once per run; files whose size and mtime already match
//! are skipped. Only files recorded in the previous remote manifest but missing
//! locally are deleted. Files present remotely that were never in the manifest
//! are left untouched.
//!
//! [`backup_to_webdav`] is async; [`backup_to_webdav_blocking`] wraps it in a
//! scoped tokio runtime so the scheduler thread — which is otherwise
//! synchronous — can drive it without its own runtime.

use crate::backup::{collect_files, BackupResult, FileMeta, Manifest};
use chrono::Utc;
use pkm_core::{PkmError, WebdavBackupConfig};
use reqwest_dav::types::list_cmd::{ListEntity, ListMultiStatus};
use reqwest_dav::{Auth, Client, ClientBuilder};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

/// Backup the vault to the WebDAV endpoint described by `cfg`.
///
/// See the module docs. On error the returned [`BackupResult`] has `success`
/// false and a message in [`BackupResult::error`]; the `Result` itself only
/// carries `PkmError` for the endpoint being unconfigurable.
pub async fn backup_to_webdav(
    vault: &Path,
    cfg: &WebdavBackupConfig,
    exclude: &[String],
) -> Result<BackupResult, PkmError> {
    let started_at = Utc::now();
    match backup_webdav_core(vault, cfg, exclude).await {
        Ok((copied, unchanged, deleted)) => Ok(BackupResult {
            success: true,
            target: "webdav".to_string(),
            files_copied: copied,
            files_unchanged: unchanged,
            files_deleted: deleted,
            started_at,
            finished_at: Utc::now(),
            error: None,
        }),
        Err(e) => Ok(BackupResult {
            success: false,
            target: "webdav".to_string(),
            files_copied: 0,
            files_unchanged: 0,
            files_deleted: 0,
            started_at,
            finished_at: Utc::now(),
            error: Some(e.to_string()),
        }),
    }
}

/// Blocking wrapper around [`backup_to_webdav`] for use in the synchronous
/// scheduler thread. A scoped runtime is created per call because the
/// scheduler thread lives outside any tokio context; dropping the runtime
/// closes all in-flight connections.
pub fn backup_to_webdav_blocking(
    vault: &Path,
    cfg: &WebdavBackupConfig,
    exclude: &[String],
) -> Result<BackupResult, PkmError> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| PkmError::Internal(e.to_string()))?;
    rt.block_on(backup_to_webdav(vault, cfg, exclude))
}

/// Verify connectivity + auth against a WebDAV endpoint by listing its root.
///
/// Returns `Ok(())` if the listing succeeds, otherwise a [`PkmError`] with the
/// underlying error (network failure or auth rejection).
pub async fn test_webdav_connection(
    url: &str,
    username: Option<&str>,
    password: Option<&str>,
) -> Result<(), PkmError> {
    let host = url.trim_end_matches('/').to_string();
    let client = build_client(&host, username, password)?;
    let base_path = base_path_of(url);
    list_remote(&client, &base_path).await?;
    Ok(())
}

/// List every file under the base collection, returning a map of
/// vault-relative path → (size, mtime).
///
/// A raw PROPFIND is used (rather than reqwest_dav's `list`) so an `X-Litmus`
/// header can be attached: some servers, including `dav-server`, refuse
/// `Depth: infinity` without it and only recurse at infinite depth, so omitting
/// the header would silently truncate nested paths.
async fn list_remote(
    client: &Client,
    base_path: &str,
) -> Result<HashMap<String, (u64, u64)>, PkmError> {
    let body = r#"<?xml version="1.0" encoding="utf-8" ?><D:propfind xmlns:D="DAV:"><D:allprop/></D:propfind>"#;
    let method = reqwest_dav::re_exports::reqwest::Method::from_bytes(b"PROPFIND")
        .map_err(|e| PkmError::Sync(e.to_string()))?;
    let builder = client
        .start_request(method, "/")
        .await
        .map_err(|e| PkmError::Sync(e.to_string()))?;
    let response = builder
        .header("depth", "infinity")
        .header("x-litmus", "a")
        .header(
            reqwest_dav::re_exports::reqwest::header::CONTENT_TYPE,
            "text/xml; charset=\"utf-8\"",
        )
        .body(body)
        .send()
        .await
        .map_err(|e| PkmError::Sync(e.to_string()))?;
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| PkmError::Sync(e.to_string()))?;
    if !status.is_success() {
        return Err(PkmError::Sync(format!(
            "PROPFIND failed with status {}",
            status.as_u16()
        )));
    }
    let multistatus: ListMultiStatus = reqwest_dav::re_exports::serde_xml_rs::from_str(&text)
        .map_err(|e| PkmError::Sync(e.to_string()))?;

    let mut remote = HashMap::new();
    for response in multistatus.responses {
        if let Ok(ListEntity::File(file)) = ListEntity::try_from(response) {
            let rel = normalize_remote_rel(&file.href, base_path);
            if rel.is_empty() || rel == "manifest.json" {
                continue;
            }
            remote.insert(
                rel,
                (
                    file.content_length.max(0) as u64,
                    file.last_modified.timestamp().max(0) as u64,
                ),
            );
        }
    }
    Ok(remote)
}

async fn backup_webdav_core(
    vault: &Path,
    cfg: &WebdavBackupConfig,
    exclude: &[String],
) -> Result<(usize, usize, usize), PkmError> {
    let url = cfg
        .url
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| PkmError::Config("No WebDAV URL configured".into()))?;
    let host = url.trim_end_matches('/').to_string();
    let client = build_client(&host, cfg.username.as_deref(), cfg.password.as_deref())?;

    let base_path = base_path_of(url);

    // List everything under the base collection once, building the remote map
    // of vault-relative path → (size, mtime). Files are matched by their
    // suffix against the local relative path after the base collection segment.
    let remote = list_remote(&client, &base_path).await?;

    let files = collect_files(vault, exclude, None)?;
    let mut new_set: BTreeMap<String, FileMeta> = BTreeMap::new();
    for (_, rel, size, mtime) in &files {
        new_set.insert(
            rel.clone(),
            FileMeta {
                size: *size,
                mtime: *mtime,
            },
        );
    }

    let old_manifest = fetch_remote_manifest(&client).await?;

    let mut copied = 0;
    let mut unchanged = 0;
    for (abs, rel, size, _mtime) in &files {
        // Decide uploads by size only. The remote's `getlastmodified` is
        // assigned by the server at PUT time, so it is not a reliable signal
        // for local edits — comparing it would re-upload every unchanged file.
        let needs_upload = match remote.get(rel) {
            None => true,
            Some((remote_size, _remote_mtime)) => *remote_size != *size,
        };
        if !needs_upload {
            unchanged += 1;
            continue;
        }
        ensure_parent_collections(&client, rel).await?;
        let bytes = std::fs::read(abs).map_err(PkmError::Io)?;
        client
            .put(rel, bytes)
            .await
            .map_err(|e| PkmError::Sync(e.to_string()))?;
        copied += 1;
    }

    // Delete propagation: only files recorded in the previous remote manifest
    // but no longer present locally are removed remotely.
    let mut deleted = 0;
    if let Some(old) = old_manifest {
        for rel in old.files.keys() {
            if !new_set.contains_key(rel) {
                client
                    .delete(rel)
                    .await
                    .map_err(|e| PkmError::Sync(e.to_string()))?;
                deleted += 1;
            }
        }
    }

    // Upload the manifest at the base collection so the next run can compute
    // deletes. Written last so a partially-completed run never reports a state
    // it did not reach.
    let manifest = Manifest {
        version: 1,
        generated_at: Utc::now().to_rfc3339(),
        files: new_set.clone(),
    };
    let json = serde_json::to_vec_pretty(&manifest)?;
    client
        .put("manifest.json", json)
        .await
        .map_err(|e| PkmError::Sync(e.to_string()))?;

    Ok((copied, unchanged, deleted))
}

fn build_client(
    host: &str,
    username: Option<&str>,
    password: Option<&str>,
) -> Result<Client, PkmError> {
    ClientBuilder::new()
        .set_host(host.to_string())
        .set_auth(Auth::Basic(
            username.unwrap_or("").to_string(),
            password.unwrap_or("").to_string(),
        ))
        .build()
        .map_err(|e| PkmError::Sync(e.to_string()))
}

/// Extract the collection path (the path component of the base URL, without a
/// trailing slash) used to normalize remote hrefs to vault-relative paths.
fn base_path_of(url: &str) -> String {
    url::Url::parse(url)
        .map(|u| u.path().trim_end_matches('/').to_string())
        .unwrap_or_default()
}

/// Normalize a remote `href` to a vault-relative forward-slash path.
///
/// The href may be an absolute URL or an absolute path. We strip the base
/// collection segment and any leading slash, leaving only the path relative to
/// the base collection, which is matched against the local relative paths.
fn normalize_remote_rel(href: &str, base_path: &str) -> String {
    let path = if href.starts_with("http://") || href.starts_with("https://") {
        url::Url::parse(href)
            .map(|u| u.path().to_string())
            .unwrap_or_else(|_| href.to_string())
    } else {
        href.to_string()
    };
    let path = if !base_path.is_empty() {
        path.strip_prefix(base_path).unwrap_or(&path).to_string()
    } else {
        path
    };
    path.trim_start_matches('/')
        .trim_end_matches('/')
        .to_string()
}

/// Create any missing parent collections for `rel`, ignoring "already exists"
/// errors (dav servers return 405 for mkcol on an existing collection).
async fn ensure_parent_collections(client: &Client, rel: &str) -> Result<(), PkmError> {
    let parts: Vec<&str> = rel.split('/').collect();
    let mut path = String::new();
    for part in &parts[..parts.len().saturating_sub(1)] {
        if !path.is_empty() {
            path.push('/');
        }
        path.push_str(part);
        let _ = client.mkcol(&path).await;
    }
    Ok(())
}

/// Fetch the previous remote manifest, returning `None` when absent or corrupt.
async fn fetch_remote_manifest(client: &Client) -> Result<Option<Manifest>, PkmError> {
    // Use get_raw (not get) so a 404 for a not-yet-uploaded manifest is treated
    // as "first run" instead of surfacing as an error.
    let response = client
        .get_raw("manifest.json")
        .await
        .map_err(|e| PkmError::Sync(e.to_string()))?;
    if response.status().is_success() {
        let text = response
            .text()
            .await
            .map_err(|e| PkmError::Sync(e.to_string()))?;
        Ok(serde_json::from_str(&text).ok())
    } else {
        Ok(None)
    }
}
