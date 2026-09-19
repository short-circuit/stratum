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
    /// Transport: stdio | http | both. When omitted, PKM_MCP_TRANSPORT (or the
    /// config default) is honored.
    #[arg(long, value_name = "TRANS")]
    transport: Option<String>,
    /// Bind address for the HTTP transport (overrides PKM_MCP_BIND).
    #[arg(long, value_name = "ADDR")]
    bind: Option<std::net::SocketAddr>,
    /// Require PAT authentication on the HTTP transport (overrides auth mode).
    #[arg(long)]
    require_auth: bool,
    /// One-shot local health probe (contract §12): resolves the vault from
    /// PKM_MCP_VAULT / --vault, opens the block store and checks index
    /// freshness, then exits 0 on healthy / non-zero on degraded. Used by the
    /// container HEALTHCHECK (the distroless runtime has no shell or curl).
    #[arg(long)]
    health_probe: bool,
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

    // CLI flags are explicit overrides. The clap defaults (transport=stdio,
    // no bind) must NOT silently clobber env-provided values (PKM_MCP_*), so
    // only apply them when the flag was actually passed by the operator. This
    // is what makes `PKM_MCP_TRANSPORT=http` (used by the Docker image and
    // env.template) actually take effect.
    if let Some(transport) = cli.transport {
        cfg.transport = transport
            .parse()
            .map_err(|e: String| anyhow::anyhow!("invalid transport: {e}"))?;
    }
    if let Some(bind) = cli.bind {
        cfg.bind = bind;
    }
    if cli.require_auth {
        cfg.auth_mode = AuthMode::Pat;
    }

    // One-shot health probe (contract §12): a real store/index liveness check,
    // not a shell alias. Used by the container HEALTHCHECK.
    if cli.health_probe {
        let db_path = cfg.vault_path.join(".pkm").join("blocks.db");
        return match pkm_mcp::kbserver::probe_health(
            std::path::Path::new(&cfg.vault_path),
            &db_path,
        ) {
            Ok(probe) if probe.index_fresh => Ok(()),
            Ok(probe) => {
                tracing::warn!(
                    vault = %cfg.vault_path.display(),
                    indexed_pages = probe.indexed_pages,
                    page_count = probe.page_count,
                    "health probe: index not fresh"
                );
                anyhow::bail!("health probe: unhealthy (index not fresh)")
            }
            Err(e) => {
                tracing::warn!(vault = %cfg.vault_path.display(), "health probe failed");
                anyhow::bail!("health probe: {e}")
            }
        };
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
