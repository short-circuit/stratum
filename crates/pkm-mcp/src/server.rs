//! The MCP `ServerHandler` implementation.
//!
//! [`KbServer`] implements the contract in `docs/advanced/mcp.md`: it
//! advertises the pinned protocol version (2025-06-18), tools-only capabilities
//! (`tools.listChanged=false`), the exact 15 `kb_*` tools with binding JSON
//! Schemas, per-request auth + scope enforcement, contract error vocabulary,
//! and a shared single-writer vault.
//!
//! Requests arrive through a transport (stdio or Streamable HTTP). The
//! [`ServerHandler`] trait methods (list_tools / call_tool / get_tool /
//! get_info) are implemented here; transport-specific bits live in `http.rs`.

use std::borrow::Cow;
use std::sync::Arc;

use rmcp::handler::server::ServerHandler;
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ListToolsResult,
    PaginatedRequestParams, ProtocolVersion, ServerConfig, Tool,
};
use rmcp::service::{MaybeSendFuture, RequestContext, RoleServer};
use rmcp::ErrorData as McpError;
use serde_json::Value;
use tracing::debug;

use crate::auth::AuthContext;
use crate::error::{ErrorDataExt, KbError, KbErrorKind};
use crate::kbserver::SharedVault;
use crate::tools::{all_tools, ToolDef};

/// Per-request context injected by the HTTP transport so tools can enforce
/// auth and rate limits without reaching into HTTP internals.
#[derive(Debug, Clone, Default)]
pub struct RequestMeta {
    /// Authenticated principal + scopes (empty = no auth / local).
    pub auth: AuthContext,
    /// Whether rate limiting applies to this request.
    pub rate_limited: bool,
}

/// The MCP server handler backed by a shared vault.
///
/// Each transport session gets its own `KbServer` (rmcp's Streamable HTTP
/// service factory creates one per session); all share the same
/// [`SharedVault`], which owns the single-writer mutex and the on-disk data
/// handles. This is safe: the data layer opens fresh SQLite connections and
/// the write mutex serializes writers.
#[derive(Debug, Clone)]
pub struct KbServer {
    pub vault: Arc<SharedVault>,
    tools: Arc<Vec<ToolDef>>,
    /// Local/stdio sessions skip auth; HTTP sessions inject a per-request
    /// AuthContext via the request extensions.
    auth_mode: crate::config::AuthMode,
    /// Per-session rate limit state, when applicable.
    rate_limiter: Option<Arc<crate::http::token_bucket::TokenBucket>>,
}

impl KbServer {
    /// Build a server over a vault.
    pub fn new(vault: Arc<SharedVault>) -> Self {
        let tools = Arc::new(all_tools());
        Self {
            vault,
            tools,
            auth_mode: crate::config::AuthMode::None,
            rate_limiter: None,
        }
    }

    /// Set the auth mode (from config).
    pub fn with_auth_mode(mut self, mode: crate::config::AuthMode) -> Self {
        self.auth_mode = mode;
        self
    }

    /// Share a token bucket across sessions (from config).
    pub fn with_rate_limiter(
        mut self,
        bucket: Option<Arc<crate::http::token_bucket::TokenBucket>>,
    ) -> Self {
        self.rate_limiter = bucket;
        self
    }

    fn tool_by_name(&self, name: &str) -> Option<ToolDef> {
        self.tools.iter().find(|t| t.name == name).cloned()
    }

    fn check_scope(&self, tool: &ToolDef, ctx: &AuthContext) -> Result<(), KbError> {
        // Local (no-auth) mode grants all scopes.
        if self.auth_mode == crate::config::AuthMode::None {
            return Ok(());
        }
        ctx.require_scope(tool.scope)
    }
}

/// Validate a tool invocation's arguments against its binding schema.
/// Returns the raw arguments object on success, or `InvalidArgs` on failure.
fn validate_args(schema: &Value, args: Option<Value>) -> Result<Value, KbError> {
    let args = args.unwrap_or(Value::Object(serde_json::Map::new()));
    let compiled = jsonschema::validator_for(schema)
        .map_err(|e| ErrorDataExt::internal(format!("schema compile failed: {e}")))?;
    let errors: Vec<String> = compiled
        .iter_errors(&args)
        .take(3)
        .map(|e| e.to_string())
        .collect();
    if !errors.is_empty() {
        return Err(ErrorDataExt::invalid_args(format!(
            "invalid arguments: {}",
            errors.join("; ")
        )));
    }
    Ok(args)
}

impl ServerHandler for KbServer {
    fn get_info(&self) -> ServerConfig {
        // Tools-only capabilities (contract §1: tools.listChanged=false).
        let capabilities = rmcp::model::ServerCapabilities::builder()
            .enable_tools()
            .build();
        ServerConfig::new(capabilities)
            .with_server_info(rmcp::model::Implementation::new(
                crate::server_name(),
                crate::server_version(),
            ))
            .with_protocol_version(ProtocolVersion::V_2025_06_18)
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListToolsResult, McpError>> + MaybeSendFuture + '_
    {
        let tools: Vec<Tool> = self.tools.iter().map(|t| t.tool.clone()).collect();
        std::future::ready(Ok(ListToolsResult {
            tools,
            next_cursor: None,
            ..Default::default()
        }))
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        self.tool_by_name(name).map(|t| t.tool.clone())
    }

    fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<CallToolResponse, McpError>> + MaybeSendFuture + '_
    {
        let tool_name = request.name.to_string();
        let args = request.arguments.map(Value::Object);
        let self_clone = self.clone();
        async move { self_clone.dispatch(&tool_name, args, &context).await }
    }

    fn supported_protocol_versions(&self) -> Cow<'static, [ProtocolVersion]> {
        Cow::Borrowed(ProtocolVersion::known_up_to(&ProtocolVersion::V_2025_06_18))
    }
}

impl KbServer {
    /// Dispatch a tool call.
    async fn dispatch(
        &self,
        name: &str,
        args: Option<Value>,
        context: &RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, McpError> {
        // Resolve the auth context from the request context extension. The
        // Streamable HTTP transport injects the raw `http::request::Parts`
        // into the context; our transport middleware stores the authenticated
        // `RequestMeta` inside those parts' extensions. On stdio (or any
        // session without an HTTP envelope) this falls back to an
        // unauthenticated default.
        let meta = context
            .extensions
            .get::<axum::http::request::Parts>()
            .and_then(|p| p.extensions.get::<RequestMeta>())
            .cloned()
            .unwrap_or_default();
        let auth = meta.auth;
        let rate_limited = meta.rate_limited;
        self.dispatch_with_meta(name, args, auth, rate_limited)
            .await
    }

    /// The core dispatch path once the per-request auth context is resolved.
    ///
    /// This is the single code path through which every authenticated tool call
    /// passes: per-token rate limiting, scope enforcement, schema validation,
    /// then execution. It is factored out of [`dispatch`] so the HTTP transport
    /// and the test suite can drive it with an explicit [`AuthContext`] instead
    /// of constructing an rmcp [`RequestContext`].
    pub async fn dispatch_with_meta(
        &self,
        name: &str,
        args: Option<Value>,
        auth: AuthContext,
        rate_limited: bool,
    ) -> Result<CallToolResponse, McpError> {
        // Rate limit (per-token) at the start of the request; only applies when
        // the HTTP transport marked this session rate-limited.
        if rate_limited {
            if let Err(e) = self.apply_rate_limit_result() {
                return Ok(CallToolResult::structured_error(error_json(&e)).into());
            }
        }

        let tool = match self.tool_by_name(name) {
            Some(t) => t,
            None => {
                let e = ErrorDataExt::method_not_found(name.to_string());
                return Err(e.into());
            }
        };

        // Scope check.
        if let Err(e) = self.check_scope(&tool, &auth) {
            self.log(e.kind, name, &e.message);
            return Ok(CallToolResult::structured_error(error_json(&e)).into());
        }

        // Schema validation (contract §5: reject non-conforming input).
        let args = match validate_args(&tool.input_schema, args) {
            Ok(a) => a,
            Err(e) => return Ok(CallToolResult::structured_error(error_json(&e)).into()),
        };

        // Execute.
        match self.execute(tool.name, &args).await {
            Ok(value) => Ok(CallToolResult::structured(value).into()),
            Err(e) => {
                self.log(e.kind, name, &e.message);
                if e.kind == KbErrorKind::Internal || e.kind == KbErrorKind::External {
                    // Protocol-error for server-internal failures (caller sees
                    // the JSON-RPC error, not the message — per rmcp guidance).
                    Err(mcp_error(&e))
                } else {
                    // Tool-level error: caller-visible (NotFound, Conflict,
                    // InvalidArgs, ScopesDenied, RateLimited...).
                    Ok(CallToolResult::structured_error(error_json(&e)).into())
                }
            }
        }
    }

    fn apply_rate_limit_result(&self) -> Result<(), KbError> {
        match &self.rate_limiter {
            Some(bucket) if !bucket.try_acquire() => {
                Err(ErrorDataExt::rate_limited(bucket.retry_after_secs()))
            }
            _ => Ok(()),
        }
    }

    fn log(&self, kind: KbErrorKind, tool: &str, message: &str) {
        debug!(tool, kind = kind.as_str(), message, "mcp tool");
    }

    /// Execute a validated tool call against the vault.
    async fn execute(&self, name: &str, args: &Value) -> Result<Value, KbError> {
        match name {
            "kb_get_page" => {
                let path = req_str(args, "path")?;
                let doc = self.vault.get_page(&path)?;
                Ok(serde_json::to_value(doc).map_err(|e| ErrorDataExt::internal(e.to_string()))?)
            }
            "kb_list_pages" => {
                let limit = req_int(args, "limit", 100).min(1000);
                let cursor = args.get("cursor").and_then(|v| v.as_str());
                let res = self.vault.list_pages(limit, cursor)?;
                Ok(serde_json::to_value(res).map_err(|e| ErrorDataExt::internal(e.to_string()))?)
            }
            "kb_write_page" => {
                let path = req_str(args, "path")?;
                let content = req_str(args, "content")?;
                let expected = args.get("expected_modified").and_then(|v| v.as_str());
                let doc = self.vault.write_page(&path, &content, expected).await?;
                Ok(serde_json::to_value(doc).map_err(|e| ErrorDataExt::internal(e.to_string()))?)
            }
            "kb_delete_page" => {
                let path = req_str(args, "path")?;
                let expected = args.get("expected_modified").and_then(|v| v.as_str());
                self.vault.delete_page(&path, expected).await?;
                Ok(Value::Object(serde_json::Map::new()))
            }
            "kb_reindex" => {
                let mode = args
                    .get("mode")
                    .and_then(|v| v.as_str())
                    .unwrap_or("incremental");
                let status = self.vault.reindex(mode).await?;
                Ok(serde_json::to_value(status)
                    .map_err(|e| ErrorDataExt::internal(e.to_string()))?)
            }
            "kb_index_status" => {
                let status = self.vault.index_status()?;
                Ok(serde_json::to_value(status)
                    .map_err(|e| ErrorDataExt::internal(e.to_string()))?)
            }
            "kb_search" => {
                let query = req_str(args, "query")?;
                let limit = req_int(args, "limit", 20).min(1000);
                let offset = req_int(args, "offset", 0);
                let res = self.vault.search(&query, limit, offset)?;
                Ok(serde_json::to_value(res).map_err(|e| ErrorDataExt::internal(e.to_string()))?)
            }
            "kb_search_by_tag" => {
                let tag = req_str(args, "tag")?;
                let limit = req_int(args, "limit", 50).min(1000);
                let res = self.vault.search_by_tag(&tag, limit)?;
                Ok(serde_json::to_value(res).map_err(|e| ErrorDataExt::internal(e.to_string()))?)
            }
            "kb_autocomplete" => {
                let query = req_str(args, "query")?;
                let kind = args.get("kind").and_then(|v| v.as_str()).unwrap_or("page");
                let limit = req_int(args, "limit", 10).min(100);
                let res = self.vault.autocomplete(&query, kind, limit)?;
                Ok(serde_json::to_value(res).map_err(|e| ErrorDataExt::internal(e.to_string()))?)
            }
            "kb_backlinks" => {
                let path = req_str(args, "path")?;
                let include_unlinked = args
                    .get("include_unlinked")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                let res = self.vault.backlinks(&path, include_unlinked)?;
                Ok(serde_json::to_value(res).map_err(|e| ErrorDataExt::internal(e.to_string()))?)
            }
            "kb_graph" => {
                let path = args.get("path").and_then(|v| v.as_str());
                let depth = req_int(args, "depth", 2).clamp(1, 5);
                let res = self.vault.graph(path, depth)?;
                Ok(serde_json::to_value(res).map_err(|e| ErrorDataExt::internal(e.to_string()))?)
            }
            "kb_resolve_link" => {
                let target = req_str(args, "target")?;
                let res = self.vault.resolve_link(&target)?;
                Ok(serde_json::to_value(res).map_err(|e| ErrorDataExt::internal(e.to_string()))?)
            }
            "kb_add_tag" => {
                let path = req_str(args, "path")?;
                let tag = req_str(args, "tag")?;
                let res = self.vault.add_tag(&path, &tag).await?;
                Ok(serde_json::to_value(res).map_err(|e| ErrorDataExt::internal(e.to_string()))?)
            }
            "kb_remove_tag" => {
                let path = req_str(args, "path")?;
                let tag = req_str(args, "tag")?;
                let res = self.vault.remove_tag(&path, &tag).await?;
                Ok(serde_json::to_value(res).map_err(|e| ErrorDataExt::internal(e.to_string()))?)
            }
            "kb_vault_info" => {
                let res = self.vault.vault_info()?;
                Ok(serde_json::to_value(res).map_err(|e| ErrorDataExt::internal(e.to_string()))?)
            }
            other => Err(ErrorDataExt::method_not_found(other.to_string())),
        }
    }
}

fn req_str(args: &Value, key: &str) -> Result<String, KbError> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            ErrorDataExt::invalid_args(format!("missing required string argument '{key}'"))
        })
}

fn req_int(args: &Value, key: &str, default: usize) -> usize {
    args.get(key)
        .and_then(|v| v.as_u64())
        .map(|v| v as usize)
        .unwrap_or(default)
}

/// Build the JSON-RPC error object for a `KbError`.
pub fn error_json(e: &KbError) -> Value {
    serde_json::json!({
        "error": {
            "code": e.code(),
            "message": e.message.to_string(),
            "data": e.to_data()
        }
    })
}

/// Convert a `KbError` into an `McpError` (JSON-RPC protocol error) for
/// server-internal failures.
pub fn mcp_error(e: &KbError) -> McpError {
    McpError::new(
        rmcp::model::ErrorCode(e.code()),
        e.message.clone(),
        Some(e.to_data()),
    )
}

impl From<KbError> for McpError {
    fn from(e: KbError) -> Self {
        mcp_error(&e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::McpConfig;
    use crate::kbserver::SharedVault;
    use rmcp::model::CallToolResult;
    use tempfile::TempDir;

    /// Build a server over a temporary vault.
    fn test_server() -> (TempDir, Arc<SharedVault>, KbServer) {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join(".pkm")).unwrap();
        let mut cfg = McpConfig::new(dir.path().to_path_buf());
        cfg.transport = crate::config::Transport::Stdio;
        let vault = Arc::new(SharedVault::new(&cfg).unwrap());
        let server = KbServer::new(vault.clone());
        (dir, vault, server)
    }

    /// Call a tool over a real in-memory client session and return the raw
    /// `CallToolResult` (the exact wire shape the contract tests assert on).
    ///
    /// This is the rmcp-3.4 documented pattern: `tokio::io::duplex` connects
    /// a `serve_server` half to a `serve_client` half, so the request goes
    /// through the full protocol stack (initialize handshake, JSON-RPC
    /// framing, `tools/call`).
    async fn call_tool(
        server: KbServer,
        name: &str,
        args: Value,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let (server_tx, client_tx) = tokio::io::duplex(4096);
        // Keep the RunningService alive for the whole test: dropping it closes
        // the transport and breaks the handshake.
        let server_handle = tokio::spawn(async move {
            let running = rmcp::serve_server(server, server_tx).await;
            match running {
                Ok(svc) => {
                    let _ = svc.waiting().await;
                }
                Err(e) => tracing::warn!("server init failed: {e}"),
            }
        });
        let client = rmcp::serve_client((), client_tx)
            .await
            .expect("client initialize");
        let arguments = match args {
            Value::Object(m) => m,
            _ => serde_json::Map::new(),
        };
        let result = client
            .call_tool(
                rmcp::model::CallToolRequestParams::new(name.to_string()).with_arguments(arguments),
            )
            .await
            .map_err(|e| match e {
                rmcp::service::ServiceError::McpError(err) => err,
                other => rmcp::ErrorData::new(
                    rmcp::model::ErrorCode(-32603),
                    format!("transport error: {other}"),
                    None,
                ),
            });
        client.cancel().await.ok();
        let _ = server_handle.await;
        result
    }

    fn json_args(pairs: &[(&str, Value)]) -> Value {
        let mut map = serde_json::Map::new();
        for (k, v) in pairs {
            map.insert(k.to_string(), v.clone());
        }
        Value::Object(map)
    }

    fn structured(result: CallToolResult) -> Value {
        result
            .structured_content
            .expect("structured_content present")
    }

    fn error_code(result: CallToolResult) -> i64 {
        structured(result)["error"]["code"]
            .as_i64()
            .unwrap_or(i64::MIN)
    }

    #[tokio::test]
    async fn test_get_page_missing_returns_not_found() {
        let (_d, _v, server) = test_server();
        let args = json_args(&[("path", Value::from("nope.md"))]);
        let result = call_tool(server, "kb_get_page", args).await.unwrap();
        assert_eq!(
            error_code(result),
            -32001,
            "missing page → NotFound (contract §8.1)"
        );
    }

    #[tokio::test]
    async fn test_write_then_read_roundtrip() {
        let (_d, _v, server) = test_server();
        let write = json_args(&[
            ("path", Value::from("projects/a.md")),
            (
                "content",
                Value::from("# Hello\n\nBody text with [[Other]]."),
            ),
        ]);
        let written = call_tool(server.clone(), "kb_write_page", write)
            .await
            .unwrap();
        assert_eq!(written.is_error, Some(false), "write succeeds");

        let read = json_args(&[("path", Value::from("projects/a.md"))]);
        let result = call_tool(server, "kb_get_page", read).await.unwrap();
        let doc = structured(result);
        assert_eq!(doc["slug"], "projects/a", "roundtrip slug");
        assert!(doc["content"].as_str().unwrap().contains("Hello"));
        assert!(
            doc["links"]
                .as_array()
                .map(|a| a.iter().any(|l| l["target"] == "Other"))
                .unwrap_or(false),
            "wiki-link parsed from content"
        );
    }

    #[tokio::test]
    async fn test_search_without_query_rejected() {
        let (_d, _v, server) = test_server();
        let args = json_args(&[]);
        let result = call_tool(server, "kb_search", args).await.unwrap();
        assert_eq!(error_code(result), -32602, "missing query → InvalidArgs");
    }

    #[tokio::test]
    async fn test_unknown_tool_is_protocol_error() {
        let (_d, _v, server) = test_server();
        let args = json_args(&[]);
        let err = call_tool(server, "kb_nope", args).await.unwrap_err();
        // Unknown method → METHOD_NOT_FOUND (-32601) at the JSON-RPC layer.
        assert_eq!(err.code.0, -32601, "unknown tool is a protocol error");
    }
}
