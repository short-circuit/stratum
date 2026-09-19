//! Error vocabulary for the MCP server (contract §8).
//!
//! Unlike generic crate errors, MCP errors are JSON-RPC 2.0 error objects with
//! a *kind* discriminator in `data.kind`, a human-readable `message`, and an
//! optional `hint`. This module provides the canonical mapping from internal
//! `pkm_core::PkmError` / domain failures into the contract's error codes, and
//! helper constructors used across tool handlers.

use serde_json::{json, Value};
use std::borrow::Cow;
use std::fmt;

use crate::{MCP_PATH_MAX, MCP_RESPONSE_MAX};

/// Canonical `data.kind` values (contract §8.1). Used in `KbErrorKind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KbErrorKind {
    ParseError,
    InvalidRequest,
    MethodNotFound,
    InvalidArgs,
    Internal,
    External,
    NotFound,
    Conflict,
    RateLimited,
    Ssid,
    ScopesDenied,
    VaultLocked,
}

impl KbErrorKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            KbErrorKind::ParseError => "ParseError",
            KbErrorKind::InvalidRequest => "InvalidRequest",
            KbErrorKind::MethodNotFound => "MethodNotFound",
            KbErrorKind::InvalidArgs => "InvalidArgs",
            KbErrorKind::Internal => "Internal",
            KbErrorKind::External => "External",
            KbErrorKind::NotFound => "NotFound",
            KbErrorKind::Conflict => "Conflict",
            KbErrorKind::RateLimited => "RateLimited",
            KbErrorKind::Ssid => "Ssid",
            KbErrorKind::ScopesDenied => "ScopesDenied",
            KbErrorKind::VaultLocked => "VaultLocked",
        }
    }

    /// The JSON-RPC code for this kind (contract §8.1).
    pub fn code(&self) -> i32 {
        match self {
            KbErrorKind::ParseError => -32700,
            KbErrorKind::InvalidRequest => -32600,
            KbErrorKind::MethodNotFound => -32601,
            KbErrorKind::InvalidArgs => -32602,
            KbErrorKind::Internal => -32603,
            KbErrorKind::External => -32000,
            KbErrorKind::NotFound => -32001,
            KbErrorKind::Conflict => -32002,
            KbErrorKind::RateLimited => -32003,
            KbErrorKind::Ssid => -32004,
            KbErrorKind::ScopesDenied => -32005,
            KbErrorKind::VaultLocked => -32006,
        }
    }

    pub fn from_code(code: i32) -> Option<Self> {
        Some(match code {
            -32700 => KbErrorKind::ParseError,
            -32600 => KbErrorKind::InvalidRequest,
            -32601 => KbErrorKind::MethodNotFound,
            -32602 => KbErrorKind::InvalidArgs,
            -32603 => KbErrorKind::Internal,
            -32000 => KbErrorKind::External,
            -32001 => KbErrorKind::NotFound,
            -32002 => KbErrorKind::Conflict,
            -32003 => KbErrorKind::RateLimited,
            -32004 => KbErrorKind::Ssid,
            -32005 => KbErrorKind::ScopesDenied,
            -32006 => KbErrorKind::VaultLocked,
            _ => return None,
        })
    }

    /// The HTTP status conventionally associated with this kind (contract
    /// §8.1). The Streamable HTTP transport maps some JSON-RPC codes to HTTP
    /// statuses itself; this is used by the `/health` endpoint and admin paths
    /// only.
    pub fn http_status(&self) -> u16 {
        match self {
            KbErrorKind::ParseError
            | KbErrorKind::InvalidRequest
            | KbErrorKind::MethodNotFound
            | KbErrorKind::InvalidArgs => 400,
            KbErrorKind::Internal | KbErrorKind::External => 500,
            KbErrorKind::NotFound => 404,
            KbErrorKind::Conflict => 409,
            KbErrorKind::RateLimited => 429,
            KbErrorKind::VaultLocked => 423,
            KbErrorKind::ScopesDenied => 403,
            KbErrorKind::Ssid => 400,
        }
    }
}

/// An MCP error produced on the server side.
///
/// Serializes to a JSON-RPC 2.0 error object with `code`, `message`, and
/// `data` (`data.kind`, optional `note_path`, optional `hint`, optional
/// `retry_after_seconds`).
#[derive(Debug, Clone)]
pub struct KbError {
    pub kind: KbErrorKind,
    pub message: Cow<'static, str>,
    pub note_path: Option<String>,
    pub hint: Option<String>,
    pub retry_after_seconds: Option<u64>,
    /// Optional additional structured details merged into `data`.
    pub extra: Option<Value>,
}

impl KbError {
    pub fn new(kind: KbErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: Cow::Owned(message.into()),
            note_path: None,
            hint: None,
            retry_after_seconds: None,
            extra: None,
        }
    }

    pub fn with_note_path(mut self, path: impl Into<String>) -> Self {
        self.note_path = Some(path.into());
        self
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn with_retry_after(mut self, seconds: u64) -> Self {
        self.retry_after_seconds = Some(seconds);
        self
    }

    /// Construct the JSON-RPC `data` object per §8.
    pub fn to_data(&self) -> Value {
        let mut data = json!({ "kind": self.kind.as_str() });
        if let Some(p) = &self.note_path {
            data["note_path"] = Value::String(p.clone());
        }
        if let Some(h) = &self.hint {
            data["hint"] = Value::String(h.clone());
        }
        if let Some(r) = self.retry_after_seconds {
            data["retry_after_seconds"] = Value::from(r);
        }
        if let Some(Value::Object(map)) = &self.extra {
            for (k, v) in map {
                data[k] = v.clone();
            }
        }
        data
    }

    pub fn code(&self) -> i32 {
        self.kind.code()
    }

    pub fn http_status(&self) -> u16 {
        self.kind.http_status()
    }
}

impl fmt::Display for KbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.kind.as_str(), self.message)
    }
}

impl std::error::Error for KbError {}

impl From<pkm_core::PkmError> for KbError {
    fn from(e: pkm_core::PkmError) -> Self {
        crate::kbserver::map_store_err(e)
    }
}

/// Helper to build common MCP errors.
pub struct ErrorDataExt;

impl ErrorDataExt {
    pub fn not_found(path: impl Into<String>) -> KbError {
        let p = path.into();
        KbError::new(KbErrorKind::NotFound, format!("Note not found: {p}"))
            .with_note_path(p)
            .with_hint("Create it with kb_write_page, or check the path.")
    }

    pub fn conflict(path: impl Into<String>) -> KbError {
        let p = path.into();
        KbError::new(
            KbErrorKind::Conflict,
            format!("Note changed since expected_modified, refusing to overwrite: {p}"),
        )
        .with_note_path(p)
        .with_hint(
            "Re-read the note with kb_get_page and retry with the current modified timestamp.",
        )
    }

    pub fn invalid_args(message: impl Into<String>) -> KbError {
        KbError::new(KbErrorKind::InvalidArgs, message)
    }

    pub fn scopes_denied(required: &str, held: &[String]) -> KbError {
        KbError::new(
            KbErrorKind::ScopesDenied,
            format!(
                "This token lacks the required scope '{required}' (held: {})",
                if held.is_empty() {
                    "none".to_string()
                } else {
                    held.join(", ")
                }
            ),
        )
        .with_hint("Issue a PAT with the needed scope, or grant scope via the operator.")
    }

    pub fn rate_limited(retry_after_seconds: u64) -> KbError {
        KbError::new(
            KbErrorKind::RateLimited,
            "Rate limit exceeded for this token",
        )
        .with_hint("Slow down; retry after the indicated delay.")
        .with_retry_after(retry_after_seconds)
    }

    pub fn vault_locked() -> KbError {
        KbError::new(
            KbErrorKind::VaultLocked,
            "Index rebuild in progress (exclusive lock held)",
        )
        .with_hint("Wait briefly and retry.")
        .with_retry_after(2)
    }

    pub fn external(message: impl Into<String>) -> KbError {
        KbError::new(KbErrorKind::External, message)
    }

    pub fn internal(message: impl Into<String>) -> KbError {
        KbError::new(KbErrorKind::Internal, message)
    }

    pub fn invalid_request(message: impl Into<String>) -> KbError {
        KbError::new(KbErrorKind::InvalidRequest, message)
    }

    pub fn method_not_found(method: impl Into<String>) -> KbError {
        KbError::new(
            KbErrorKind::MethodNotFound,
            format!("Method not found: {}", method.into()),
        )
    }
}

/// Validate a vault-relative path against §6/§11 constraints.
///
/// - Must not be empty.
/// - Length capped at `MCP_PATH_MAX` (4096).
/// - Uses `/` separators (contract: "All `path` values are vault-relative, use
///   `/` separators"). Accepts `\` on Windows as an input convenience but
///   normalizes.
/// - Rejects `..` escaping the root via `resolve_safe_path`-style containment.
pub fn validate_rel_path(path: &str) -> Result<String, KbError> {
    let p = path.trim();
    if p.is_empty() {
        return Err(ErrorDataExt::invalid_args("path must not be empty"));
    }
    if p.len() > MCP_PATH_MAX {
        return Err(ErrorDataExt::invalid_args(format!(
            "path exceeds {MCP_PATH_MAX} characters"
        )));
    }
    let normalized = p.replace('\\', "/");
    // Reject traversal that would escape the vault root.
    for comp in normalized.split('/') {
        if comp == ".." {
            return Err(ErrorDataExt::invalid_args(
                "path must not escape the vault root (no '..' segments)",
            ));
        }
    }
    Ok(normalized)
}

/// Cap a result set to the response payload limit by dropping trailing
/// items and returning how many were dropped (used by pagination-aware tools).
/// Enforces contract §11 (`MCP_RESPONSE_MAX` = 1 MiB; never a hard failure —
/// truncate with a continuation).
pub fn cap_to_response_limit(items: &[Value]) -> (&[Value], usize) {
    // Estimate: each item is at least a few bytes; iterate until the JSON
    // serialization would exceed the cap. For safety we cap at a generous
    // count when items are tiny.
    let mut kept = items.len();
    loop {
        if kept == 0 {
            break;
        }
        let slice = &items[..kept];
        let serialized = serde_json::to_vec(slice)
            .map(|v| v.len())
            .unwrap_or(usize::MAX);
        if serialized <= MCP_RESPONSE_MAX {
            break;
        }
        kept = (kept / 2).max(1);
    }
    (items.split_at(kept).0, items.len() - kept)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_kind_codes() {
        assert_eq!(KbErrorKind::NotFound.code(), -32001);
        assert_eq!(KbErrorKind::Conflict.code(), -32002);
        assert_eq!(KbErrorKind::RateLimited.code(), -32003);
        assert_eq!(KbErrorKind::ScopesDenied.code(), -32005);
        assert_eq!(KbErrorKind::VaultLocked.code(), -32006);
        assert_eq!(KbErrorKind::InvalidArgs.code(), -32602);
        let not_found = KbErrorKind::NotFound;
        assert_eq!(not_found.as_str(), "NotFound");
        assert_eq!(KbErrorKind::from_code(-32001), Some(KbErrorKind::NotFound));
        assert_eq!(KbErrorKind::from_code(1), None);
    }

    #[test]
    fn test_not_found_error_data_shape() {
        let e = ErrorDataExt::not_found("projects/example.md");
        assert_eq!(e.code(), -32001);
        let data = e.to_data();
        assert_eq!(data["kind"], "NotFound");
        assert_eq!(data["note_path"], "projects/example.md");
        assert!(data["hint"].as_str().is_some());
    }

    #[test]
    fn test_path_validation() {
        assert_eq!(validate_rel_path(" a/b.md ").unwrap(), "a/b.md");
        assert_eq!(validate_rel_path("a\\b.md").unwrap(), "a/b.md");
        assert!(validate_rel_path("").is_err());
        assert!(validate_rel_path("../escape.md").is_err());
        let long = format!("{}x", "a".repeat(MCP_PATH_MAX + 1));
        assert!(validate_rel_path(&long).is_err());
    }

    #[test]
    fn test_cap_to_response_limit() {
        let items: Vec<Value> = (0..1_000_000).map(|i| json!({ "i": i })).collect();
        let (kept, dropped) = cap_to_response_limit(&items);
        assert!(kept.len() < items.len());
        assert_eq!(kept.len() + dropped, items.len());
        // The kept slice must serialize within the cap.
        let serialized = serde_json::to_vec(kept).unwrap();
        assert!(serialized.len() <= MCP_RESPONSE_MAX);
    }
}
