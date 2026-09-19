//! # Stratum MCP server
//!
//! Exposes the Stratum knowledge base to external AI clients over the Model
//! Context Protocol (MCP). Implements ADR-0005 and the NORMATIVE contract in
//! `docs/advanced/mcp.md`.
//!
//! This crate provides:
//! - A `KbServer` implementing the MCP `ServerHandler` trait over the existing
//!   `pkm-block` / `pkm-index` / `pkm-markdown` data layer (thin adapters — no
//!   duplicated business logic).
//! - The 15 `kb_*` tools defined by the contract, each with an exact binding
//!   JSON Schema, scope requirement, and error-code mapping.
//! - Two transports: `stdio` (same-host, no auth) and Streamable HTTP
//!   (`/mcp`), plus a `/health` operational endpoint.
//! - PAT bearer-token authentication with per-token scopes, SHA-256 hashed
//!   storage, and a per-token token-bucket rate limiter over HTTP.
//!
//! ## Entry points
//!
//! - [`main`](crate::main): CLI entry point (`--stdio` or `--http`).
//! - [`KbServer::new`]: construct the MCP server handler.
//!
//! ## Architecture
//!
//! The crate is split into focused modules:
//!
//! - `config` — server configuration (vault, transport, auth, limits).
//! - `auth` — PAT parsing/validation, scope enforcement, hashed lookup.
//! - `error` — the MCP/JSON-RPC error vocabulary from contract §8.
//! - `tools` — the 15 tool definitions (schemas + handlers).
//! - `kbserver` — the `ServerHandler` implementation wiring routing, schema
//!   validation, auth, rate limiting, and adapters together.
//! - `http` — Streamable HTTP transport + `/health` endpoint via axum.
//! - `models` — contract DTOs (NoteDocument, IndexStatus, etc.).
//!
//! The write path reuses the exact atomic semantics of the desktop
//! `save_page`: temp-file+rename for the `.md`, then a SQLite transaction, then
//! index refresh, then plugin `onSave`/`onLink` dispatch. A per-vault tokio
//! write mutex serializes writes (single-writer atomicity, ADR-0005 §8/§10).
//!
//! # License
//! AGPL-3.0-only.
#![forbid(unsafe_code)]
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
// `KbError` is the crate-wide error type; boxing every returned error would
// add noise for marginal stack-size wins. Matches the per-crate allow style.
#![allow(clippy::result_large_err)]

pub mod auth;
pub mod config;
pub mod error;
pub mod http;
pub mod kbserver;
pub mod models;
pub mod server;
pub mod tools;

pub use config::McpConfig;
pub use error::{ErrorDataExt, KbError, KbErrorKind};
pub use server::KbServer;

pub const PROTOCOL_VERSION: &str = "2025-06-18";
pub const MCP_RESPONSE_MAX: usize = 1024 * 1024; // 1 MiB per tool response
pub const MCP_BODY_MAX: usize = 2 * 1024 * 1024; // 2 MiB inbound bodies
pub const MCP_PAGE_LIMIT_MAX: usize = 1000;
pub const MCP_PATH_MAX: usize = 4096;
pub const MCP_TOKEN_MAX_SCORE_MATCHES: usize = 5000;

/// Server implementation name/version reported in `initialize`.
pub fn server_name() -> &'static str {
    "stratum-mcp"
}

pub fn server_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
