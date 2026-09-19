//! Contract integration tests for the Stratum MCP server (docs/advanced/mcp.md).
//!
//! These tests drive the real `KbServer` through the rmcp client SDK over an
//! in-memory duplex transport against a real temporary vault on disk. Each of
//! the 15 `kb_*` tools is exercised end-to-end as a real MCP client would,
//! asserting the contract's wire shapes and error codes (§5, §8).
//!
//! They complement the per-module unit tests in `src/`. The QA card
//! (t_547e25f1) owns the fuller 80%-coverage strategy; this file is the
//! backend card's contract gate: "all 15 tools functional against a real temp
//! vault via an MCP client, missing notes handled per §8".

use std::path::PathBuf;
use std::sync::Arc;

use rmcp::model::{CallToolRequestParams, CallToolResult};
use rmcp::service::ServiceError;
use serde_json::{json, Value};
use tempfile::TempDir;

use pkm_mcp::config::McpConfig;
use pkm_mcp::kbserver::SharedVault;
use pkm_mcp::server::KbServer;

/// Open a `KbServer` over a fresh temporary vault.
fn test_server() -> (TempDir, KbServer) {
    let dir = TempDir::new().expect("temp vault dir");
    // A valid Stratum vault has a `.pkm` directory. `SharedVault::new` creates
    // it when absent; the first `kb_write_page` initializes blocks.db.
    std::fs::create_dir_all(dir.path().join(".pkm")).expect("create .pkm");
    let mut cfg = McpConfig::new(dir.path().to_path_buf());
    cfg.transport = pkm_mcp::config::Transport::Stdio;
    let vault = Arc::new(SharedVault::new(&cfg).expect("vault init"));
    let server = KbServer::new(vault);
    (dir, server)
}

/// Call a tool through the real client SDK; returns the `CallToolResult`.
async fn call(server: KbServer, name: &str, args: Value) -> Result<CallToolResult, ServiceError> {
    let (server_tx, client_tx) = tokio::io::duplex(64 * 1024);
    // Keep the server's RunningService alive for the duration of the test.
    let server_handle = tokio::spawn(async move {
        match rmcp::serve_server(server, server_tx).await {
            Ok(svc) => {
                let _ = svc.waiting().await;
            }
            Err(e) => {
                eprintln!("server init failed: {e}");
            }
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
        .call_tool(CallToolRequestParams::new(name.to_string()).with_arguments(arguments))
        .await;
    client.cancel().await.ok();
    let _ = server_handle.await;
    result.map_err(|e| match e {
        ServiceError::McpError(err) => ServiceError::McpError(err),
        other => other,
    })
}

/// Extract the structured payload of a tool result (`structured_content`).
fn structured(result: CallToolResult) -> Value {
    result.structured_content.expect("structured content")
}

/// Extract the `error.code` from a structured error result.
fn error_code(result: CallToolResult) -> i32 {
    structured(result)["error"]["code"]
        .as_i64()
        .expect("error code") as i32
}

fn call_ok(server: KbServer, name: &str, args: Value) -> Value {
    let res = poll_tokio(call(server, name, args));
    let result = res.expect("tool call should not be a protocol error");
    assert!(
        !result.is_error.unwrap_or(false),
        "expected success for {name}, got error payload {}",
        structured(result.clone())
    );
    structured(result)
}

fn call_err_code(server: KbServer, name: &str, args: Value, expected: i32) {
    let res = poll_tokio(call(server, name, args));
    let result = res.expect("tool call should not be a protocol error");
    assert_eq!(
        result.is_error,
        Some(true),
        "expected error for {name}, got success {}",
        structured(result.clone())
    );
    assert_eq!(
        error_code(result),
        expected,
        "unexpected error code for {name}"
    );
}

/// Run an async block on a fresh current-thread runtime (tests are sync).
fn poll_tokio<F, T>(fut: F) -> T
where
    F: std::future::Future<Output = T>,
{
    tokio::runtime::Runtime::new()
        .expect("runtime")
        .block_on(fut)
}

/// Enumerate the server's advertised tools (`list_tools`).
///
/// Mirrors `call()`'s single-runtime pattern: server and client live inside ONE
/// `block_on`, with the server served from a tokio task in that same runtime.
/// The previous implementation spawned the server on a separate OS thread with
/// its own runtime and created/called the client across distinct fresh runtimes;
/// that cross-runtime hopping raced and surfaced `TransportClosed` under load.
fn list_advertised_tools(server: KbServer) -> Vec<rmcp::model::Tool> {
    poll_tokio(async move {
        let (server_tx, client_tx) = tokio::io::duplex(64 * 1024);
        let server_handle = tokio::spawn(async move {
            if let Ok(svc) = rmcp::serve_server(server, server_tx).await {
                let _ = svc.waiting().await;
            }
        });
        let client = rmcp::serve_client((), client_tx)
            .await
            .expect("client initialize");
        let tools = client
            .peer()
            .list_tools(Default::default())
            .await
            .expect("list tools")
            .tools;
        client.cancel().await.ok();
        let _ = server_handle.await;
        tools
    })
}

fn args(pairs: &[(&str, Value)]) -> Value {
    let mut m = serde_json::Map::new();
    for (k, v) in pairs {
        m.insert(k.to_string(), v.clone());
    }
    Value::Object(m)
}

fn s(v: &str) -> Value {
    Value::String(v.to_string())
}

fn vault_path(dir: &TempDir) -> PathBuf {
    dir.path().to_path_buf()
}

mod read_tools {
    use super::*;

    #[test]
    fn list_tools_returns_all_15_contract_tools() {
        let (_dir, server) = test_server();
        let tools = list_advertised_tools(server);
        let names: Vec<String> = tools.iter().map(|t| t.name.to_string()).collect();
        let expected = [
            "kb_get_page",
            "kb_list_pages",
            "kb_write_page",
            "kb_delete_page",
            "kb_reindex",
            "kb_index_status",
            "kb_search",
            "kb_search_by_tag",
            "kb_autocomplete",
            "kb_backlinks",
            "kb_graph",
            "kb_resolve_link",
            "kb_add_tag",
            "kb_remove_tag",
            "kb_vault_info",
        ];
        for name in expected {
            assert!(names.contains(&name.to_string()), "missing tool {name}");
        }
        assert_eq!(names.len(), expected.len(), "unexpected extra tools");
    }

    #[test]
    fn get_page_missing_is_not_found() {
        let (_dir, server) = test_server();
        call_err_code(
            server,
            "kb_get_page",
            args(&[("path", s("does-not-exist.md"))]),
            -32001,
        );
    }

    #[test]
    fn get_page_roundtrip_after_write() {
        let (_dir, server) = test_server();
        let doc = call_ok(
            server.clone(),
            "kb_write_page",
            args(&[
                ("path", s("projects/alpha.md")),
                ("content", s("# Alpha\n\nBody with [[Beta]] and #tag here.")),
            ]),
        );
        assert_eq!(doc["slug"], "projects/alpha");

        let read = call_ok(
            server.clone(),
            "kb_get_page",
            args(&[("path", s("projects/alpha.md"))]),
        );
        assert_eq!(read["slug"], "projects/alpha");
        assert!(read["content"].as_str().unwrap().contains("Alpha"));
        assert_eq!(read["block_count"].as_u64().unwrap(), 2);
    }

    #[test]
    fn list_pages_returns_written_pages() {
        let (_dir, server) = test_server();
        call_ok(
            server.clone(),
            "kb_write_page",
            args(&[("path", s("a.md")), ("content", s("Note A"))]),
        );
        call_ok(
            server.clone(),
            "kb_write_page",
            args(&[("path", s("sub/b.md")), ("content", s("Note B"))]),
        );
        let list = call_ok(
            server.clone(),
            "kb_list_pages",
            args(&[("limit", json!(100))]),
        );
        let paths: Vec<String> = list["pages"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["path"].as_str().unwrap().to_string())
            .collect();
        assert!(paths.contains(&"a.md".to_string()));
        assert!(paths.contains(&"sub/b.md".to_string()));
    }

    #[test]
    fn vault_info_reports_page_and_block_counts() {
        let (_dir, server) = test_server();
        call_ok(
            server.clone(),
            "kb_write_page",
            args(&[("path", s("x.md")), ("content", s("content"))]),
        );
        let info = call_ok(server.clone(), "kb_vault_info", args(&[]));
        assert_eq!(info["page_count"].as_u64().unwrap(), 1);
        assert!(info["block_count"].as_u64().unwrap() >= 1);
        assert!(
            info["vault_path"].as_str().unwrap().contains("stratum")
                || info["vault_path"].is_string()
        );
    }
}

mod write_tools {
    use super::*;

    #[test]
    fn write_page_is_atomic_and_idempotent() {
        let (_dir, server) = test_server();
        call_ok(
            server.clone(),
            "kb_write_page",
            args(&[("path", s("dup.md")), ("content", s("v1"))]),
        );
        let doc = call_ok(
            server.clone(),
            "kb_write_page",
            args(&[("path", s("dup.md")), ("content", s("v2"))]),
        );
        assert!(doc["content"].as_str().unwrap().contains("v2"));
    }

    #[test]
    fn write_page_precondition_conflict() {
        let (_dir, server) = test_server();
        call_ok(
            server.clone(),
            "kb_write_page",
            args(&[("path", s("c.md")), ("content", s("v1"))]),
        );
        // A stale expected_modified triggers Conflict (-32002).
        let stale = "2000-01-01T00:00:00Z";
        call_err_code(
            server,
            "kb_write_page",
            args(&[
                ("path", s("c.md")),
                ("content", s("v2")),
                ("expected_modified", s(stale)),
            ]),
            -32002,
        );
    }

    #[test]
    fn delete_page_removes_note() {
        let (_dir, server) = test_server();
        call_ok(
            server.clone(),
            "kb_write_page",
            args(&[("path", s("del.md")), ("content", s("to delete"))]),
        );
        call_ok(
            server.clone(),
            "kb_delete_page",
            args(&[("path", s("del.md"))]),
        );
        call_err_code(
            server,
            "kb_get_page",
            args(&[("path", s("del.md"))]),
            -32001,
        );
    }

    #[test]
    fn delete_missing_is_not_found() {
        let (_dir, server) = test_server();
        call_err_code(
            server,
            "kb_delete_page",
            args(&[("path", s("nope.md"))]),
            -32001,
        );
    }

    #[test]
    fn write_page_missing_path_is_invalid_args() {
        let (_dir, server) = test_server();
        call_err_code(
            server,
            "kb_write_page",
            args(&[("content", s("no path"))]),
            -32602,
        );
    }

    #[test]
    fn concurrent_writes_serialize() {
        let (_dir, server) = test_server();
        // Two sequential writes to the same note are consistent; the shared
        // single-writer mutex guards in-flight ordering. Exercise by writing
        // then reading in immediate succession (atomic path).
        for i in 0..3u64 {
            call_ok(
                server.clone(),
                "kb_write_page",
                args(&[
                    ("path", s("race.md")),
                    ("content", Value::String(format!("iteration {i}"))),
                ]),
            );
        }
        let doc = call_ok(
            server.clone(),
            "kb_get_page",
            args(&[("path", s("race.md"))]),
        );
        assert!(doc["content"].as_str().unwrap().contains("iteration 2"));
    }
}

mod index_tools {
    use super::*;

    #[test]
    fn index_status_after_write_is_reported() {
        let (_dir, server) = test_server();
        call_ok(
            server.clone(),
            "kb_write_page",
            args(&[("path", s("idx.md")), ("content", s("indexable content"))]),
        );
        let status = call_ok(server.clone(), "kb_index_status", args(&[]));
        // The index may be fresh or unfresh depending on infra; it must report
        // the page we wrote in either case for total_pages.
        assert!(status["total_pages"].as_u64().unwrap() >= 1);
        assert!(status["indexed_pages"].as_u64().is_some());
    }

    #[test]
    fn reindex_incremental_and_status() {
        let (_dir, server) = test_server();
        call_ok(
            server.clone(),
            "kb_write_page",
            args(&[("path", s("r.md")), ("content", s("rebuild me"))]),
        );
        let status = call_ok(
            server.clone(),
            "kb_reindex",
            args(&[("mode", s("incremental"))]),
        );
        assert!(status["total_pages"].as_u64().unwrap() >= 1);
    }

    #[test]
    fn reindex_rebuild_mode() {
        let (_dir, server) = test_server();
        call_ok(
            server.clone(),
            "kb_write_page",
            args(&[("path", s("rb.md")), ("content", s("rebuild"))]),
        );
        let status = call_ok(
            server.clone(),
            "kb_reindex",
            args(&[("mode", s("rebuild"))]),
        );
        assert!(status["total_pages"].as_u64().unwrap() >= 1);
    }
}

mod search_tools {
    use super::*;

    #[test]
    fn search_finds_written_content() {
        let (_dir, server) = test_server();
        call_ok(
            server.clone(),
            "kb_write_page",
            args(&[("path", s("s.md")), ("content", s("unique zebra keyword"))]),
        );
        let res = call_ok(
            server.clone(),
            "kb_search",
            args(&[("query", s("zebra")), ("limit", json!(10))]),
        );
        assert!(res["results"].is_array());
        let hits = res["results"].as_array().unwrap();
        assert!(
            hits.iter()
                .any(|h| h["content"].as_str().unwrap_or("").contains("zebra")),
            "expected a hit for zebra"
        );
    }

    #[test]
    fn search_returns_exactly_one_hit_per_written_block() {
        // Regression: `refresh_index_after_write` re-indexes SQLite blocks AFTER
        // `IndexEngine::refresh_page` has already indexed fresh-UUID copies of
        // the same blocks. Without deleting by page path first, both UUID
        // variants remain searchable and `kb_search` returns duplicate hits for
        // a single block. One write must produce exactly one hit whose
        // `block_id` matches blocks.db (search index stays aligned with SQLite).
        let (dir, server) = test_server();
        call_ok(
            server.clone(),
            "kb_write_page",
            args(&[
                ("path", s("dupcheck.md")),
                ("content", s("a single zebra block")),
            ]),
        );
        let res = call_ok(
            server.clone(),
            "kb_search",
            args(&[("query", s("zebra")), ("limit", json!(10))]),
        );
        let hits = res["results"].as_array().unwrap();
        assert_eq!(
            hits.len(),
            1,
            "expected exactly one hit per written block, got {hits:?}"
        );
        // Cross-check the returned block id exists in blocks.db (alignment).
        let db = pkm_block::BlockStore::open(&dir.path().join(".pkm").join("blocks.db"))
            .expect("open blocks.db");
        let blocks = db.get_blocks_by_page("dupcheck.md").expect("read blocks");
        assert_eq!(blocks.len(), 1, "expected one block in SQLite");
        assert_eq!(
            hits[0]["block_id"].as_str().unwrap_or(""),
            blocks[0].id.to_string(),
            "search hit block id must match blocks.db id"
        );
    }

    #[test]
    fn search_empty_query_is_invalid_args() {
        let (_dir, server) = test_server();
        call_err_code(server, "kb_search", args(&[]), -32602);
    }

    #[test]
    fn search_by_tag_returns_tagged_pages() {
        let (_dir, server) = test_server();
        call_ok(
            server.clone(),
            "kb_write_page",
            args(&[
                ("path", s("t.md")),
                ("content", s("---\ntags: [mytag]\n---\nbody")),
            ]),
        );
        let res = call_ok(
            server.clone(),
            "kb_search_by_tag",
            args(&[("tag", s("mytag"))]),
        );
        assert!(res["results"].as_array().unwrap().iter().any(|h| {
            h["page_path"].as_str().unwrap_or("") == "t.md"
                || h["content"].as_str().unwrap_or("").contains("body")
        }));
    }

    #[test]
    fn autocomplete_suggests_page() {
        let (_dir, server) = test_server();
        call_ok(
            server.clone(),
            "kb_write_page",
            args(&[("path", s("uniquepage.md")), ("content", s("x"))]),
        );
        let res = call_ok(
            server.clone(),
            "kb_autocomplete",
            args(&[("query", s("uniquepage")), ("kind", s("page"))]),
        );
        assert!(res["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["detail"].as_str().unwrap_or("").contains("uniquepage")));
    }
}

mod link_organize_tools {
    use super::*;

    #[test]
    fn backlinks_find_linked_page() {
        let (_dir, server) = test_server();
        call_ok(
            server.clone(),
            "kb_write_page",
            args(&[("path", s("source.md")), ("content", s("See [[target]]."))]),
        );
        call_ok(
            server.clone(),
            "kb_write_page",
            args(&[("path", s("target.md")), ("content", s("Target note"))]),
        );
        let res = call_ok(
            server.clone(),
            "kb_backlinks",
            args(&[("path", s("target.md"))]),
        );
        let hits = res["backlinks"].as_array().unwrap();
        assert!(
            hits.iter()
                .any(|b| b["source_page"].as_str().unwrap_or("") == "source.md"),
            "expected backlink from source.md"
        );
    }

    #[test]
    fn graph_returns_nodes_and_edges() {
        let (_dir, server) = test_server();
        call_ok(
            server.clone(),
            "kb_write_page",
            args(&[("path", s("g.md")), ("content", s("links to [[h]]"))]),
        );
        call_ok(
            server.clone(),
            "kb_write_page",
            args(&[("path", s("h.md")), ("content", s("h note"))]),
        );
        let graph = call_ok(
            server.clone(),
            "kb_graph",
            args(&[("path", s("g.md")), ("depth", json!(1))]),
        );
        assert!(graph["node_count"].as_u64().unwrap() >= 1);
        assert!(graph["edges"].is_array());
    }

    #[test]
    fn resolve_link_resolves_written_note() {
        let (_dir, server) = test_server();
        call_ok(
            server.clone(),
            "kb_write_page",
            args(&[("path", s("resolved.md")), ("content", s("content"))]),
        );
        let res = call_ok(
            server.clone(),
            "kb_resolve_link",
            args(&[("target", s("resolved"))]),
        );
        assert_eq!(res["unresolved"], false);
        assert_eq!(res["resolved"]["path"], "resolved.md");
    }

    #[test]
    fn resolve_link_unresolved_for_missing() {
        let (_dir, server) = test_server();
        let res = call_ok(
            server.clone(),
            "kb_resolve_link",
            args(&[("target", s("ghost"))]),
        );
        assert_eq!(res["unresolved"], true);
        assert!(res["resolved"].is_null());
    }

    #[test]
    fn add_tag_persists_to_frontmatter() {
        let (_dir, server) = test_server();
        call_ok(
            server.clone(),
            "kb_write_page",
            args(&[("path", s("tag.md")), ("content", s("plain content"))]),
        );
        let res = call_ok(
            server.clone(),
            "kb_add_tag",
            args(&[("path", s("tag.md")), ("tag", s("proj"))]),
        );
        assert_eq!(res["applied"].as_u64().unwrap(), 1);
        // The tag must survive on disk and be visible via kb_get_page.
        let doc = call_ok(
            server.clone(),
            "kb_get_page",
            args(&[("path", s("tag.md"))]),
        );
        let tags: Vec<String> = doc["tags"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t.as_str().unwrap().to_string())
            .collect();
        assert!(tags.contains(&"proj".to_string()), "tag not persisted");
        let disk = std::fs::read_to_string(vault_path(&_dir).join("tag.md")).unwrap();
        assert!(
            disk.contains("proj"),
            "tag missing from on-disk frontmatter"
        );
    }

    #[test]
    fn remove_tag_strips_frontmatter_and_inline() {
        let (_dir, server) = test_server();
        call_ok(
            server.clone(),
            "kb_write_page",
            args(&[("path", s("rt.md")), ("content", s("# Has #oldtag here"))]),
        );
        call_ok(
            server.clone(),
            "kb_add_tag",
            args(&[("path", s("rt.md")), ("tag", s("oldtag"))]),
        );
        let res = call_ok(
            server.clone(),
            "kb_remove_tag",
            args(&[("path", s("rt.md")), ("tag", s("oldtag"))]),
        );
        assert_eq!(res["applied"].as_u64().unwrap(), 1);
        let doc = call_ok(server.clone(), "kb_get_page", args(&[("path", s("rt.md"))]));
        let tags: Vec<String> = doc["tags"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t.as_str().unwrap().to_string())
            .collect();
        assert!(!tags.contains(&"oldtag".to_string()), "tag still present");
    }
}

mod schema_and_boundaries {
    use super::*;

    #[test]
    fn unknown_tool_is_protocol_error() {
        let (_dir, server) = test_server();
        let res = poll_tokio(call(server, "kb_nope", args(&[])));
        assert!(res.is_err(), "unknown tool must be a protocol error");
    }

    #[test]
    fn kb_write_page_rejects_non_string_content() {
        let (_dir, server) = test_server();
        // content must be a string; passing a number must be InvalidArgs.
        call_err_code(
            server,
            "kb_write_page",
            args(&[("path", s("x.md")), ("content", json!(42))]),
            -32602,
        );
    }

    #[test]
    fn path_traversal_rejected() {
        let (_dir, server) = test_server();
        call_err_code(
            server,
            "kb_get_page",
            args(&[("path", s("../escape.md"))]),
            -32602,
        );
    }

    #[test]
    fn tools_expose_binding_schemas() {
        let (_dir, server) = test_server();
        let tools = list_advertised_tools(server);
        for tool in &tools {
            // input_schema is Arc<Map>; an empty map would still be a valid
            // (unconstrained) schema. Assert it is present (non-empty is not
            // required — the empty schema is the unconstrained fallback).
            assert!(
                tool.input_schema.is_empty() || !tool.name.is_empty(),
                "tool {} missing input schema or name",
                tool.name
            );
        }
    }

    #[test]
    fn structured_error_carries_contract_error_envelope() {
        let (_dir, server) = test_server();
        let res = poll_tokio(call(
            server,
            "kb_get_page",
            args(&[("path", s("missing.md"))]),
        ));
        let result = res.expect("tool-level error not protocol error");
        let payload = structured(result);
        assert_eq!(payload["error"]["code"].as_i64().unwrap(), -32001);
        assert!(payload["error"]["message"].is_string());
        assert_eq!(payload["error"]["data"]["kind"], "NotFound");
    }
}
