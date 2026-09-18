//! Real host backends for the plugin runtime.
//!
//! Implements the `[`crate::abi::HostApi`]` contract from
//! `docs/advanced/plugins.md` §4:
//!
//! * `note_read`: reads the raw vault-relative note with a path-traversal
//!   guard (mirrors `resolve_safe_path` in `src-tauri/src/commands/page.rs`).
//! * `note_write`: writes the note to disk, creating parent directories as
//!   needed and performing the write atomically; failures propagate.
//! * `http_request`: async HTTP with an enforced timeout, an SSRF guard
//!   constrained to the vault's `network.allowlist`, and contract error
//!   mapping (`http_transport`, `http_timeout`, `http_ssid`, `http_status`,
//!   `invalid_argument`).

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, ToSocketAddrs};
use std::path::{Path, PathBuf};

use pkm_core::PkmResult;
use reqwest::Method;
use tokio::time::timeout;

use crate::abi::{HostHttpRequest, HostHttpResponse, HostNote, PluginErrorCode};

/// Unified `HostApi` implementation backed by a real vault directory.
///
/// This is the host seam used by `src-tauri`/CLI and by the runtime's tests.
/// It holds the canonical vault root and the SSRF allowlist strings from
/// `Config.network.allowlist`.
pub struct VaultHost {
    vault_root: PathBuf,
    allowlist: Vec<AllowlistEntry>,
    http_client: reqwest::Client,
}

/// An allowlist entry: either an exact host name or a CIDR range.
#[derive(Debug, Clone, PartialEq, Eq)]
enum AllowlistEntry {
    Host(String),
    Cidr(ipnet::IpNet),
}

impl AllowlistEntry {
    fn parse(raw: &str) -> Option<Self> {
        let raw = raw.trim();
        if raw.is_empty() {
            return None;
        }
        // A CIDR entry contains a '/'.
        if raw.contains('/') {
            raw.parse::<ipnet::IpNet>().ok().map(Self::Cidr)
        } else {
            Some(Self::Host(raw.to_ascii_lowercase()))
        }
    }

    fn matches_host(&self, host: &str, resolved: &[IpAddr]) -> bool {
        match self {
            Self::Host(h) => host.eq_ignore_ascii_case(h),
            Self::Cidr(net) => resolved.iter().any(|ip| net.contains(ip)),
        }
    }
}

impl VaultHost {
    /// Create a host backend rooted at `vault_root` with the given allowlist.
    pub fn new(vault_root: PathBuf, allowlist: Vec<String>) -> Self {
        let entries = allowlist
            .iter()
            .filter_map(|s| AllowlistEntry::parse(s))
            .collect();
        Self {
            vault_root,
            allowlist: entries,
            http_client: reqwest::Client::new(),
        }
    }

    /// Create a host backend with an empty SSRF allowlist (private/loopback only).
    pub fn root(vault_root: PathBuf) -> Self {
        Self::new(vault_root, Vec::new())
    }

    /// Canonical vault root; fails if it does not exist.
    fn canonical_vault(&self) -> PkmResult<PathBuf> {
        self.vault_root.canonicalize().map_err(|e| {
            pkm_core::PkmError::Io(std::io::Error::new(
                e.kind(),
                format!("Invalid vault path {}: {e}", self.vault_root.display()),
            ))
        })
    }

    /// Resolve a vault-relative path for reading.
    ///
    /// Mirrors `resolve_safe_path`: canonicalizes the result and rejects on
    /// path traversal outside the vault root. The target need not exist for
    /// the caller to receive a path; existence checking is the caller's job.
    fn resolve_read_path(&self, rel_path: &str) -> PkmResult<PathBuf> {
        let canonical_vault = self.canonical_vault()?;
        let full = canonical_vault.join(rel_path);
        let canonical_full = full.canonicalize().map_err(|_| {
            pkm_core::PkmError::NoteNotFound(format!("note does not exist: {}", rel_path))
        })?;
        if !canonical_full.starts_with(&canonical_vault) {
            return Err(pkm_core::PkmError::Plugin(
                "path traversal detected".to_string(),
            ));
        }
        Ok(canonical_full)
    }

    /// Resolve a vault-relative path for writing (target may not exist yet).
    ///
    /// Mirrors `resolve_safe_write_path`: walks up to the nearest existing
    /// ancestor and verifies it is inside the vault, then returns the full
    /// write path.
    fn resolve_write_path(&self, rel_path: &str) -> PkmResult<PathBuf> {
        let canonical_vault = self.canonical_vault()?;
        let full = canonical_vault.join(rel_path);
        let mut check_path = full.as_path();
        loop {
            if check_path.exists() {
                let canonical = check_path.canonicalize().map_err(|e| {
                    pkm_core::PkmError::Plugin(format!(
                        "path resolution failed for {}: {e}",
                        rel_path
                    ))
                })?;
                if !canonical.starts_with(&canonical_vault) {
                    return Err(pkm_core::PkmError::Plugin(
                        "path traversal detected".to_string(),
                    ));
                }
                break;
            }
            match check_path.parent() {
                Some(parent) => check_path = parent,
                None => {
                    return Err(pkm_core::PkmError::Plugin(
                        "path is outside vault".to_string(),
                    ));
                }
            }
        }
        Ok(full)
    }

    /// Read the RFC 3339 UTC modification timestamp of a file, or empty
    /// string if unavailable (matching the contract's `mtime` semantics).
    fn mtime_of(&self, path: &Path) -> String {
        std::fs::metadata(path)
            .and_then(|m| m.modified())
            .map(|t| {
                let dt: chrono::DateTime<chrono::Utc> = t.into();
                dt.to_rfc3339()
            })
            .unwrap_or_default()
    }

    /// Normalize a vault-relative path: strip a leading `/`, collapse and
    /// require a non-empty value.
    fn normalize_rel_path(rel_path: &str) -> Result<String, PluginErrorCode> {
        let trimmed = rel_path.trim().trim_start_matches('/');
        if trimmed.is_empty() {
            return Err(PluginErrorCode::InvalidArgument);
        }
        Ok(trimmed.to_string())
    }

    /// Evaluate the SSRF guard for a parsed URL (contract §10).
    ///
    /// The host is allowed if (a) it matches a `network.allowlist` entry (host
    /// or CIDR), or (b) it resolves to a loopback/private/link-local address.
    /// A hostname that cannot be resolved is rejected with `HttpSsid` (it can
    /// neither match the allowlist by name nor be classified private).
    fn ssrf_allows(&self, parsed: &url::Url) -> Result<bool, PluginErrorCode> {
        let host = parsed.host().ok_or(PluginErrorCode::InvalidArgument)?;
        match host {
            url::Host::Domain(domain) => {
                let addrs = match resolve_host(domain) {
                    Ok(a) => a,
                    Err(_) => return Ok(false),
                };
                Ok(self
                    .allowlist
                    .iter()
                    .any(|e| e.matches_host(domain, &addrs))
                    || addrs.iter().any(is_private_ip))
            }
            url::Host::Ipv4(ip) => {
                let ip = IpAddr::V4(ip);
                Ok(self
                    .allowlist
                    .iter()
                    .any(|e| e.matches_host(&ip.to_string(), std::slice::from_ref(&ip)))
                    || is_private_ip(&ip))
            }
            url::Host::Ipv6(ip) => {
                let ip = IpAddr::V6(ip);
                Ok(self
                    .allowlist
                    .iter()
                    .any(|e| e.matches_host(&ip.to_string(), std::slice::from_ref(&ip)))
                    || is_private_ip(&ip))
            }
        }
    }
}

#[async_trait::async_trait]
impl crate::abi::HostApi for VaultHost {
    async fn read_note(&self, rel_path: &str) -> Result<HostNote, PluginErrorCode> {
        let rel = Self::normalize_rel_path(rel_path)?;
        let full = self.resolve_read_path(&rel).map_err(|e| match e {
            pkm_core::PkmError::NoteNotFound(_) => PluginErrorCode::NoteNotFound,
            _ => PluginErrorCode::PluginRuntimeError,
        })?;
        let content = std::fs::read(&full).map_err(|e| {
            tracing::warn!(
                "[stratum] note_read: failed to read {}: {e}",
                full.display()
            );
            match e.kind() {
                std::io::ErrorKind::NotFound => PluginErrorCode::NoteNotFound,
                _ => PluginErrorCode::PluginRuntimeError,
            }
        })?;
        let content =
            String::from_utf8(content).map_err(|_| PluginErrorCode::PluginRuntimeError)?;
        let mtime = self.mtime_of(&full);
        Ok(HostNote {
            path: rel,
            content,
            mtime,
        })
    }

    async fn write_note(&self, rel_path: &str, content: &str) -> Result<(), PluginErrorCode> {
        let rel = Self::normalize_rel_path(rel_path)?;
        let full = self.resolve_write_path(&rel).map_err(|e| {
            tracing::warn!("[stratum] note_write: path resolution failed: {e}");
            PluginErrorCode::NoteWriteFailed
        })?;
        // Create parent directories as needed.
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                tracing::warn!(
                    "[stratum] note_write: create_dir_all {} failed: {e}",
                    parent.display()
                );
                PluginErrorCode::NoteWriteFailed
            })?;
        }
        // Write atomically: temp file in the same directory, then rename.
        let tmp = full.with_extension(format!("tmp.{}", std::process::id()));
        std::fs::write(&tmp, content.as_bytes()).map_err(|e| {
            tracing::warn!(
                "[stratum] note_write: temp write to {} failed: {e}",
                tmp.display()
            );
            PluginErrorCode::NoteWriteFailed
        })?;
        if let Err(e) = std::fs::rename(&tmp, &full) {
            tracing::warn!(
                "[stratum] note_write: rename {} -> {} failed: {e}",
                tmp.display(),
                full.display()
            );
            // Attempt cleanup of the temp file on failure.
            let _ = std::fs::remove_file(&tmp);
            return Err(PluginErrorCode::NoteWriteFailed);
        }
        Ok(())
    }

    async fn http_request(
        &self,
        req: HostHttpRequest,
    ) -> Result<HostHttpResponse, PluginErrorCode> {
        let parsed = crate::abi::parse_http_url(&req.url)?;
        let timeout_ms = req.timeout_ms.clamp(1, 60_000);
        let method = reqwest_method(&req.method)?;

        // 1. SSRF guard (always on): allow if the host matches the allowlist or
        //    resolves to a loopback/private/link-local address; otherwise reject
        //    without a network round-trip (contract §10, §4.4 rule 1).
        if !self.ssrf_allows(&parsed)? {
            return Err(PluginErrorCode::HttpSsid);
        }

        // Build the request.
        let mut builder = self.http_client.request(method, &req.url);
        for (name, value) in &req.headers {
            builder = builder.header(name, value);
        }
        if let Some(body) = &req.body {
            builder = builder.body(body.clone());
        }

        let result = timeout(std::time::Duration::from_millis(timeout_ms), builder.send()).await;

        let response = match result {
            Ok(Ok(resp)) => resp,
            Ok(Err(_)) => return Err(PluginErrorCode::HttpTransport),
            Err(_) => return Err(PluginErrorCode::HttpTimeout),
        };

        let status = response.status().as_u16();
        let headers: Vec<(String, String)> = response
            .headers()
            .iter()
            .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();

        // Body is read fully, capped at 1 MiB; larger is truncated.
        let body_bytes = response
            .bytes()
            .await
            .map_err(|_| PluginErrorCode::HttpTransport)?;
        let body = String::from_utf8_lossy(
            &body_bytes.as_ref()[..body_bytes.len().min(crate::abi::HOST_PAYLOAD_MAX)],
        )
        .into_owned();

        // 4. HTTP status 400..=599 is surfaced as an error to the plugin.
        if !crate::abi::is_http_success(status) {
            return Err(PluginErrorCode::HttpStatus);
        }

        Ok(HostHttpResponse {
            status,
            headers,
            body,
        })
    }
}

/// Map a canonical method token to a `reqwest::Method`.
fn reqwest_method(method: &str) -> Result<Method, PluginErrorCode> {
    match method {
        "GET" => Ok(Method::GET),
        "POST" => Ok(Method::POST),
        "PUT" => Ok(Method::PUT),
        "PATCH" => Ok(Method::PATCH),
        "DELETE" => Ok(Method::DELETE),
        "HEAD" => Ok(Method::HEAD),
        "OPTIONS" => Ok(Method::OPTIONS),
        _ => Err(PluginErrorCode::InvalidArgument),
    }
}

/// Resolve a hostname to its IP addresses (system resolver).
fn resolve_host(domain: &str) -> std::io::Result<Vec<IpAddr>> {
    let addrs = (domain, 0u16).to_socket_addrs()?;
    let ips: Vec<IpAddr> = addrs.map(|sa| sa.ip()).collect();
    if ips.is_empty() {
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("no addresses for {domain}"),
        ))
    } else {
        Ok(ips)
    }
}

/// Returns `true` if the IP is in a private/loopback/link-local/reserved range
/// spanning both IPv4 and IPv6 (contract §10, matching the ranges enumerated
/// in `pkm_core::validation::validate_endpoint_safe`).
pub(crate) fn is_private_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_private_ipv4(v4),
        IpAddr::V6(v6) => is_private_ipv6(v6),
    }
}

fn is_private_ipv4(ip: &Ipv4Addr) -> bool {
    let octets = ip.octets();
    // 0.0.0.0/8 — current network
    if octets[0] == 0 {
        return true;
    }
    // 10.0.0.0/8 — private
    if octets[0] == 10 {
        return true;
    }
    // 100.64.0.0/10 — CGNAT
    if octets[0] == 100 && (octets[1] & 0xC0) == 64 {
        return true;
    }
    // 127.0.0.0/8 — loopback
    if octets[0] == 127 {
        return true;
    }
    // 169.254.0.0/16 — link-local (includes 169.254.169.254)
    if octets[0] == 169 && octets[1] == 254 {
        return true;
    }
    // 172.16.0.0/12 — private
    if octets[0] == 172 && (octets[1] & 0xF0) == 16 {
        return true;
    }
    // 192.168.0.0/16 — private
    if octets[0] == 192 && octets[1] == 168 {
        return true;
    }
    // 198.18.0.0/15 — benchmarking
    if octets[0] == 198 && (octets[1] & 0xFE) == 18 {
        return true;
    }
    false
}

fn is_private_ipv6(ip: &Ipv6Addr) -> bool {
    let octets = ip.octets();
    // ::1 — loopback
    if *ip == Ipv6Addr::LOCALHOST {
        return true;
    }
    // ::/128 — unspecified
    if ip.is_unspecified() {
        return true;
    }
    // fe80::/10 — link-local
    if octets[0] == 0xFE && (octets[1] & 0xC0) == 0x80 {
        return true;
    }
    // fc00::/7 — unique-local
    if (octets[0] & 0xFE) == 0xFC {
        return true;
    }
    false
}

// ---------------------------------------------------------------------------
// SSRF allowlist
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::abi::HostApi;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    fn rt() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime")
    }

    fn temp_vault() -> tempfile::TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    /// Spawn a tiny HTTP server that responds with the given status/body.
    /// Returns the port it is listening on.
    pub(crate) fn serve(status_line: &'static str, body: &'static str) -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            for stream in listener.incoming().take(2) {
                let Ok(mut stream) = stream else { continue };
                let mut buf = [0u8; 2048];
                let _ = stream.read(&mut buf);
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status_line}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
            }
        });
        port
    }

    // ---- host-level unit tests (no WASM) ----------------------------------

    #[test]
    fn is_private_ip_classifier() {
        // Private / loopback / link-local must be true.
        assert!(is_private_ip(&"127.0.0.1".parse().unwrap()));
        assert!(is_private_ip(&"10.1.2.3".parse().unwrap()));
        assert!(is_private_ip(&"172.16.0.1".parse().unwrap()));
        assert!(is_private_ip(&"192.168.1.1".parse().unwrap()));
        assert!(is_private_ip(&"169.254.169.254".parse().unwrap()));
        assert!(is_private_ip(&"::1".parse().unwrap()));
        assert!(is_private_ip(&"fd00::1".parse().unwrap()));
        assert!(is_private_ip(&"fe80::1".parse().unwrap()));
        // Public addresses must be false.
        assert!(!is_private_ip(&"8.8.8.8".parse().unwrap()));
        assert!(!is_private_ip(&"93.184.216.34".parse().unwrap()));
        assert!(!is_private_ip(&"2606:4700::1111".parse().unwrap()));
    }

    #[test]
    fn note_read_success_returns_real_content() {
        let dir = temp_vault();
        let vault = dir.path();
        std::fs::create_dir_all(vault.join("notes/")).unwrap();
        let target = vault.join("notes/a.md");
        std::fs::write(&target, "---\ntitle: A\n---\nbody content\n").unwrap();

        let host = VaultHost::root(vault.to_path_buf());
        let note = rt().block_on(host.read_note("notes/a.md")).expect("read");

        // No fake note: content is exactly the on-disk bytes.
        assert_eq!(note.path, "notes/a.md");
        assert_eq!(note.content, "---\ntitle: A\n---\nbody content\n");
        assert!(!note.mtime.is_empty());
    }

    #[test]
    fn note_read_missing_yields_note_not_found() {
        let dir = temp_vault();
        let host = VaultHost::root(dir.path().to_path_buf());
        let err = rt()
            .block_on(host.read_note("notes/missing.md"))
            .expect_err("should fail");
        assert_eq!(err, PluginErrorCode::NoteNotFound);
    }

    #[test]
    fn note_read_empty_path_is_invalid_argument() {
        let dir = temp_vault();
        let host = VaultHost::root(dir.path().to_path_buf());
        let err = rt()
            .block_on(host.read_note(""))
            .expect_err("empty path should fail");
        assert_eq!(err, PluginErrorCode::InvalidArgument);
    }

    #[test]
    fn note_read_path_traversal_is_rejected() {
        let dir = temp_vault();
        let vault = dir.path();
        // A file one level above the vault.
        let outside = dir.path().parent().unwrap().join("outside-node.md");
        std::fs::write(&outside, "x").unwrap();

        let host = VaultHost::root(vault.to_path_buf());
        let err = rt()
            .block_on(host.read_note("../outside-node.md"))
            .expect_err("traversal should fail");
        // The traversal escapes the vault: the canonical file is outside the
        // vault root, so this is reported as a runtime error.
        assert_eq!(err, PluginErrorCode::PluginRuntimeError);
        let _ = std::fs::remove_file(&outside);
    }

    #[test]
    fn note_write_creates_file_and_dirs() {
        let dir = temp_vault();
        let vault = dir.path();
        let host = VaultHost::root(vault.to_path_buf());

        rt().block_on(host.write_note("deep/sub/b.md", "hi there"))
            .expect("write");

        let full = vault.join("deep/sub/b.md");
        assert_eq!(std::fs::read_to_string(&full).unwrap(), "hi there");
    }

    #[test]
    fn note_write_failure_propagates() {
        let dir = temp_vault();
        let vault = dir.path();
        let host = VaultHost::root(vault.to_path_buf());

        // A path that traverses outside the vault to a location whose nearest
        // existing ancestor is outside the vault root.
        let err = rt()
            .block_on(host.write_note("../escaped/x.md", "boom"))
            .expect_err("write should fail");
        assert_eq!(err, PluginErrorCode::NoteWriteFailed);
    }

    // ---- HTTP / SSRF tests -------------------------------------------------

    #[test]
    fn http_loopback_allowed_by_default() {
        let body = "{\"hello\":1}";
        let port = serve("200 OK", body);
        let dir = temp_vault();
        let host = VaultHost::root(dir.path().to_path_buf()); // empty allowlist

        let req = HostHttpRequest {
            method: "GET".to_string(),
            url: format!("http://127.0.0.1:{port}/x"),
            headers: vec![],
            body: None,
            timeout_ms: 5_000,
        };
        let resp = rt().block_on(host.http_request(req)).expect("ok");
        assert_eq!(resp.status, 200);
        assert!(resp.body.contains(body));
        assert!(!resp.headers.is_empty());
    }

    #[test]
    fn http_public_host_blocked_without_allowlist() {
        let dir = temp_vault();
        let host = VaultHost::root(dir.path().to_path_buf()); // empty allowlist

        // Literal public IPv4 — no DNS needed; blocked by SSRF guard.
        let req = HostHttpRequest {
            method: "GET".to_string(),
            url: "http://8.8.8.8/".to_string(),
            headers: vec![],
            body: None,
            timeout_ms: 1_000,
        };
        let err = rt().block_on(host.http_request(req)).expect_err("blocked");
        assert_eq!(err, PluginErrorCode::HttpSsid);
    }

    #[test]
    fn http_allowlist_host_allows_public_target() {
        let dir = temp_vault();

        // A public IP is blocked by default (no allowlist, not private).
        let host = VaultHost::new(dir.path().to_path_buf(), vec![]);
        let url = url::Url::parse("http://8.8.8.8/").unwrap();
        assert!(!host.ssrf_allows(&url).unwrap());

        // Adding an exact allowlist entry for it permits it.
        let host = VaultHost::new(dir.path().to_path_buf(), vec!["8.8.8.8".to_string()]);
        assert!(host.ssrf_allows(&url).unwrap());

        // A CIDR entry also permits it.
        let host = VaultHost::new(dir.path().to_path_buf(), vec!["8.8.8.0/24".to_string()]);
        assert!(host.ssrf_allows(&url).unwrap());
    }

    #[test]
    fn http_status_error_maps_to_http_status() {
        let port = serve("404 Not Found", "nope");
        let dir = temp_vault();
        let host = VaultHost::root(dir.path().to_path_buf());

        let req = HostHttpRequest {
            method: "GET".to_string(),
            url: format!("http://127.0.0.1:{port}/missing"),
            headers: vec![],
            body: None,
            timeout_ms: 5_000,
        };
        let err = rt()
            .block_on(host.http_request(req))
            .expect_err("400..=599");
        assert_eq!(err, PluginErrorCode::HttpStatus);
    }

    #[test]
    fn http_transport_error_maps_to_http_transport() {
        // Bind then drop the listener: connection refused on the port.
        let port = {
            let l = TcpListener::bind("127.0.0.1:0").unwrap();
            l.local_addr().unwrap().port()
        };
        let dir = temp_vault();
        let host = VaultHost::root(dir.path().to_path_buf());

        let req = HostHttpRequest {
            method: "GET".to_string(),
            url: format!("http://127.0.0.1:{port}/"),
            headers: vec![],
            body: None,
            timeout_ms: 5_000,
        };
        let err = rt()
            .block_on(host.http_request(req))
            .expect_err("conn refused");
        assert_eq!(err, PluginErrorCode::HttpTransport);
    }

    #[test]
    fn http_timeout_maps_to_http_timeout() {
        // A server that accepts but never responds; the accepted stream is held
        // alive so the connection stays open and the client-side timeout fires.
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            if let Ok((mut _stream, _)) = listener.accept() {
                // Keep the (otherwise unused) stream in scope so it is not
                // dropped, keeping the connection open.
                std::thread::sleep(std::time::Duration::from_secs(5));
                let _ = _stream;
            }
        });
        let dir = temp_vault();
        let host = VaultHost::root(dir.path().to_path_buf());

        let req = HostHttpRequest {
            method: "GET".to_string(),
            url: format!("http://127.0.0.1:{port}/slow"),
            headers: vec![],
            body: None,
            timeout_ms: 100,
        };
        let err = rt().block_on(host.http_request(req)).expect_err("timeout");
        assert_eq!(err, PluginErrorCode::HttpTimeout);
    }

    #[test]
    fn http_invalid_method_is_invalid_argument() {
        let dir = temp_vault();
        let host = VaultHost::root(dir.path().to_path_buf());
        let req = HostHttpRequest {
            method: "TRACE".to_string(),
            url: "http://127.0.0.1:1/".to_string(),
            headers: vec![],
            body: None,
            timeout_ms: 1_000,
        };
        let err = rt()
            .block_on(host.http_request(req))
            .expect_err("bad method");
        assert_eq!(err, PluginErrorCode::InvalidArgument);
    }

    #[test]
    fn ssrf_host_entry_matches_exact_host() {
        let entry = AllowlistEntry::parse("api.example.com").unwrap();
        let addrs = vec!["8.8.8.8".parse::<IpAddr>().unwrap()];
        assert!(entry.matches_host("api.example.com", &addrs));
        // Case-insensitive.
        assert!(entry.matches_host("API.Example.COM", &addrs));
        // Different hostname does not match.
        assert!(!entry.matches_host("other.example.com", &addrs));
    }

    #[test]
    fn ssrf_cidr_entry_matches_subnet() {
        let entry = AllowlistEntry::parse("10.0.0.0/8").unwrap();
        let addrs = vec!["10.20.30.40".parse::<IpAddr>().unwrap()];
        assert!(entry.matches_host("x", &addrs));
        let outside = vec!["11.0.0.1".parse::<IpAddr>().unwrap()];
        assert!(!entry.matches_host("x", &outside));
    }
}
