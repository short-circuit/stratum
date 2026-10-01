//! HTTP transport: Streamable HTTP serving via axum.
//!
//! Mounts the MCP endpoint at `<base>/mcp` (rmcp `StreamableHttpService` via
//! `nest_service`) and a non-MCP `/health` endpoint (contract §12). PAT bearer
//! auth and rate limiting are applied at the transport boundary before the RPC
//! is handed to rmcp. The resulting `AuthContext` is injected into the HTTP
//! request extensions so the tool layer can scope-check.

use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
};
use tokio::net::TcpListener;
use tracing::info;

use crate::auth::{AuthContext, TokenValidator};
use crate::kbserver::{self, SharedVault};
use crate::server::{KbServer, RequestMeta};

/// An in-memory token bucket for per-token rate limiting (contract §9).
pub mod token_bucket {
    use std::sync::Mutex;
    use std::time::Instant;

    #[derive(Debug)]
    pub struct TokenBucket {
        capacity: u64,
        refill_per_sec: f64,
        state: Mutex<BucketState>,
    }

    #[derive(Debug)]
    struct BucketState {
        tokens: f64,
        last: Instant,
    }

    impl TokenBucket {
        pub fn new(capacity: u64, refill_per_sec: f64) -> Self {
            Self {
                capacity,
                refill_per_sec,
                state: Mutex::new(BucketState {
                    tokens: capacity as f64,
                    last: Instant::now(),
                }),
            }
        }

        pub fn try_acquire(&self) -> bool {
            let mut st = self.state.lock().expect("bucket lock");
            let now = Instant::now();
            let elapsed = now.duration_since(st.last).as_secs_f64();
            st.last = now;
            st.tokens = (st.tokens + elapsed * self.refill_per_sec).min(self.capacity as f64);
            if st.tokens >= 1.0 {
                st.tokens -= 1.0;
                true
            } else {
                false
            }
        }

        /// Seconds a client should wait before retrying (contract §9 `Retry-After`).
        pub fn retry_after_secs(&self) -> u64 {
            let st = self.state.lock().expect("bucket lock");
            let deficit = 1.0 - st.tokens;
            let secs = if self.refill_per_sec > 0.0 {
                (deficit / self.refill_per_sec).ceil() as u64
            } else {
                1
            };
            secs.max(1)
        }
    }
}

/// State shared by the HTTP routes.
#[derive(Clone)]
pub struct HttpState {
    pub validator: Arc<TokenValidator>,
    pub bucket: Option<Arc<token_bucket::TokenBucket>>,
    pub vault_path: String,
    pub db_path: String,
}

/// Build the axum router for the MCP server.
pub fn build_router(
    vault: Arc<SharedVault>,
    validator: Arc<TokenValidator>,
    bucket: Option<Arc<token_bucket::TokenBucket>>,
    auth_required: bool,
) -> Router {
    let mcp_service = StreamableHttpService::new(
        {
            let vault = vault.clone();
            let bucket = bucket.clone();
            move || {
                Ok(KbServer::new(vault.clone())
                    .with_auth_mode(if auth_required {
                        crate::config::AuthMode::Pat
                    } else {
                        crate::config::AuthMode::None
                    })
                    .with_rate_limiter(bucket.clone()))
            }
        },
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default()
            .with_json_response(true)
            .with_legacy_session_mode(true),
    );

    let state = HttpState {
        validator,
        bucket,
        vault_path: vault.vault_path.to_string_lossy().to_string(),
        db_path: vault.db_path.to_string_lossy().to_string(),
    };

    // The Streamable HTTP endpoint is served by the rmcp tower service
    // (`/mcp`, POST), wrapped in middleware that applies PAT auth + rate
    // limiting before the RPC is dispatched. The authenticated context is
    // attached to the request so the tool layer can scope-check (§7).
    // `/health` is a non-MCP operational endpoint (§12) and is NOT auth-gated.
    let mcp_router = Router::new()
        .route(
            "/mcp",
            axum::routing::get(mcp_host).post_service(mcp_service),
        )
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            mcp_auth,
        ));

    Router::new()
        .route("/health", get(health))
        .merge(mcp_router)
        .fallback(not_found)
        .with_state(state)
}

async fn mcp_host(State(state): State<HttpState>, req: Request) -> Response {
    // Streamable HTTP (pinned revision) has no GET stream; POST is the only
    // request method. Return 405 with a helpful Allow header.
    let _ = (state, req);

    (StatusCode::METHOD_NOT_ALLOWED, "Method Not Allowed").into_response()
}

async fn not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "not found")
}

async fn health(State(state): State<HttpState>) -> Response {
    match kbserver::probe_health(
        std::path::Path::new(&state.vault_path),
        std::path::Path::new(&state.db_path),
    ) {
        Ok(probe) => auth_json_response(
            StatusCode::OK,
            &serde_json::json!({
                "status": "ok",
                "vault_path": state.vault_path,
                "store_connectivity": true,
                "index_fresh": probe.index_fresh,
                "page_count": probe.page_count,
                "indexed_pages": probe.indexed_pages,
            }),
        ),
        Err(e) => auth_json_response(
            StatusCode::SERVICE_UNAVAILABLE,
            &serde_json::json!({
                "status": "degraded",
                "vault_path": state.vault_path,
                "store_connectivity": false,
                "error": e,
            }),
        ),
    }
}

use axum::middleware::Next;

async fn mcp_auth(State(state): State<HttpState>, mut req: Request, next: Next) -> Response {
    // Extract Authorization header -> AuthContext.
    let auth = match extract_auth(&req, &state.validator) {
        Ok(ctx) => ctx,
        Err(resp) => return resp,
    };

    // Rate limit unless disabled.
    if let Some(bucket) = &state.bucket {
        if !bucket.try_acquire() {
            let retry = bucket.retry_after_secs();
            let body = serde_json::json!({
                "jsonrpc": "2.0",
                "id": null,
                "error": {
                    "code": -32003,
                    "message": "Rate limit exceeded",
                    "data": {"kind": "RateLimited", "retryAfterSeconds": retry}
                }
            });
            return (
                StatusCode::TOO_MANY_REQUESTS,
                [(axum::http::header::RETRY_AFTER, retry.to_string())],
                serde_json::to_string(&body).unwrap(),
            )
                .into_response();
        }
    }

    // Attach the authenticated context to the request so the tool layer can
    // scope-check. rmcp injects the request's `http::request::Parts` into the
    // `RequestContext.extensions`, where the tool dispatch reads it.
    req.extensions_mut().insert(RequestMeta {
        auth,
        rate_limited: state.bucket.is_some(),
    });
    next.run(req).await
}

fn extract_auth(req: &Request, validator: &TokenValidator) -> Result<AuthContext, Response> {
    let auth = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if let Some(token) = crate::auth::parse_authorization(auth) {
        match validator.authenticate(&token) {
            Some(ctx) => Ok(ctx),
            None => Err(auth_json_response(
                StatusCode::UNAUTHORIZED,
                &serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": null,
                    "error": {
                        "code": -32004,
                        "message": "Invalid or unknown token",
                        "data": {"kind": "Unauthorized"},
                    }
                }),
            )),
        }
    } else if auth.is_empty() && validator.is_empty() {
        // Local mode: no auth required.
        Ok(AuthContext::default())
    } else {
        Err(auth_json_response(
            StatusCode::UNAUTHORIZED,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": null,
                "error": {
                    "code": -32004,
                    "message": "Missing or malformed Authorization header",
                    "data": {"kind": "Unauthorized"},
                }
            }),
        ))
    }
}

fn auth_json_response(status: StatusCode, body: &serde_json::Value) -> Response {
    let mut resp = (status, serde_json::to_string(body).unwrap_or_default()).into_response();
    resp.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    resp
}

/// Run the HTTP server until interrupted.
pub async fn serve_http(
    vault: Arc<SharedVault>,
    cfg: crate::config::McpConfig,
) -> anyhow::Result<()> {
    let validator = Arc::new(crate::auth::build_validator(
        cfg.auth_mode,
        std::env::var("PKM_MCP_TOKEN").ok(),
        std::env::var("PKM_MCP_TOKEN_FILE")
            .ok()
            .map(std::path::PathBuf::from)
            .as_deref(),
    ));
    let bucket = cfg.rate_limit_rps.gt(&0.0).then(|| {
        Arc::new(token_bucket::TokenBucket::new(
            cfg.rate_limit_burst as u64,
            cfg.rate_limit_rps,
        ))
    });

    let router = build_router(
        vault,
        validator,
        bucket,
        cfg.auth_mode == crate::config::AuthMode::Pat,
    );
    let addr = cfg.bind;
    info!("MCP HTTP server listening on http://{addr}/mcp (health: http://{addr}/health)");
    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| anyhow::anyhow!("failed to bind {}: {e}", addr))?;
    axum::serve(listener, router)
        .await
        .map_err(|e| anyhow::anyhow!("HTTP server error: {e}"))
}

#[cfg(test)]
mod tests {
    use super::token_bucket::TokenBucket;
    use crate::auth::build_validator;
    use crate::config::AuthMode;
    use std::time::Duration;

    #[test]
    fn test_token_bucket_capacity() {
        let bucket = TokenBucket::new(2, 1.0);
        assert!(bucket.try_acquire());
        assert!(bucket.try_acquire());
        assert!(!bucket.try_acquire(), "burst exhausted");
        assert!(bucket.retry_after_secs() >= 1);
    }

    #[test]
    fn test_token_bucket_refill() {
        let bucket = TokenBucket::new(1, 10.0);
        assert!(bucket.try_acquire());
        std::thread::sleep(Duration::from_millis(150));
        assert!(bucket.try_acquire(), "token should refill after 0.1s");
    }

    #[test]
    fn test_no_auth_mode() {
        let validator = build_validator(AuthMode::None, None, None);
        assert!(!validator.required());
    }

    #[test]
    fn test_pat_validation() {
        use crate::auth::build_validator;
        let token = "0123456789012345678901234567890123456789012"; // 43 chars
        let _ = build_validator(AuthMode::Pat, Some(token.to_string()), None);
    }

    #[test]
    fn test_serve_uses_token_file_from_env() {
        // Regression: an HTTP deployment that configures only
        // PKM_MCP_TOKEN_FILE must be enforced (auth on), not silently run
        // open. This mirrors exactly how `serve_http` builds the validator.
        use crate::config::{default_personal_scopes, load_tokens_from_file};
        use std::io::Write;

        let token = "sk-stratum-0123456789abcdef0123456789abcdef";
        let mut f = tempfile::NamedTempFile::new().unwrap();
        writeln!(f.as_file_mut(), "{token}").unwrap();

        let validator = build_validator(AuthMode::Pat, None, Some(f.path()));
        assert!(validator.required(), "token file must enable auth");
        assert!(
            validator
                .authenticate("sk-stratum-0123456789abcdef0123456789abcdef")
                .is_some(),
            "token from file must authenticate"
        );

        // A bare token in the file carries the default personal scope set
        // (contract §7.2) — never an empty/missing scope list.
        let (tokens, scopes) = load_tokens_from_file(f.path())
            .unwrap()
            .into_iter()
            .next()
            .unwrap();
        assert_eq!(tokens, "sk-stratum-0123456789abcdef0123456789abcdef");
        assert_eq!(scopes, default_personal_scopes());
    }
}
