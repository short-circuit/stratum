//! Server configuration.
//!
//! The MCP server reads its configuration from environment variables and
//! command-line flags. All mutable operational knobs are overridable per
//! deployment; defaults are chosen for a local, single-tenant instance.
//!
//! # Environment variables
//!
//! | Variable | Default | Meaning |
//! |---|---|---|
//! | `PKM_MCP_VAULT` | (required) | Path to the Stratum vault. |
//! | `PKM_MCP_TOKEN` | (optional) | PAT accepted by the server (plaintext). Multiple tokens are not supported via env. |
//! | `PKM_MCP_TOKEN_FILE` | (optional) | Path to a file containing PATs, one per line (`token[:comma-separated-scopes]`). |
//! | `PKM_MCP_BIND` | `127.0.0.1:8787` | Bind address for the HTTP transport. |
//! | `PKM_MCP_RATE_LIMIT_BURST` | `60` | Token bucket burst. |
//! | `PKM_MCP_RATE_LIMIT_RPS` | `10` | Token bucket refill per second. |
//! | `PKM_MCP_ALLOWED_HOSTS` | `127.0.0.1,localhost` | Allowed `Host` headers (Streamable HTTP). |
//! | `RUST_LOG` | `info` | `tracing` filter. |
//!
//! The file at `vault/.pkm/config.toml` is read if present via
//! `pkm_core::Config` to discover vault metadata; the MCP server additionally
//! requires `blocks.db` under `.pkm/` to be openable.

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Transport to serve.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum Transport {
    #[default]
    Stdio,
    Http,
    Both,
}

impl std::str::FromStr for Transport {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "stdio" => Ok(Transport::Stdio),
            "http" => Ok(Transport::Http),
            "both" => Ok(Transport::Both),
            other => Err(format!(
                "unknown transport: {other} (expected stdio|http|both)"
            )),
        }
    }
}

/// Authentication mode (contract §7.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum AuthMode {
    /// No authentication (stdio / loopback-only).
    #[default]
    None,
    /// PAT bearer tokens (required for remote v0.7.x).
    Pat,
}

/// Validation of the configured vault at startup to fail fast.
pub struct VaultCheck {
    pub vault_path: PathBuf,
    pub db_path: PathBuf,
    pub has_blocks_db: bool,
}

/// Server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    /// Path to the vault root.
    pub vault_path: PathBuf,
    /// Transport to serve.
    pub transport: Transport,
    /// Authentication mode.
    pub auth_mode: AuthMode,
    /// Bind address for HTTP transport.
    pub bind: SocketAddr,
    /// Token bucket burst.
    pub rate_limit_burst: u32,
    /// Token bucket refill per second (as a double).
    pub rate_limit_rps: f64,
    /// Allowed HTTP `Host` headers.
    pub allowed_hosts: Vec<String>,
    /// Read timeout for HTTP transport (sessions).
    pub read_timeout: Duration,
    /// Maximum inbound request body size.
    pub body_max: usize,
    /// Single-writer wait budget before returning `VaultLocked` for rebuilds.
    pub rebuild_wait_timeout: Duration,
}

impl McpConfig {
    pub fn new(vault_path: PathBuf) -> Self {
        Self {
            vault_path,
            transport: Transport::Stdio,
            auth_mode: AuthMode::None,
            bind: "127.0.0.1:8787".parse().expect("valid default bind"),
            rate_limit_burst: 60,
            rate_limit_rps: 10.0,
            allowed_hosts: vec!["127.0.0.1".into(), "localhost".into()],
            read_timeout: Duration::from_secs(60),
            body_max: crate::MCP_BODY_MAX,
            rebuild_wait_timeout: Duration::from_secs(30),
        }
    }

    /// Load configuration from the process environment.
    pub fn from_env() -> anyhow::Result<Self> {
        let vault = std::env::var("PKM_MCP_VAULT")
            .map(PathBuf::from)
            .map_err(|_| anyhow::anyhow!("PKM_MCP_VAULT is required"))?;
        let mut cfg = Self::new(vault);

        if let Ok(t) = std::env::var("PKM_MCP_TRANSPORT") {
            cfg.transport = t
                .parse()
                .map_err(|e: String| anyhow::anyhow!("invalid PKM_MCP_TRANSPORT: {e}"))?;
        }
        if let Ok(b) = std::env::var("PKM_MCP_BIND") {
            cfg.bind = b
                .parse()
                .map_err(|e| anyhow::anyhow!("invalid PKM_MCP_BIND: {e}"))?;
        }
        if let Ok(v) = std::env::var("PKM_MCP_RATE_LIMIT_BURST") {
            cfg.rate_limit_burst = v
                .parse()
                .map_err(|e| anyhow::anyhow!("invalid rate limit burst: {e}"))?;
        }
        if let Ok(v) = std::env::var("PKM_MCP_RATE_LIMIT_RPS") {
            cfg.rate_limit_rps = v
                .parse()
                .map_err(|e| anyhow::anyhow!("invalid rate limit rps: {e}"))?;
        }
        if std::env::var("PKM_MCP_NO_AUTH").is_ok() {
            cfg.auth_mode = AuthMode::None;
        }
        if let Ok(hosts) = std::env::var("PKM_MCP_ALLOWED_HOSTS") {
            cfg.allowed_hosts = hosts.split(',').map(|s| s.trim().to_string()).collect();
        }
        if cfg.transport == Transport::Http || cfg.transport == Transport::Both {
            cfg.auth_mode = AuthMode::Pat;
        }
        Ok(cfg)
    }

    /// Validate the vault exists and contains a `.pkm/blocks.db` (or a
    /// companion marker), failing fast otherwise.
    pub fn validate_vault(&self) -> anyhow::Result<VaultCheck> {
        let vault = &self.vault_path;
        if !vault.is_dir() {
            anyhow::bail!("vault path is not a directory: {}", vault.display());
        }
        let db = vault.join(".pkm").join("blocks.db");
        Ok(VaultCheck {
            vault_path: vault.clone(),
            db_path: db.clone(),
            has_blocks_db: db.exists(),
        })
    }
}

/// Load PATs from an optional token file.
///
/// Format: one token per line. Optionally scoped:
/// `token:scope1,scope2` or `token kb:read,kb:write`.
/// A token without an explicit scope list defaults to the "personal" default
/// scopes (contract §7.2: read/write/index/search/link/organize).
pub fn load_tokens_from_file(path: &Path) -> std::io::Result<Vec<(String, Vec<String>)>> {
    let content = std::fs::read_to_string(path)?;
    let mut out = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((token, scopes)) = line.split_once([':', ' ']) {
            let scopes: Vec<String> = scopes
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            out.push((token.trim().to_string(), scopes));
        } else if !line.is_empty() {
            out.push((line.to_string(), default_personal_scopes()));
        }
    }
    Ok(out)
}

/// The default scope set for a "personal" PAT (contract §7.2).
pub fn default_personal_scopes() -> Vec<String> {
    vec![
        "kb:read".into(),
        "kb:write".into(),
        "kb:index".into(),
        "kb:search".into(),
        "kb:link".into(),
        "kb:organize".into(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_parse() {
        assert_eq!("stdio".parse::<Transport>().unwrap(), Transport::Stdio);
        assert_eq!("HTTP".parse::<Transport>().unwrap(), Transport::Http);
        assert!("bogus".parse::<Transport>().is_err());
    }

    #[test]
    fn test_load_tokens_no_scope_gets_defaults() {
        let dir = tempfile::TempDir::new().unwrap();
        let f = dir.path().join("tokens.txt");
        std::fs::write(&f, "# comment\n\ntoken1\n").unwrap();
        let tokens = load_tokens_from_file(&f).unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].0, "token1");
        assert_eq!(tokens[0].1, default_personal_scopes());
    }

    #[test]
    fn test_load_tokens_scoped() {
        let dir = tempfile::TempDir::new().unwrap();
        let f = dir.path().join("tokens.txt");
        std::fs::write(&f, "tokX:kb:read,kb:write\n").unwrap();
        let tokens = load_tokens_from_file(&f).unwrap();
        assert_eq!(tokens[0].0, "tokX");
        assert_eq!(tokens[0].1, vec!["kb:read", "kb:write"]);
    }
}
