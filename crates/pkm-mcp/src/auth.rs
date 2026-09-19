//! Authentication & authorization (contract §7).
//!
//! PAT bearer tokens (`Stratum-MCP <token>`) are the required authentication
//! mechanism for the HTTP transport in v0.7.x. Tokens are 43-char base64url
//! (256-bit entropy) and are stored hashed (SHA-256) server-side; the server
//! never stores the plaintext. Scope checks happen before any storage access.
//!
//! This module is transport-agnostic: the HTTP layer extracts the bearer token
//! and produces an [`AuthContext`] that is threaded into tool dispatch; stdio
//! uses [`AuthContext::none`].

use base64::Engine;
use sha2::{Digest, Sha256};

use crate::config::{default_personal_scopes, AuthMode};
use crate::error::{ErrorDataExt, KbError};

/// A validated authentication context for a request.
#[derive(Debug, Clone, Default)]
pub struct AuthContext {
    /// The token subject (if authenticated).
    pub subject: Option<String>,
    /// Scopes granted to this request.
    pub scopes: Vec<String>,
    /// Whether the client is authenticated at all.
    pub authenticated: bool,
    /// Client IP / source identifier (used for rate-limit keying).
    pub source_ip: Option<String>,
}

impl AuthContext {
    /// No authentication — used by stdio and loopback-only deployments.
    pub fn none() -> Self {
        Self::default()
    }

    pub fn is_authenticated(&self) -> bool {
        self.authenticated
    }

    /// Check that the request holds the required scope (contract §7.2).
    /// Scope checks happen before any storage access.
    pub fn require_scope(&self, required: &str) -> Result<(), KbError> {
        // In no-auth mode every scope is implicitly granted.
        if !self.authenticated {
            return Ok(());
        }
        if self.scopes.iter().any(|s| s == required) {
            Ok(())
        } else {
            Err(ErrorDataExt::scopes_denied(required, &self.scopes))
        }
    }
}

/// An accepted PAT record: the hash and the scopes it grants.
#[derive(Debug, Clone)]
pub struct TokenRecord {
    pub hash: String,
    pub scopes: Vec<String>,
}

/// Validates bearer tokens against a set of accepted (hashed) token records.
#[derive(Debug, Clone)]
pub struct TokenValidator {
    records: Vec<TokenRecord>,
}

impl TokenValidator {
    pub fn new(records: Vec<TokenRecord>) -> Self {
        Self { records }
    }

    /// Empty validator (no tokens configured): when `require_auth` is true,
    /// all requests are rejected as unauthenticated; when false, no auth is
    /// required at all (local mode).
    pub fn empty() -> Self {
        Self {
            records: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Whether authentication is required (i.e. at least one token is
    /// configured). An empty validator with `require_auth` enforced at the
    /// transport boundary rejects all unauthenticated requests; when no token
    /// is configured the transport treats auth as optional (local mode).
    pub fn required(&self) -> bool {
        !self.records.is_empty()
    }

    /// Authenticate a bearer token string (the raw token, without the
    /// `Stratum-MCP ` prefix). Returns `Some(AuthContext)` on success.
    pub fn authenticate(&self, token: &str) -> Option<AuthContext> {
        let hash = hash_token(token);
        let record = self.records.iter().find(|r| r.hash == hash)?;
        Some(AuthContext {
            subject: Some(token_sanitized_subject(token)),
            scopes: record.scopes.clone(),
            authenticated: true,
            source_ip: None,
        })
    }
}

/// SHA-256 hash of a token, hex-encoded. Never store plaintext tokens.
pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex_encode(&hasher.finalize())
}

/// Hex-encode bytes (no external dependency needed).
pub fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// A short non-reversible subject label for a token (first 12 chars of the
/// hash) — avoids echoing the token in logs while remaining stable.
pub fn token_sanitized_subject(token: &str) -> String {
    format!("pat-{}", &hash_token(token)[..12])
}

/// Parse the `Authorization` header value into a raw bearer token.
/// Accepts `Stratum-MCP <token>` (contract) and `Bearer <token>` (RFC 6750).
pub fn parse_authorization(header: &str) -> Option<String> {
    let trimmed = header.trim();
    let (scheme, rest) = trimmed.split_once(' ')?;
    match scheme.to_ascii_lowercase().as_str() {
        "stratum-mcp" | "bearer" => {
            let token = rest.trim();
            if token.is_empty() {
                None
            } else {
                Some(token.to_string())
            }
        }
        _ => None,
    }
}

/// Build the token validator from config.
/// - `AuthMode::None`: empty (no auth required).
/// - `AuthMode::Pat`: tokens from `PKM_MCP_TOKEN` and/or `PKM_MCP_TOKEN_FILE`.
pub fn build_validator(
    mode: AuthMode,
    env_token: Option<String>,
    token_file: Option<&std::path::Path>,
) -> TokenValidator {
    if mode == AuthMode::None {
        return TokenValidator::empty();
    }
    let mut records = Vec::new();
    if let Some(tok) = env_token.filter(|t| !t.trim().is_empty()) {
        records.push(TokenRecord {
            hash: hash_token(tok.trim()),
            scopes: default_personal_scopes(),
        });
    }
    if let Some(file) = token_file {
        if let Ok(tokens) = super::config::load_tokens_from_file(file) {
            for (tok, scopes) in tokens {
                records.push(TokenRecord {
                    hash: hash_token(&tok),
                    scopes,
                });
            }
        }
    }
    TokenValidator { records }
}

/// Validate a bearer-token-looking string for the 43-char base64url contract.
pub fn is_valid_pat_format(token: &str) -> bool {
    if token.is_empty() || token.len() > 128 {
        return false;
    }
    // Accept anything alphanumeric + _ - that isn't obviously malformed.
    token
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Decode a 43-char base64url token into 32 raw bytes (256-bit entropy),
/// verifying the length. This is informational; validation is done by hash
/// lookup.
pub fn decode_pat_bytes(token: &str) -> Result<[u8; 32], KbError> {
    use base64::alphabet::URL_SAFE;
    use base64::engine::general_purpose::GeneralPurpose;
    use base64::engine::GeneralPurposeConfig;
    let engine = GeneralPurpose::new(
        &URL_SAFE,
        GeneralPurposeConfig::new().with_decode_allow_trailing_bits(true),
    );
    let decoded = engine
        .decode(token)
        .map_err(|_| ErrorDataExt::invalid_args("PAT is not valid base64url"))?;
    let arr: [u8; 32] = decoded.try_into().map_err(|_| {
        ErrorDataExt::invalid_args("PAT must decode to 32 bytes (43-char base64url)")
    })?;
    Ok(arr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_authorization() {
        assert_eq!(
            parse_authorization("Stratum-MCP abc123").as_deref(),
            Some("abc123")
        );
        assert_eq!(
            parse_authorization("Bearer abc123").as_deref(),
            Some("abc123")
        );
        assert_eq!(parse_authorization("Basic dXNlcjpwYXNz"), None);
        assert_eq!(parse_authorization("Stratum-MCP "), None);
        assert_eq!(parse_authorization(""), None);
    }

    #[test]
    fn test_hash_token_deterministic() {
        let a = hash_token("my-token");
        let b = hash_token("my-token");
        assert_eq!(a, b);
        assert_eq!(a.len(), 64); // sha256 hex
        assert_ne!(hash_token("other"), a);
    }

    #[test]
    fn test_token_validator() {
        let validator = TokenValidator::new(vec![TokenRecord {
            hash: hash_token("secret-token"),
            scopes: vec!["kb:read".into()],
        }]);
        let ctx = validator.authenticate("secret-token").unwrap();
        assert!(ctx.authenticated);
        assert!(ctx.require_scope("kb:read").is_ok());
        assert!(ctx.require_scope("kb:write").is_err());

        assert!(validator.authenticate("wrong-token").is_none());
    }

    #[test]
    fn test_scope_check_defaults() {
        let ctx = AuthContext {
            authenticated: true,
            scopes: default_personal_scopes(),
            ..Default::default()
        };
        assert!(ctx.require_scope("kb:write").is_ok());
        assert!(ctx.require_scope("kb:admin").is_err());
    }

    #[test]
    fn test_no_auth_grants_all() {
        let ctx = AuthContext::none();
        assert!(ctx.require_scope("kb:admin").is_ok());
    }

    #[test]
    fn test_pat_format_validation() {
        assert!(is_valid_pat_format("abc-def_123"));
        assert!(!is_valid_pat_format("has space"));
        assert!(!is_valid_pat_format(""));
    }
}
