use std::sync::Arc;
use pkm_mcp::config::McpConfig;
use pkm_mcp::kbserver::SharedVault;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let dir = tempfile::TempDir::new()?;
    std::fs::create_dir_all(dir.path().join(".pkm"))?;
    let mut cfg = McpConfig::new(dir.path().to_path_buf());
    cfg.transport = pkm_mcp::config::Transport::Stdio;
    let vault = Arc::new(SharedVault::new(&cfg)?);
    vault.write_page("s.md", "unique zebra keyword", None).await?;
    // Inspect blocks in store
    let store = vault.store()?;
    let blocks = store.get_blocks_by_page("s.md")?;
    println!("blocks for s.md: {}", blocks.len());
    for b in &blocks { println!("  id={} content={:?}", b.id, b.content); }
    // Try get_block by uuid
    if let Some(b) = blocks.first() {
        match store.get_block(b.id) {
            Ok(b2) => println!("get_block ok content={:?}", b2.content),
            Err(e) => println!("get_block err: {e}"),
        }
    }
    Ok(())
}
