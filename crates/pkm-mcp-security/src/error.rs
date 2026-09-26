//! Error vocabulary (contract §8).
//!
//! The MCP/JSON-RPC error codes in this module are the canonical values from
//! the NORMATIVE contract §8.1. The server wraps its transport errors in these
//! codes; this module exists so the crate's own security-layer errors carry a
//! stable code that maps 1:1 onto the contract vocabulary.

/// Contract JSON-RPC error codes (§8.1).
pub mod mcperror_code {
    /// Malformed JSON (`ParseError`).
    pub const PARSE_ERROR: i32 = -32700;
    /// Invalid JSON-RPC request.
    pub const INVALID_REQUEST: i32 = -32600;
    /// Unknown method / tool.
    pub const METHOD_NOT_FOUND: i32 = -32601;
    /// JSON Schema validation failed (`InvalidArgs`).
    pub const INVALID_ARGS: i32 = -32602;
    /// Unexpected internal error.
    pub const INTERNAL: i32 = -32603;
    /// Downstream store/search error.
    pub const EXTERNAL: i32 = -32000;
    /// Note/path/tag not found.
    pub const NOT_FOUND: i32 = -32001;
    /// Precondition guard failed (`expected_modified`).
    pub const CONFLICT: i32 = -32002;
    /// Rate limit exceeded.
    pub const RATE_LIMITED: i32 = -32003;
    /// SSRF guard denied target (future).
    pub const SSRF: i32 = -32004;
    /// Missing required scope.
    pub const SCOPES_DENIED: i32 = -32005;
    /// Index rebuild exclusive lock held.
    pub const VAULT_LOCKED: i32 = -32006;
}

/// JSON-RPC error-code constants (contract §8).
pub use mcperror_code as MCPErrorCode;

/// Errors raised by the security-control layer itself.
#[derive(Debug, thiserror::Error)]
pub enum SecurityError {
    /// A required scope was not granted.
    #[error("missing required scope `{0}`")]
    ScopeDenied(String),
    /// The rate limit was exceeded; `retry_after_seconds` is the backoff.
    #[error("rate limit exceeded")]
    RateLimited {
        /// Seconds to wait before the next request is admitted.
        retry_after_seconds: u64,
    },
    /// A path escapes the vault root.
    #[error("path escapes vault root: {0}")]
    PathTraversal(String),
    /// A bearer token is missing or malformed.
    #[error("missing or malformed bearer token")]
    MissingToken,
    /// The supplied token did not authenticate.
    #[error("invalid token")]
    InvalidToken,
    /// An input value exceeded a contract size cap.
    #[error("input exceeds limit: {0}")]
    InputTooLarge(String),
    /// An input value was structurally invalid (missing/empty/malformed).
    #[error("invalid input: {0}")]
    InvalidInput(String),
}

/// Machine kind labels mirroring the contract `data.kind` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityErrorKind {
    /// `ScopesDenied`
    ScopeDenied,
    /// `RateLimited`
    RateLimited,
    /// `InvalidArgs` (validation)
    InvalidArgs,
    /// `NotFound`
    NotFound,
}

impl SecurityError {
    /// Map a security error onto its contract JSON-RPC code.
    pub fn code(&self) -> i32 {
        match self {
            SecurityError::ScopeDenied(_) => mcperror_code::SCOPES_DENIED,
            SecurityError::RateLimited { .. } => mcperror_code::RATE_LIMITED,
            SecurityError::PathTraversal(_)
            | SecurityError::InputTooLarge(_)
            | SecurityError::InvalidInput(_) => mcperror_code::INVALID_ARGS,
            SecurityError::MissingToken | SecurityError::InvalidToken => {
                mcperror_code::INVALID_ARGS
            }
        }
    }

    /// The contract `data.kind` string for this error.
    pub fn kind(&self) -> &'static str {
        match self {
            SecurityError::ScopeDenied(_) => "ScopesDenied",
            SecurityError::RateLimited { .. } => "RateLimited",
            SecurityError::PathTraversal(_)
            | SecurityError::InputTooLarge(_)
            | SecurityError::InvalidInput(_) => "InvalidArgs",
            SecurityError::MissingToken | SecurityError::InvalidToken => "InvalidArgs",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_error_codes_are_stable() {
        // These are wire values — pins from contract §8.1.
        assert_eq!(mcperror_code::PARSE_ERROR, -32700);
        assert_eq!(mcperror_code::INVALID_REQUEST, -32600);
        assert_eq!(mcperror_code::METHOD_NOT_FOUND, -32601);
        assert_eq!(mcperror_code::INVALID_ARGS, -32602);
        assert_eq!(mcperror_code::INTERNAL, -32603);
        assert_eq!(mcperror_code::EXTERNAL, -32000);
        assert_eq!(mcperror_code::NOT_FOUND, -32001);
        assert_eq!(mcperror_code::CONFLICT, -32002);
        assert_eq!(mcperror_code::RATE_LIMITED, -32003);
        assert_eq!(mcperror_code::SSRF, -32004);
        assert_eq!(mcperror_code::SCOPES_DENIED, -32005);
        assert_eq!(mcperror_code::VAULT_LOCKED, -32006);
    }

    #[test]
    fn security_error_maps_to_contract_codes() {
        assert_eq!(SecurityError::ScopeDenied("kb:read".into()).code(), -32005);
        assert_eq!(
            SecurityError::RateLimited {
                retry_after_seconds: 1
            }
            .code(),
            -32003
        );
        assert_eq!(SecurityError::PathTraversal("x".into()).code(), -32602);
        assert_eq!(SecurityError::MissingToken.code(), -32602);
    }

    #[test]
    fn kind_labels_match_contract() {
        assert_eq!(
            SecurityError::ScopeDenied("s".into()).kind(),
            "ScopesDenied"
        );
        assert_eq!(
            SecurityError::RateLimited {
                retry_after_seconds: 1
            }
            .kind(),
            "RateLimited"
        );
    }
}
