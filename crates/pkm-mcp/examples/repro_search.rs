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
    println!("wrote page; now search");
    match vault.search("zebra", 10, 0) {
        Ok(res) => {
            println!("search returned {} hits", res.results.len());
            for h in res.results.iter() { println!("  hit: {} [{}]", h.page_path, h.content); }
        }
        Err(e) => println!("search error: {:?}", e),
    }
    let idx = dir.path().join(".pkm/search/blocks");
    if idx.exists() {
        for e in std::fs::read_dir(&idx)? { println!("  idxfile {}", e?.file_name().to_string_lossy()); }
    } else {
        println!("NO INDEX DIR");
    }
    println!("db: {} file: {}", dir.path().join(".pkm/blocks.db").exists(), dir.path().join("s.md").exists());
    Ok(())
}
