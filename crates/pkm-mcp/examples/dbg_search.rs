// Debug integration search via full MCP client
use std::sync::Arc;
use rmcp::model::{CallToolRequestParams, CallToolResult};
use serde_json::{Value, json};
use pkm_mcp::config::McpConfig;
use pkm_mcp::kbserver::SharedVault;
use pkm_mcp::server::KbServer;

#[tokio::main]
async fn main() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path().join(".pkm")).unwrap();
    let mut cfg = McpConfig::new(dir.path().to_path_buf());
    cfg.transport = pkm_mcp::config::Transport::Stdio;
    let vault = Arc::new(SharedVault::new(&cfg).unwrap());
    let server = KbServer::new(vault);

    async fn call(server: KbServer, name: &str, args: Value) -> Result<CallToolResult, rmcp::service::ServiceError> {
        let (st, ct) = tokio::io::duplex(64*1024);
        let sh = tokio::spawn(async move {
            if let Ok(svc) = rmcp::serve_server(server, st).await { let _ = svc.waiting().await; }
        });
        let client = rmcp::serve_client((), ct).await.unwrap();
        let m = match args { Value::Object(m) => m, _ => serde_json::Map::new() };
        let r = client.call_tool(CallToolRequestParams::new(name.to_string()).with_arguments(m)).await;
        client.cancel().await.ok();
        let _ = sh.await;
        r
    }

    let w = call(server.clone(), "kb_write_page", json!({"path":"s.md","content":"unique zebra keyword"})).await;
    println!("write: is_err={} payload={:?}", w.is_err(), w.ok().and_then(|r| r.structured_content));

    let s = call(server.clone(), "kb_search", json!({"query":"zebra","limit":10})).await;
    match s {
        Ok(r) => println!("search ok: is_error={:?} payload={}", r.is_error, serde_json::to_string_pretty(&r.structured_content).unwrap()),
        Err(e) => println!("search err: {e}"),
    }
}
