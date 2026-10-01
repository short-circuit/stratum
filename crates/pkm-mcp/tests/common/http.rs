//! Shared HTTP test harness for the Stratum MCP server.
//!
//! Spins up the real production HTTP router (`pkm_mcp::http::build_router`)
//! on an ephemeral 127.0.0.1 port and drives it with a raw reqwest client.
//! This is the closest thing to a real deployment short of the full `main`:
//! it exercises the actual PAT middleware, rate limiter, `/health` endpoint,
//! and the Streamable HTTP transport (legacy session handshake) end-to-end.
//!
//! Used by the `tests/http_scope.rs` and `tests/http_rate_limit.rs` suites.
//! It complements the stdio contract tests, which cannot exercise the
//! HTTP-only auth surface.
//!
//! This module is compiled into each integration-test binary, and different
//! suites use different subsets of the helpers, so unused-function warnings
//! are expected per binary. Silence them: the helpers are a library shared
//! across suites, not dead code.
#![allow(dead_code)]

use std::sync::Arc;

use pkm_mcp::auth::{hash_token, TokenRecord, TokenValidator};
use pkm_mcp::config::McpConfig;
use pkm_mcp::http::{build_router, token_bucket::TokenBucket};
use pkm_mcp::kbserver::SharedVault;
use serde_json::Value;
use tempfile::TempDir;

/// A running MCP test server (real axum router on an ephemeral port).
pub struct TestServer {
    pub _dir: TempDir,
    pub base_url: String,
    pub client: reqwest::Client,
    /// Holds the server thread alive for the test's duration.
    pub _keepalive: std::thread::JoinHandle<()>,
}

/// Default personal scopes (mirrors `pkm_mcp::config::default_personal_scopes`).
pub fn personal_scopes() -> Vec<String> {
    vec!["kb:read".into(), "kb:write".into(), "kb:admin".into()]
}

/// Build a `TokenValidator` with a single token granted the given scopes.
pub fn validator_with(token: &str, scopes: Vec<String>) -> TokenValidator {
    TokenValidator::new(vec![TokenRecord {
        hash: hash_token(token),
        scopes,
    }])
}

/// A raw HTTP response together with the parsed JSON body and any session id.
pub struct HttpResponse {
    pub status: reqwest::StatusCode,
    pub json: Value,
    pub session_id: Option<String>,
}

impl HttpResponse {
    /// Extract the MCP JSON-RPC error code from the response, if any.
    pub fn error_code(&self) -> Option<i64> {
        self.json["error"]["code"]
            .as_i64()
            .or_else(|| self.json["result"]["structuredContent"]["error"]["code"].as_i64())
    }

    /// Whether the tool call reported `isError: true`.
    pub fn is_tool_error(&self) -> bool {
        self.json["result"]["isError"].as_bool().unwrap_or(false)
            || self.json["result"]["structuredContent"]["isError"]
                .as_bool()
                .unwrap_or(false)
            || self.json["result"]["structuredContent"]["error"].is_object()
    }

    /// The `structuredContent` object, if present.
    pub fn structured(&self) -> Option<&Value> {
        if self.json["result"]["structuredContent"].is_object() {
            Some(&self.json["result"]["structuredContent"])
        } else {
            None
        }
    }
}

/// Spawn the real HTTP server with the given auth configuration.
pub fn spawn(
    auth_required: bool,
    validator: TokenValidator,
    bucket: Option<Arc<TokenBucket>>,
) -> TestServer {
    let dir = TempDir::new().expect("temp vault dir");
    std::fs::create_dir_all(dir.path().join(".pkm")).expect("create .pkm");
    let mut cfg = McpConfig::new(dir.path().to_path_buf());
    cfg.transport = pkm_mcp::config::Transport::Http;
    let vault = Arc::new(SharedVault::new(&cfg).expect("vault init"));

    let router = build_router(vault, Arc::new(validator), bucket, auth_required);

    // Run the server on its own OS thread with a dedicated runtime so that
    // dropping the test's `TestServer` cannot race the runtime shutdown (a
    // runtime dropped while a background task is still running panics with
    // "Cannot drop a runtime in a context where blocking is not allowed").
    let (addr_tx, addr_rx) = std::sync::mpsc::channel();
    let thread = std::thread::Builder::new()
        .name("pkm-mcp-test-server".into())
        .spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("server runtime");
            rt.block_on(async {
                let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                    .await
                    .expect("bind ephemeral port");
                let addr = listener.local_addr().expect("local addr");
                addr_tx.send(addr).expect("send addr");
                let _ = axum::serve(listener, router).await;
            });
        })
        .expect("spawn server thread");
    let addr = addr_rx.recv().expect("receive addr");

    TestServer {
        _dir: dir,
        base_url: format!("http://{addr}"),
        client: reqwest::Client::new(),
        _keepalive: thread,
    }
}

/// Perform the MCP legacy `initialize` handshake.
pub async fn initialize(server: &TestServer, auth: Option<&str>) -> HttpResponse {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "pkm-mcp-http-test", "version": "1.0"}
        }
    });
    let mut req = server
        .client
        .post(format!("{}/mcp", server.base_url))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream");
    if let Some(tok) = auth {
        req = req.header("Authorization", format!("Stratum-MCP {tok}"));
    }
    let resp = req
        .body(body.to_string())
        .send()
        .await
        .expect("send initialize");
    parse_response(resp).await
}

/// Issue a `tools/call` over the session.
pub async fn call_tool(
    server: &TestServer,
    session: Option<&str>,
    auth: Option<&str>,
    tool: &str,
    args: Value,
) -> HttpResponse {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": tool,
            "arguments": args
        }
    });
    post_json(server, session, auth, body).await
}

/// POST a raw JSON-RPC request and return the parsed response.
pub async fn post_json(
    server: &TestServer,
    session: Option<&str>,
    auth: Option<&str>,
    body: Value,
) -> HttpResponse {
    let mut req = server
        .client
        .post(format!("{}/mcp", server.base_url))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream");
    if let Some(sid) = session {
        if !sid.is_empty() {
            req = req.header("Mcp-Session-Id", sid);
        }
    }
    if let Some(tok) = auth {
        req = req.header("Authorization", format!("Stratum-MCP {tok}"));
    }
    let resp = req
        .body(body.to_string())
        .send()
        .await
        .expect("send json-rpc");
    parse_response(resp).await
}

async fn parse_response(resp: reqwest::Response) -> HttpResponse {
    // The server is configured with legacy session mode, which responds to
    // successful JSON-RPC requests with SSE framing (`data:<json>`), not raw
    // JSON, even though `Accept` includes application/json (rmcp: json_response
    // is ignored in legacy session mode). Parse both:
    //   1. a direct JSON body (our 401/429 middleware responses),
    //   2. an SSE body (rmcp transport responses) — take the LAST `data:`
    //      frame, which is the JSON-RPC response (earlier frames are progress
    //      notifications).
    let status = resp.status();
    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let session_id = resp
        .headers()
        .get("mcp-session-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let text = resp.text().await.unwrap_or_default();

    let json = if content_type.contains("text/event-stream") {
        // SSE: parse each `data:` frame as JSON; keep the last.
        let mut last: Value = Value::Null;
        for line in text.lines() {
            if let Some(data) = line.strip_prefix("data:") {
                let data = data.trim();
                if data.is_empty() || data.starts_with("[DONE]") {
                    continue;
                }
                if let Ok(v) = serde_json::from_str::<Value>(data) {
                    last = v;
                }
            }
        }
        last
    } else {
        serde_json::from_str(&text).unwrap_or(Value::Null)
    };

    HttpResponse {
        status,
        json,
        session_id,
    }
}

/// GET `/health` and return (status, parsed body).
pub async fn health(server: &TestServer) -> (reqwest::StatusCode, Value) {
    let resp = server
        .client
        .get(format!("{}/health", server.base_url))
        .send()
        .await
        .expect("health GET");
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    let parsed = serde_json::from_str(&text).unwrap_or(Value::Null);
    (status, parsed)
}

/// Run an async block on a fresh runtime (tests are sync).
pub fn block_on<F, T>(fut: F) -> T
where
    F: std::future::Future<Output = T>,
{
    tokio::runtime::Runtime::new()
        .expect("runtime")
        .block_on(fut)
}

/// The MCP JSON-RPC error code for scope/permission denial (contract §8.1).
pub fn scopes_denied_code() -> i64 {
    -32005
}
