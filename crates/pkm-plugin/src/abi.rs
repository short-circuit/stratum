//! Frozen host ABI types shared between the plugin runtime and the host.
//!
//! This module is the normative reference implementation of the "Host
//! Function ABI" contract in `docs/advanced/plugins.md` §3 and §12 (types
//! defined here are ABI-frozen for the v0.7.x series). The wire formats of
//! every struct in this file are part of the plugin contract; do not change
//! them without a new ADR.
//!
//! The `HostApi` trait is the seam the host (`src-tauri`, CLI) implements to
//! back the `pkm.note_read`, `pkm.note_write` and `pkm.http_request` imports.
//! The runtime dispatches requests/envelopes in [`crate::runtime`].

use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::permissions::Permission;

/// Maximum host payload size in either direction, in bytes (1 MiB).
pub const HOST_PAYLOAD_MAX: usize = 1 << 20;

/// Default HTTP timeout for `pkm.http_request`, in milliseconds (10 s).
pub const HTTP_TIMEOUT_MS_DEFAULT: u64 = 10_000;

/// Maximum length of a `pkm.log` message before it is truncated, in bytes.
pub const LOG_MESSAGE_MAX: usize = 4096;

/// Maximum length of an error message carried inside an envelope.
pub const ERROR_MESSAGE_MAX: usize = 1024;

/// The four host functions exposed to WASM plugins.
///
/// ABI-frozen for the v0.7.x series — do not add or re-order variants without
/// a new ADR. This type is used for dispatch and permission gating only; the
/// canonical import names that the linker registers under the `"pkm"` module
/// namespace are in [`HostFunction::import_name`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HostFunction {
    Log,
    NoteRead,
    NoteWrite,
    HttpRequest,
}

impl HostFunction {
    /// The permission required to call this host function.
    ///
    /// Permissions per contract §5.2: `file:read` for `note_read`, `file:write`
    /// for `note_write`, `network` for `http_request`. `log` requires none and
    /// is therefore represented as [`Permission::All`] (always granted, since
    /// `All` implies every capability).
    pub fn required_permission(&self) -> Permission {
        match self {
            Self::Log => Permission::All,
            Self::NoteRead => Permission::FileRead,
            Self::NoteWrite => Permission::FileWrite,
            Self::HttpRequest => Permission::Network,
        }
    }
}

/// Canonical machine-readable error codes surfaced to plugins.
///
/// Serialized snake_case. ABI-frozen for the v0.7.x series.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginErrorCode {
    /// Permission not granted for the host function.
    PluginDenied,
    /// No plugin with this id is loaded.
    PluginNotFound,
    /// Manifest/compile/instantiation failure.
    PluginLoadError,
    /// Guest or host trap / runtime failure.
    PluginRuntimeError,
    /// note_read: target missing.
    NoteNotFound,
    /// note_write: I/O or traversal failure.
    NoteWriteFailed,
    /// http: dns/connect/tls/abort.
    HttpTransport,
    /// http: deadline exceeded.
    HttpTimeout,
    /// http: blocked by SSRF guard.
    HttpSsid,
    /// http: HTTP status 400..=599.
    HttpStatus,
    /// Malformed request payload.
    InvalidArgument,
    /// Manifest/config invalid.
    ConfigError,
    /// Unexpected host error.
    Internal,
}

impl PluginErrorCode {
    /// Human-readable label used in log lines and error messages.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PluginDenied => "plugin_denied",
            Self::PluginNotFound => "plugin_not_found",
            Self::PluginLoadError => "plugin_load_error",
            Self::PluginRuntimeError => "plugin_runtime_error",
            Self::NoteNotFound => "note_not_found",
            Self::NoteWriteFailed => "note_write_failed",
            Self::HttpTransport => "http_transport",
            Self::HttpTimeout => "http_timeout",
            Self::HttpSsid => "http_ssid",
            Self::HttpStatus => "http_status",
            Self::InvalidArgument => "invalid_argument",
            Self::ConfigError => "config_error",
            Self::Internal => "internal",
        }
    }
}

/// Success envelope every host callback (except `pkm.http_request`) returns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum HostResponse {
    #[serde(rename_all = "snake_case")]
    Ok { data: serde_json::Value },
    #[serde(rename_all = "snake_case")]
    Err {
        code: PluginErrorCode,
        message: String,
    },
}

impl HostResponse {
    /// Build a success envelope.
    pub fn ok(data: impl Serialize) -> Self {
        Self::Ok {
            data: serde_json::to_value(data).unwrap_or(serde_json::Value::Null),
        }
    }

    /// Build an error envelope with a `[stratum]`-prefixed message.
    pub fn err(code: PluginErrorCode, message: impl Into<String>) -> Self {
        Self::Err {
            code,
            message: Self::sanitize_message(message.into()),
        }
    }

    /// Truncate a message to [`ERROR_MESSAGE_MAX`] characters.
    fn sanitize_message(mut message: String) -> String {
        if message.len() > ERROR_MESSAGE_MAX {
            message.truncate(ERROR_MESSAGE_MAX);
        }
        message
    }
}

/// Dedicated envelope returned by `pkm.http_request`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum HttpResponseEnvelope {
    #[serde(rename_all = "snake_case")]
    Ok {
        status: u16,
        headers: Vec<(String, String)>,
        body: String,
    },
    #[serde(rename_all = "snake_case")]
    Err {
        code: PluginErrorCode,
        message: String,
    },
}

/// A note as resolved by the host backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostNote {
    /// Vault-relative path that was read (normalized).
    pub path: String,
    /// Full text content (frontmatter + body, exactly as on disk).
    pub content: String,
    /// RFC 3339 UTC `modified` timestamp, or empty string if unavailable.
    pub mtime: String,
}

/// A parsed `pkm.http_request` request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostHttpRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
    pub timeout_ms: u64,
}

/// A successful HTTP response returned to a plugin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostHttpResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

/// The linker-registered WASM import name for a host function.
///
/// Kept in `abi` so both the runtime and host tooling reference the same
/// strings; these names are ABI-frozen.
impl HostFunction {
    pub fn import_name(&self) -> (&'static str, &'static str) {
        match self {
            Self::Log => ("pkm", "log"),
            Self::NoteRead => ("pkm", "note_read"),
            Self::NoteWrite => ("pkm", "note_write"),
            Self::HttpRequest => ("pkm", "http_request"),
        }
    }
}

/// Parse a lowercase HTTP method token into its canonical uppercase form.
///
/// Returns `None` for anything outside the allowed set.
pub fn normalize_http_method(method: &str) -> Option<&'static str> {
    match method.trim().to_ascii_uppercase().as_str() {
        "GET" => Some("GET"),
        "POST" => Some("POST"),
        "PUT" => Some("PUT"),
        "PATCH" => Some("PATCH"),
        "DELETE" => Some("DELETE"),
        "HEAD" => Some("HEAD"),
        "OPTIONS" => Some("OPTIONS"),
        _ => None,
    }
}

/// Whether the given HTTP status is a "success" per the contract.
///
/// `2xx` and `3xx` are success; `400..=599` is an error surfaced to the plugin
/// as `PluginErrorCode::HttpStatus`. Other statuses (e.g. `1xx`) fall through
/// to the transport path.
pub fn is_http_success(status: u16) -> bool {
    (200..400).contains(&status)
}

/// Parse a `timeout_ms` request field per the contract.
///
/// * Missing/`null` ⇒ default [`HTTP_TIMEOUT_MS_DEFAULT`].
/// * Out of `[1, 60000]` ⇒ `Err(PluginErrorCode::InvalidArgument)`.
pub fn parse_timeout_ms(timeout_ms: Option<u64>) -> Result<u64, PluginErrorCode> {
    match timeout_ms {
        None => Ok(HTTP_TIMEOUT_MS_DEFAULT),
        Some(ms) if (1..=60_000).contains(&ms) => Ok(ms),
        Some(_) => Err(PluginErrorCode::InvalidArgument),
    }
}

/// Parse and validate an absolute http(s) URL.
///
/// * Missing scheme or non-http(s) scheme ⇒ `InvalidArgument`.
/// * Host required (a URL like `http://` has no host) ⇒ `InvalidArgument`.
pub fn parse_http_url(url: &str) -> Result<::url::Url, PluginErrorCode> {
    let parsed = ::url::Url::parse(url).map_err(|_| PluginErrorCode::InvalidArgument)?;
    match parsed.scheme() {
        "http" | "https" => {}
        _ => return Err(PluginErrorCode::InvalidArgument),
    }
    if parsed.host_str().is_none() {
        return Err(PluginErrorCode::InvalidArgument);
    }
    Ok(parsed)
}

/// The notes/filesystem + network backend the plugin runtime talks to.
///
/// Implemented by the host and injected into the runtime via
/// [`crate::runtime::PluginRuntime::with_host`]. Methods are async per the
/// normative ADR §12 reference; the runtime drives a `tokio` runtime for
/// synchronous dispatch into WASM.
///
/// Implementations MUST enforce the permission/SSRF constraints described in
/// `docs/advanced/plugins.md` §4 (the runtime gates on permissions before
/// calling in; backends remain responsible for path-safety and SSRF).
#[async_trait::async_trait]
pub trait HostApi: Send + Sync {
    async fn read_note(&self, rel_path: &str) -> Result<HostNote, PluginErrorCode>;
    async fn write_note(&self, rel_path: &str, content: &str) -> Result<(), PluginErrorCode>;
    async fn http_request(&self, req: HostHttpRequest)
        -> Result<HostHttpResponse, PluginErrorCode>;
}

/// A no-op host backend. Every call yields a [`PluginErrorCode::PluginRuntimeError`],
/// used to drive the runtime in tests without a host.
pub struct NoopHost;

#[async_trait::async_trait]
impl HostApi for NoopHost {
    async fn read_note(&self, _rel_path: &str) -> Result<HostNote, PluginErrorCode> {
        Err(PluginErrorCode::PluginRuntimeError)
    }

    async fn write_note(&self, _rel_path: &str, _content: &str) -> Result<(), PluginErrorCode> {
        Err(PluginErrorCode::PluginRuntimeError)
    }

    async fn http_request(
        &self,
        _req: HostHttpRequest,
    ) -> Result<HostHttpResponse, PluginErrorCode> {
        Err(PluginErrorCode::PluginRuntimeError)
    }
}

/// Convert a parsed `pkm.http_request` request into a validated
/// [`HostHttpRequest`], mapping contract violations to
/// `PluginErrorCode::InvalidArgument`.
pub fn to_host_request(
    method: Option<String>,
    url: &str,
    headers: Option<serde_json::Map<String, serde_json::Value>>,
    body: Option<String>,
    timeout_ms: Option<u64>,
) -> Result<HostHttpRequest, PluginErrorCode> {
    let method = match method {
        Some(m) => normalize_http_method(&m)
            .ok_or(PluginErrorCode::InvalidArgument)?
            .to_string(),
        None => "GET".to_string(),
    };
    let parsed = parse_http_url(url)?;
    let timeout_ms = parse_timeout_ms(timeout_ms)?;
    let headers = match headers {
        Some(map) => map
            .into_iter()
            .filter_map(|(k, v)| v.as_str().map(|s| (k, s.to_string())))
            .collect(),
        None => Vec::new(),
    };
    Ok(HostHttpRequest {
        method,
        url: parsed.to_string(),
        headers,
        body,
        timeout_ms,
    })
}

/// Convenience for reading the `path` field of a raw JSON request without a
/// full typed deserialization, used in error paths.
pub fn extract_path(json: &serde_json::Value) -> Option<&str> {
    json.get("path").and_then(|v| v.as_str())
}

impl FromStr for PluginErrorCode {
    type Err = ();

    /// Parse an error code from its serialized snake_case form.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "plugin_denied" => Ok(Self::PluginDenied),
            "plugin_not_found" => Ok(Self::PluginNotFound),
            "plugin_load_error" => Ok(Self::PluginLoadError),
            "plugin_runtime_error" => Ok(Self::PluginRuntimeError),
            "note_not_found" => Ok(Self::NoteNotFound),
            "note_write_failed" => Ok(Self::NoteWriteFailed),
            "http_transport" => Ok(Self::HttpTransport),
            "http_timeout" => Ok(Self::HttpTimeout),
            "http_ssid" => Ok(Self::HttpSsid),
            "http_status" => Ok(Self::HttpStatus),
            "invalid_argument" => Ok(Self::InvalidArgument),
            "config_error" => Ok(Self::ConfigError),
            "internal" => Ok(Self::Internal),
            _ => Err(()),
        }
    }
}
