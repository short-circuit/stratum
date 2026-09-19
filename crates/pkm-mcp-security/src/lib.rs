//! # Stratum MCP server security controls
//!
//! A self-contained library of security controls for the Stratum MCP server
//! (`crates/pkm-mcp`), implemented from the NORMATIVE contract in
//! `docs/advanced/mcp.md` §§6–11 and ADR-0005. The MCP server crate binds
//! these controls to its transport/dispatch layer; this crate deliberately has
//! **no dependency on `crates/pkm-mcp` internals**, so it can be unit-tested,
//! audited, and reused independently of the server's transport scaffolding.
//!
//! ## Controls provided
//!
//! - `pat` — Personal-access-token generation, SHA-256 hashing (tokens are
//!   never stored in plaintext), constant-time verification, and log-safe
//!   subject derivation (contract §7.1, §7.4).
//! - `scope` — the `kb:*` scope vocabulary and exact-match authorization
//!   checks (contract §7.2). Scope checks MUST run before any storage access.
//! - `ratelimit` — per-key token-bucket rate limiter with a `Retry-After`
//!   computation (contract §9).
//! - `path` — vault-root path containment for all user-supplied `path` values;
//!   rejects path traversal before any filesystem access (contract §6).
//! - `input` — inbound request-size caps and payload validation constants
//!   (contract §11).
//! - `output` — response-size capping and log/path sanitization so absolute
//!   filesystem paths and sensitive values never leak (contract §8.2).
//! - `error` — the contract's JSON-RPC error vocabulary (§8.1) as stable
//!   constants plus a crate-local [`SecurityError`].
//!
//! ## Threat model
//!
//! See `docs/development/adr/0006-mcp-server-threat-model.md` for the STRIDE
//! analysis and the vulnerability assessment referenced from this crate.
//!
//! # License
//! AGPL-3.0-only
#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod error;
pub mod input;
pub mod output;
pub mod pat;
pub mod path;
pub mod ratelimit;
pub mod scope;
pub mod write;

pub use error::{MCPErrorCode, SecurityError, SecurityErrorKind};

/// Server implementation name (matches `pkm-mcp` and the contract server
/// identity reported in `initialize`).
pub const SERVER_NAME: &str = "stratum-mcp";
/// MCP protocol version pinned by ADR-0005.
pub const PROTOCOL_VERSION: &str = "2025-06-18";

/// Max inbound request body size, contract §11 (`MCP_BODY_MAX`).
pub const MCP_BODY_MAX: usize = 2 * 1024 * 1024;
/// Max per-tool response size, contract §11 (`MCP_RESPONSE_MAX`).
pub const MCP_RESPONSE_MAX: usize = 1024 * 1024;
/// Max vault-relative path length, contract §11 (`MCP_PATH_MAX`).
pub const MCP_PATH_MAX: usize = 4096;
/// Max page-list limit, contract §11 (`MCP_PAGE_LIMIT_MAX`).
pub const MCP_PAGE_LIMIT_MAX: usize = 1000;
/// Max result-set size a search may score, contract §11.
pub const MCP_TOKEN_MAX_SCORE_MATCHES: usize = 5000;
