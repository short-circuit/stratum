//! `pkm-mcp` binary entry point.
//!
//! Launches the Stratum MCP server over one of two transports:
//! - `--stdio` (default): line-delimited JSON-RPC 2.0 over stdin/stdout, for
//!   same-host single-client use (Claude Desktop, etc.).
//! - `--http`: Streamable HTTP server at `<bind>/mcp` plus `/health`.
//!
//! Configuration comes from environment variables (see `crate::config`) and
//! the CLI (`--vault`, `--transport`, `--bind`).
//!
//! # License
//! AGPL-3.0-only.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;
use clap::Parser;
use tracing_subscriber::EnvFilter;

use pkm_mcp::config::{AuthMode, McpConfig, Transport};
use pkm_mcp::http;
use pkm_mcp::kbserver::SharedVault;
use pkm_mcp::server::KbServer;

/// Stratum MCP server.
#[derive(Debug, Parser)]
#[command(
    name = "pkm-mcp",
    version,
    about = "Stratum MCP server (Model Context Protocol)"
)]
struct Cli {
    /// Path to the Stratum vault (overrides PKM_MCP_VAULT).
    #[arg(long, value_name = "DIR")]
    vault: Option<PathBuf>,
    /// Transport: stdio | http | both.
    #[arg(long, value_name = "TRANS", default_value = "stdio")]
    transport: String,
    /// Bind address for the HTTP transport (overrides PKM_MCP_BIND).
    #[arg(long, value_name = "ADDR")]
    bind: Option<std::net::SocketAddr>,
    /// Require PAT authentication on the HTTP transport (overrides auth mode).
    #[arg(long)]
    require_auth: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Stdout is the MCP JSON-RPC transport on `--transport stdio`, so all log
    // output must go to stderr or it will corrupt the protocol frame stream.
    // See crates/pkm-mcp/examples/smoke_stdio.py (the end-to-end smoke gate).
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    // Build config from env, then apply CLI overrides.
    let mut cfg = if let Some(vault) = &cli.vault {
        let mut c = McpConfig::new(vault.clone());
        // Still layer env knobs (bind, rate limits, etc.) over the vault path.
        apply_env_overrides(&mut c)?;
        c
    } else {
        McpConfig::from_env().with_context(|| "set PKM_MCP_VAULT or pass --vault")?
    };

    if let Ok(t) = cli.transport.parse::<Transport>() {
        cfg.transport = t;
    }
    if let Some(bind) = cli.bind {
        cfg.bind = bind;
    }
    if cli.require_auth {
        cfg.auth_mode = AuthMode::Pat;
    }

    let vault = Arc::new(SharedVault::new(&cfg)?);
    tracing::info!(
        vault = %cfg.vault_path.display(),
        transport = ?cfg.transport,
        "stratum-mcp {} starting",
        pkm_mcp::server_version()
    );

    match cfg.transport {
        Transport::Stdio => run_stdio(vault).await,
        Transport::Http => http::serve_http(vault, cfg).await,
        Transport::Both => {
            // Both transports share one vault; stdio is foreground and HTTP is
            // spawned so a single process can serve either client.
            let http_cfg = cfg.clone();
            let http_vault = vault.clone();
            let http_task =
                tokio::spawn(async move { http::serve_http(http_vault, http_cfg).await });
            let stdio_task = tokio::spawn(async move { run_stdio(vault).await });
            // If either completes with an error, the whole process should fail.
            let (a, b) = tokio::join!(stdio_task, http_task);
            a.context("stdio transport")??;
            b.context("http transport")??;
            Ok(())
        }
    }
}

/// Read env overrides onto a config created from a CLI path.
fn apply_env_overrides(cfg: &mut McpConfig) -> anyhow::Result<()> {
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
    Ok(())
}

/// Serve the MCP protocol over stdio until EOF.
async fn run_stdio(vault: Arc<SharedVault>) -> anyhow::Result<()> {
    let server = KbServer::new(vault);
    let (stdin, stdout) = rmcp::transport::stdio();
    let svc = rmcp::service::serve_server(server, (stdin, stdout))
        .await
        .map_err(|e| anyhow::anyhow!("failed to serve over stdio: {e}"))?;
    svc.waiting()
        .await
        .map_err(|e| anyhow::anyhow!("stdio transport error: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // `std::env::set_var` mutates process-global state that the Rust test
    // harness shares across threads. Serialize the env-mutating tests behind
    // a static lock so one test's PKM_MCP_* value cannot bleed into a
    // concurrently-running sibling test (a race that makes the suite flaky).
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    /// Apply the given env overrides, run `f`, then restore the previous env.
    fn with_env<F, R>(vars: &[(&str, &str)], f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut previous = Vec::with_capacity(vars.len());
        for &(k, v) in vars {
            previous.push((k, std::env::var_os(k)));
            unsafe { std::env::set_var(k, v) };
        }
        let result = std::hint::black_box(f());
        for (k, prev) in previous {
            match prev {
                Some(v) => unsafe { std::env::set_var(k, v) },
                None => unsafe { std::env::remove_var(k) },
            };
        }
        result
    }

    #[test]
    fn cli_defaults_to_stdio() {
        let cli = Cli::try_parse_from(["pkm-mcp"]).expect("parse");
        assert_eq!(cli.transport, "stdio");
        assert!(!cli.require_auth);
    }

    #[test]
    fn cli_vault_flag_sets_path() {
        let cli = Cli::try_parse_from(["pkm-mcp", "--vault", "/tmp/v"]).expect("parse");
        assert_eq!(cli.vault.as_deref(), Some(std::path::Path::new("/tmp/v")));
    }

    #[test]
    fn cli_http_transport_and_bind() {
        let cli =
            Cli::try_parse_from(["pkm-mcp", "--transport", "http", "--bind", "127.0.0.1:8080"])
                .expect("parse");
        assert_eq!(cli.transport, "http");
        assert_eq!(cli.bind, Some("127.0.0.1:8080".parse().unwrap()));
    }

    #[test]
    fn cli_require_auth_flag() {
        let cli = Cli::try_parse_from(["pkm-mcp", "--require-auth"]).expect("parse");
        assert!(cli.require_auth);
    }

    #[test]
    fn cli_invalid_transport_accepts_string() {
        // clap accepts any string for the transport arg; main() ignores
        // invalid ones and falls back to the configured transport.
        let cli = Cli::try_parse_from(["pkm-mcp", "--transport", "bogus"]).expect("parse");
        assert_eq!(cli.transport, "bogus");
    }

    #[test]
    fn env_overrides_apply_bind() {
        with_env(&[("PKM_MCP_BIND", "127.0.0.1:9999")], || {
            let mut cfg = McpConfig::new(std::path::PathBuf::from("/tmp/v"));
            apply_env_overrides(&mut cfg).expect("apply");
            assert_eq!(cfg.bind.to_string(), "127.0.0.1:9999");
        });
    }

    #[test]
    fn env_overrides_invalid_bind_errors() {
        with_env(&[("PKM_MCP_BIND", "not-an-addr")], || {
            let mut cfg = McpConfig::new(std::path::PathBuf::from("/tmp/v"));
            let err = apply_env_overrides(&mut cfg).expect_err("must error");
            assert!(err.to_string().contains("PKM_MCP_BIND"));
        });
    }

    #[test]
    fn env_overrides_rate_limit_vals() {
        with_env(
            &[
                ("PKM_MCP_RATE_LIMIT_BURST", "5"),
                ("PKM_MCP_RATE_LIMIT_RPS", "2.5"),
            ],
            || {
                let mut cfg = McpConfig::new(std::path::PathBuf::from("/tmp/v"));
                apply_env_overrides(&mut cfg).expect("apply");
                assert_eq!(cfg.rate_limit_burst, 5);
                assert_eq!(cfg.rate_limit_rps, 2.5);
            },
        );
    }

    #[test]
    fn env_overrides_no_auth() {
        with_env(&[("PKM_MCP_NO_AUTH", "1")], || {
            let mut cfg = McpConfig::new(std::path::PathBuf::from("/tmp/v"));
            cfg.auth_mode = AuthMode::Pat;
            apply_env_overrides(&mut cfg).expect("apply");
            assert_eq!(cfg.auth_mode, AuthMode::None);
        });
    }
}
