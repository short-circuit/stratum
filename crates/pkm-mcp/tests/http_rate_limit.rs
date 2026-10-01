//! HTTP rate-limit integration tests for the MCP server.
//!
//! The transport middleware (`http.rs::mcp_auth`) enforces rate limiting at
//! the HTTP boundary: when the shared token bucket is exhausted the request
//! is rejected with HTTP 429 and a `Retry-After` header, and the body carries
//! the contract's `RateLimited` code (-32003, §8.1 / §9).
//!
//! This is a live-router test (same harness as `http_transport.rs`),
//! complementing the dispatch-level rate-limit coverage in `http_scope.rs`
//! (which asserts the -32003 tool-level error for an exhausted bucket).

mod common;
use common::http::*;

use pkm_mcp::http::token_bucket::TokenBucket;
use serde_json::Value;
use std::sync::Arc;

const TOKEN: &str = "rate-token";
const INIT_BODY: &str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"pkm-mcp-http-test","version":"1.0"}}}"#;

/// POST a raw body to `/mcp` and return (status, parsed error code, Retry-After).
async fn post(
    server: &TestServer,
    body: &str,
) -> (reqwest::StatusCode, Option<i64>, Option<String>) {
    let resp = server
        .client
        .post(format!("{}/mcp", server.base_url))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("Authorization", format!("Stratum-MCP {TOKEN}"))
        .body(body.to_string())
        .send()
        .await
        .expect("send");
    let status = resp.status();
    let retry_after = resp
        .headers()
        .get("retry-after")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let text = resp.text().await.unwrap_or_default();
    let parsed: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    let code = parsed["error"]["code"].as_i64();
    (status, code, retry_after)
}

#[test]
fn http_exhausted_bucket_returns_429_with_retry_after() {
    // Capacity 1, zero refill: the first request consumes the only token,
    // every subsequent request is rejected at the HTTP boundary.
    let bucket = Arc::new(TokenBucket::new(1, 0.0));
    let server = spawn(true, validator_with(TOKEN, personal_scopes()), Some(bucket));

    block_on(async {
        let (s0, c0, _) = post(&server, INIT_BODY).await;
        assert_eq!(s0, reqwest::StatusCode::OK, "first request succeeds");
        assert!(c0.is_none(), "no error on first request");

        let (s1, c1, retry) = post(&server, INIT_BODY).await;
        assert_eq!(s1, reqwest::StatusCode::TOO_MANY_REQUESTS, "rate limited");
        assert_eq!(
            c1,
            Some(-32003),
            "RateLimited (§8.1) maps to 429 at the boundary"
        );
        assert!(
            retry.is_some(),
            "Retry-After header present on 429 (contract §9)"
        );

        let (s2, c2, _) = post(&server, INIT_BODY).await;
        assert_eq!(s2, reqwest::StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(c2, Some(-32003), "still exhausted");
    });
}

#[test]
fn http_bucket_exhaustion_tracks_consumed_tokens() {
    // Deterministic multi-consumption: capacity 2 with no refill. The first
    // two requests consume the two tokens; the third is rejected with 429.
    // (Wall-clock refill is covered by the unit test
    // `pkm_mcp::http::tests::test_token_bucket_refill` — driving wall-clock
    // refill through the live router would be timing-flaky.)
    let bucket = Arc::new(TokenBucket::new(2, 0.0));
    let server = spawn(true, validator_with(TOKEN, personal_scopes()), Some(bucket));

    block_on(async {
        let (s0, _, _) = post(&server, INIT_BODY).await;
        assert_eq!(s0, reqwest::StatusCode::OK, "first request consumes token");

        let (s1, _, _) = post(&server, INIT_BODY).await;
        assert_eq!(
            s1,
            reqwest::StatusCode::OK,
            "second request consumes last token"
        );

        let (s2, c2, _) = post(&server, INIT_BODY).await;
        assert_eq!(s2, reqwest::StatusCode::TOO_MANY_REQUESTS, "exhausted");
        assert_eq!(c2, Some(-32003), "RateLimited (§8.1)");
    });
}
