//! HTTP transport integration tests for the MCP server.
//!
//! Drives the real production axum router (`pkm_mcp::http::build_router`)
//! over an ephemeral 127.0.0.1 listener, through the actual Streamable HTTP
//! transport (legacy session handshake) — the same path a real MCP client
//! (Claude Desktop, Cursor) uses. These tests exercise:
//!
//! * The full MCP `initialize` handshake over HTTP (contract §6) and the
//!   returned `mcp-session-id`.
//! * `tools/call` round-trips over an established session.
//! * PAT bearer auth at the transport boundary: a valid token is accepted,
//!   a missing/malformed/unknown token is rejected with `Unauthorized`
//!   (-32004, contract §8.1), and a read-only token is denied on a write
//!   tool with `ScopesDenied` (-32005) — all enforced in the transport
//!   middleware before any tool executes.
//! * The non-MCP `/health` endpoint (contract §12) is reachable without auth.
//!
//! This is the live-wire complement to `http_scope.rs`, which drives the
//! same auth/scope enforcement at the `dispatch_with_meta` layer.

mod common;
use common::http::*;

use serde_json::json;
use serde_json::Value;

/// A valid full-scope token for the server.
const FULL_TOKEN: &str = "full-scope-token-0123456789abcdefABCDEF";

fn full_scopes() -> Vec<String> {
    vec![
        "kb:read".to_string(),
        "kb:write".to_string(),
        "kb:admin".to_string(),
    ]
}

// ── Handshake & session lifecycle ──────────────────────────────────────

#[test]
fn http_initialize_returns_session_and_server_info() {
    let server = spawn(true, validator_with(FULL_TOKEN, full_scopes()), None);
    let resp = block_on(async { initialize(&server, Some(FULL_TOKEN)).await });
    assert_eq!(resp.status, reqwest::StatusCode::OK, "initialize 200");
    assert!(resp.session_id.is_some(), "session id issued");
    assert_eq!(resp.json["result"]["serverInfo"]["name"], "stratum-mcp");
    assert_eq!(
        resp.json["result"]["protocolVersion"], "2025-06-18",
        "contract §6 protocol version"
    );
    // The server advertises tools capability. rmcp serializes the
    // tools capability as `{"listChanged": false}`; assert the capability is
    // present without assuming its internal shape.
    let tools_cap = resp
        .json
        .get("result")
        .and_then(|r| r.get("capabilities"))
        .and_then(|c| c.get("tools"));
    assert!(tools_cap.is_some(), "tools capability present");
}

#[test]
fn http_session_roundtrip_write_then_read() {
    let server = spawn(true, validator_with(FULL_TOKEN, full_scopes()), None);
    block_on(async {
        let init = initialize(&server, Some(FULL_TOKEN)).await;
        assert_eq!(init.status, reqwest::StatusCode::OK);
        let sid = init.session_id.expect("session id");

        // Write a page over the established session.
        let write = call_tool(
            &server,
            Some(&sid),
            Some(FULL_TOKEN),
            "kb_write_page",
            json!({"path": "http-roundtrip.md", "content": "# Via HTTP transport"}),
        )
        .await;
        assert_eq!(write.status, reqwest::StatusCode::OK);
        assert!(!write.is_tool_error(), "write succeeds: {}", write.json);

        // Read it back over the same session.
        let read = call_tool(
            &server,
            Some(&sid),
            Some(FULL_TOKEN),
            "kb_get_page",
            json!({"path": "http-roundtrip.md"}),
        )
        .await;
        assert_eq!(read.status, reqwest::StatusCode::OK);
        assert!(!read.is_tool_error(), "read succeeds");
        assert_eq!(
            read.structured().and_then(|s| s["slug"].as_str()),
            Some("http-roundtrip")
        );
    });
}

#[test]
fn http_health_endpoint_is_unauthenticated() {
    let server = spawn(true, validator_with(FULL_TOKEN, full_scopes()), None);
    let (status, body) = block_on(async { health(&server).await });
    // `/health` (contract §12) is an operational endpoint and is NOT
    // auth-gated even when PAT auth is required for `/mcp`.
    assert_eq!(status, reqwest::StatusCode::OK);
    assert_eq!(body["status"], "ok");
    assert_eq!(body["store_connectivity"], true);
}

// ── PAT bearer auth at the transport boundary (contract §7) ───────────

#[test]
fn http_missing_token_is_unauthorized() {
    let server = spawn(true, validator_with(FULL_TOKEN, full_scopes()), None);
    let resp = block_on(async { initialize(&server, None).await });
    assert_eq!(
        resp.status,
        reqwest::StatusCode::UNAUTHORIZED,
        "no token -> 401"
    );
    assert_eq!(resp.error_code(), Some(-32004), "Unauthorized code (§8.1)");
}

#[test]
fn http_unknown_token_is_unauthorized() {
    let server = spawn(true, validator_with(FULL_TOKEN, full_scopes()), None);
    let resp = block_on(async { initialize(&server, Some("not-a-real-token")).await });
    assert_eq!(resp.status, reqwest::StatusCode::UNAUTHORIZED);
    assert_eq!(resp.error_code(), Some(-32004));
}

#[test]
fn http_malformed_auth_header_is_unauthorized() {
    let server = spawn(true, validator_with(FULL_TOKEN, full_scopes()), None);
    let resp = block_on(async {
        // Send a header that is neither `Stratum-MCP <token>` nor empty, so
        // the extractor must reject it as malformed (§8.1 Unauthorized).
        let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "t", "version": "1"}
            }
        });
        let raw = server
            .client
            .post(format!("{}/mcp", server.base_url))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .header("Authorization", "Basic ZnJlZDpmcmVk")
            .body(body.to_string())
            .send()
            .await
            .expect("send");
        // Drain the body ourselves since this path bypasses the harness wrapper.
        let status = raw.status();
        let text = raw.text().await.unwrap_or_default();
        let json = serde_json::from_str(&text).unwrap_or(Value::Null);
        (status, json)
    });
    assert_eq!(resp.0, reqwest::StatusCode::UNAUTHORIZED);
    let code = resp.1.pointer("/error/code").and_then(Value::as_i64);
    assert_eq!(code, Some(-32004));
}

#[test]
fn http_readonly_token_denied_on_write_tool() {
    let server = spawn(
        true,
        validator_with("ro-token", vec!["kb:read".to_string()]),
        None,
    );
    block_on(async {
        let init = initialize(&server, Some("ro-token")).await;
        assert_eq!(init.status, reqwest::StatusCode::OK);
        let sid = init.session_id.expect("session id");

        let write = call_tool(
            &server,
            Some(&sid),
            Some("ro-token"),
            "kb_write_page",
            json!({"path": "denied.md", "content": "# nope"}),
        )
        .await;
        assert_eq!(
            write.status,
            reqwest::StatusCode::OK,
            "tool-level error, not HTTP 4xx"
        );
        assert!(write.is_tool_error(), "write denied for read-only token");
        assert_eq!(write.error_code(), Some(-32005), "ScopesDenied (§8.1)");
    });
}

#[test]
fn http_unknown_tool_is_protocol_error() {
    let server = spawn(true, validator_with(FULL_TOKEN, full_scopes()), None);
    block_on(async {
        let init = initialize(&server, Some(FULL_TOKEN)).await;
        let sid = init.session_id.expect("session id");
        let resp = call_tool(
            &server,
            Some(&sid),
            Some(FULL_TOKEN),
            "kb_definitely_not_a_tool",
            json!({}),
        )
        .await;
        // Unknown method surfaces as a JSON-RPC protocol error body.
        let body = resp.json.to_string();
        assert!(
            body.to_lowercase().contains("method not found") || resp.error_code().is_some(),
            "unknown tool reported as error: {body}"
        );
    });
}

#[test]
fn http_get_on_mcp_is_405() {
    let server = spawn(true, validator_with(FULL_TOKEN, full_scopes()), None);
    block_on(async {
        // The auth/rate-limit middleware runs before method dispatch, so an
        // unauthenticated GET is rejected with 401 first. Send a valid token
        // to reach the router's method handler, which is GET-unaware (the
        // pinned Streamable HTTP revision only supports POST).
        let resp = server
            .client
            .get(format!("{}/mcp", server.base_url))
            .header("Authorization", format!("Stratum-MCP {FULL_TOKEN}"))
            .send()
            .await
            .expect("GET /mcp");
        assert_eq!(resp.status(), reqwest::StatusCode::METHOD_NOT_ALLOWED);
    });
}

#[test]
fn http_unknown_route_is_404() {
    let server = spawn(true, validator_with(FULL_TOKEN, full_scopes()), None);
    let (status, body) = block_on(async {
        let resp = server
            .client
            .get(format!("{}/nope", server.base_url))
            .send()
            .await
            .expect("GET unknown");
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        let parsed: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
        (status, parsed)
    });
    assert_eq!(status, reqwest::StatusCode::NOT_FOUND);
    assert_eq!(body, Value::Null);
}
