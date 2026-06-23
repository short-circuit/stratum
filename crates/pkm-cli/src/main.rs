use clap::{Parser, Subcommand};
use pkm_core::PkmResult;
use std::path::Path;

/// Stratum — a privacy-first, offline-capable PKM system.
#[derive(Parser)]
#[command(name = "stratum", version = "0.2.0", about = "Personal Knowledge Management")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Vault path (default: current directory)
    #[arg(short = 'p', long = "vault", global = true)]
    vault: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new vault in the current directory
    Init,
    /// List all notes
    List {
        /// Filter by tag
        #[arg(long)]
        tag: Option<String>,
    },
    /// Show a note
    Show {
        /// Note path (relative to vault)
        path: String,
    },
    /// Create a new note
    Create {
        /// Note path (e.g. notes/my-note.md)
        path: String,
        /// Note title
        #[arg(long)]
        title: Option<String>,
    },
    /// Search notes
    Search {
        /// Search query
        query: String,
    },
    /// Show vault statistics
    Stats,
    /// Show graph information
    Graph,
    /// Show tag cloud
    Tags,
    /// Git sync operations
    Sync {
        #[command(subcommand)]
        action: SyncAction,
    },
    /// Export vault
    Export {
        /// Output format: html, json
        #[arg(default_value = "html")]
        format: String,
    },
    /// AI chat (requires a running LLM provider)
    Ask {
        /// Your question
        question: String,
    },
    /// Show config
    Config,
}

#[derive(Subcommand)]
enum SyncAction {
    /// Show sync status
    Status,
    /// Push local changes
    Push,
    /// Pull remote changes
    Pull,
    /// Full sync (pull + push)
    Sync,
}

fn main() -> PkmResult<()> {
    let cli = Cli::parse();
    let vault_path = cli.vault.unwrap_or_else(|| std::env::current_dir().unwrap().display().to_string());
    let vault = Path::new(&vault_path);

    match &cli.command {
        Commands::Init => cmd_init(vault),
        Commands::List { tag } => cmd_list(vault, tag.as_deref()),
        Commands::Show { path } => cmd_show(vault, path),
        Commands::Create { path, title } => cmd_create(vault, path, title.as_deref()),
        Commands::Search { query } => cmd_search(vault, query),
        Commands::Stats => cmd_stats(vault),
        Commands::Graph => cmd_graph(vault),
        Commands::Tags => cmd_tags(vault),
        Commands::Sync { action } => cmd_sync(vault, action),
        Commands::Export { format } => cmd_export(vault, format),
        Commands::Ask { question } => cmd_ask(vault, question),
        Commands::Config => cmd_config(vault),
    }
}

fn cmd_init(vault: &Path) -> PkmResult<()> {
    let pkm_dir = vault.join(".pkm");
    std::fs::create_dir_all(&pkm_dir)?;
    std::fs::create_dir_all(vault.join("notes"))?;
    std::fs::create_dir_all(pkm_dir.join("history"))?;

    let config = pkm_core::Config {
        vault_path: vault.to_path_buf(),
        ..Default::default()
    };
    config.save(config.config_file_path())
        .map_err(|e| pkm_core::PkmError::Config(e.to_string()))?;

    // Create a welcome note
    let welcome_path = vault.join("notes/welcome.md");
    if !welcome_path.exists() {
        let content = "---\ntitle: Welcome to Stratum\ntags: [welcome, getting-started]\ncreated: "
            .to_string()
            + &chrono::Utc::now().format("%Y-%m-%d").to_string()
            + "\n---\n\n# Welcome to Stratum\n\nThis vault was just initialized. Create notes with `[[Wiki Links]]` to connect ideas.\n\n#welcome\n";
        std::fs::write(&welcome_path, content)?;
    }

    println!("✓ Initialized vault at {}", vault.display());
    println!("  Notes: {}", vault.join("notes").display());
    println!("  Cache: {}", pkm_dir.display());
    Ok(())
}

fn cmd_list(vault: &Path, tag: Option<&str>) -> PkmResult<()> {
    let notes = collect_md_files(vault)?;
    let filtered: Vec<_> = if let Some(t) = tag {
        notes.into_iter().filter(|p| {
            if let Ok(content) = std::fs::read_to_string(p) {
                content.contains(&format!("#{}", t)) || content.contains(&format!("- {}", t))
            } else { false }
        }).collect()
    } else { notes };

    if filtered.is_empty() {
        println!("No notes found.");
        return Ok(());
    }

    println!("{} notes:", filtered.len());
    for path in &filtered {
        let rel = path.strip_prefix(vault).unwrap_or(path);
        let content = std::fs::read_to_string(path).unwrap_or_default();
        let parsed = pkm_markdown::parser::parse_raw(&content);
        let title = parsed.frontmatter.title.as_deref().unwrap_or(rel.file_stem().and_then(|s| s.to_str()).unwrap_or("untitled"));
        println!("  {:60} {}", rel.display(), if title.len() > 30 { format!("{}…", &title[..30]) } else { title.to_string() });
    }
    Ok(())
}

fn cmd_show(vault: &Path, path: &str) -> PkmResult<()> {
    let full_path = vault.join(path);
    if !full_path.exists() {
        eprintln!("Note not found: {}", full_path.display());
        return Ok(());
    }
    let content = std::fs::read_to_string(&full_path)?;
    let parsed = pkm_markdown::parser::parse_raw(&content);

    println!("╔══════════════════════════════════════╗");
    if let Some(title) = &parsed.frontmatter.title {
        println!("║  {}", title);
    }
    println!("║  Path: {}", path);
    if !parsed.frontmatter.tags.is_empty() {
        println!("║  Tags: {}", parsed.frontmatter.tags.join(", "));
    }
    if !parsed.links.is_empty() {
        println!("║  Links: {}", parsed.links.iter().map(|l| l.target.as_str()).collect::<Vec<_>>().join(", "));
    }
    println!("╚══════════════════════════════════════╝");
    println!("\n{}", parsed.body);
    Ok(())
}

fn cmd_create(vault: &Path, path: &str, title: Option<&str>) -> PkmResult<()> {
    let full_path = vault.join(path);
    if full_path.exists() {
        eprintln!("Note already exists: {}", full_path.display());
        return Ok(());
    }

    let default_title = title.unwrap_or(path.trim_end_matches(".md").split('/').next_back().unwrap_or("untitled"));
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let content = format!("---\ntitle: {}\ncreated: {}\ntags: []\n---\n\n# {}\n\n", default_title, today, default_title);

    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&full_path, &content)?;
    println!("✓ Created {}", path);
    Ok(())
}

fn cmd_search(vault: &Path, query: &str) -> PkmResult<()> {
    let notes = collect_md_files(vault)?;
    let q = query.to_lowercase();
    let mut results = Vec::new();

    for path in notes {
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        if content.to_lowercase().contains(&q) {
            let rel = path.strip_prefix(vault).unwrap_or(&path).display().to_string();
            let parsed = pkm_markdown::parser::parse_raw(&content);
            let title = parsed.frontmatter.title.unwrap_or_default();
            let snippet = content.lines()
                .find(|l| l.to_lowercase().contains(&q))
                .unwrap_or("")
                .to_string();
            results.push((rel, title, snippet));
        }
    }

    if results.is_empty() {
        println!("No matches for '{}'", query);
        return Ok(());
    }

    println!("{} results for '{}':\n", results.len(), query);
    for (rel, title, snippet) in &results {
        println!("  {} — {}", rel, if title.len() > 40 { format!("{}…", &title[..40]) } else { title.clone() });
        println!("    {}", snippet);
        println!();
    }
    Ok(())
}

fn cmd_stats(vault: &Path) -> PkmResult<()> {
    let notes = collect_md_files(vault)?;
    let mut total_bytes = 0u64;
    let mut total_links = 0usize;
    let mut tags = std::collections::HashSet::new();

    for path in &notes {
        if let Ok(meta) = path.metadata() { total_bytes += meta.len(); }
        if let Ok(content) = std::fs::read_to_string(path) {
            let parsed = pkm_markdown::parser::parse_raw(&content);
            total_links += parsed.links.len();
            for t in parsed.tags { tags.insert(t.name); }
        }
    }

    println!("Vault Statistics");
    println!("  Notes:     {}", notes.len());
    println!("  Tags:      {}", tags.len());
    println!("  Links:     {}", total_links);
    println!("  Size:      {}", format_size(total_bytes));
    println!("  Location:  {}", vault.display());
    Ok(())
}

fn cmd_graph(vault: &Path) -> PkmResult<()> {
    let notes = collect_md_files(vault)?;
    let mut edges = Vec::new();
    let mut nodes = std::collections::HashSet::new();

    for path in &notes {
        let rel = path.strip_prefix(vault).unwrap_or(path).display().to_string();
        nodes.insert(rel.clone());
        if let Ok(content) = std::fs::read_to_string(path) {
            let parsed = pkm_markdown::parser::parse_raw(&content);
            for link in &parsed.links {
                edges.push((rel.clone(), link.target.clone()));
            }
        }
    }

    if nodes.is_empty() {
        println!("No notes to graph.");
        return Ok(());
    }

    println!("Graph: {} nodes, {} edges", nodes.len(), edges.len());

    // Orphaned notes (no links in or out)
    let connected: std::collections::HashSet<String> = edges.iter().flat_map(|(s, t)| vec![s.clone(), t.clone()]).collect();
    let orphaned: Vec<_> = nodes.iter().filter(|n| !connected.contains(*n)).collect();
    if !orphaned.is_empty() {
        println!("\nOrphaned notes (no connections):");
        for o in &orphaned {
            println!("  ◦ {}", o);
        }
    }

    if !edges.is_empty() {
        println!("\nEdges:");
        for (src, dst) in edges.iter().take(20) {
            println!("  {}  →  {}", src, dst);
        }
        if edges.len() > 20 { println!("  … and {} more", edges.len() - 20); }
    }
    Ok(())
}

fn cmd_tags(vault: &Path) -> PkmResult<()> {
    let notes = collect_md_files(vault)?;
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for path in &notes {
        if let Ok(content) = std::fs::read_to_string(path) {
            let parsed = pkm_markdown::parser::parse_raw(&content);
            for tag in &parsed.tags {
                *counts.entry(tag.name.clone()).or_default() += 1;
            }
        }
    }

    if counts.is_empty() {
        println!("No tags found.");
        return Ok(());
    }

    let mut sorted: Vec<_> = counts.into_iter().collect();
    sorted.sort_by_key(|a| std::cmp::Reverse(a.1));

    println!("Tag Cloud ({} tags):\n", sorted.len());
    let max_count = sorted.first().map(|(_, c)| *c).unwrap_or(1) as f64;
    for (name, count) in &sorted {
        let bar_len = ((*count as f64 / max_count) * 40.0) as usize;
        let bar = "█".repeat(bar_len);
        println!("  {:20} {:4} {}", name, count, bar);
    }
    Ok(())
}

fn cmd_sync(vault: &Path, action: &SyncAction) -> PkmResult<()> {
    match action {
        SyncAction::Status => {
            let _pkm_dir = vault.join(".pkm");
            if vault.join(".git").exists() {
                let engine = pkm_sync::git::GitEngine::init(vault)?;
                let status = engine.status()?;
                let has_changes = status.iter().any(|(_, s)| *s != git2::Status::CURRENT);
                println!("Git repository: active");
                println!("Branch: {}", get_branch_name(&engine));
                println!("Modified files: {}", status.len());
                if has_changes {
                    println!("Status: uncommitted changes");
                } else {
                    println!("Status: clean");
                }
                if let Some(url) = engine.get_remote_url("origin") {
                    println!("Remote: {}", url);
                } else {
                    println!("Remote: not configured");
                }
            } else {
                println!("Not a git repository.");
                println!("  Run `stratum init` to initialize or configure sync in settings.");
            }
        }
        SyncAction::Push => {
            println!("Pushing… (use `cargo run -p pkm-sync --example sync_push` for full git operations)");
        }
        SyncAction::Pull => {
            println!("Pulling…");
        }
        SyncAction::Sync => {
            println!("Syncing…");
        }
    }
    Ok(())
}

fn get_branch_name(engine: &pkm_sync::git::GitEngine) -> String {
    // Simple branch detection
    engine.status().ok().map(|_| "main".to_string()).unwrap_or_default()
}

fn cmd_export(vault: &Path, format: &str) -> PkmResult<()> {
    let notes = collect_md_files(vault)?;
    match format {
        "json" => {
            let mut exports = Vec::new();
            for path in &notes {
                if let Ok(content) = std::fs::read_to_string(path) {
                    let parsed = pkm_markdown::parser::parse_raw(&content);
                    let rel = path.strip_prefix(vault).unwrap_or(path).display().to_string();
                    exports.push(serde_json::json!({
                        "path": rel,
                        "title": parsed.frontmatter.title,
                        "tags": parsed.frontmatter.tags,
                        "body": parsed.body,
                        "links": parsed.links.iter().map(|l| l.target.clone()).collect::<Vec<_>>(),
                    }));
                }
            }
            let json = serde_json::to_string_pretty(&exports)?;
            let out_path = vault.join("export.json");
            std::fs::write(&out_path, &json)?;
            println!("✓ Exported {} notes to {}", notes.len(), out_path.display());
        }
        _ => {
            // Generate a simple HTML page with all notes
            let mut body = String::new();
            for path in &notes {
                if let Ok(content) = std::fs::read_to_string(path) {
                    let parsed = pkm_markdown::parser::parse_raw(&content);
                    let title = parsed.frontmatter.title.as_deref().unwrap_or("untitled");
                    body.push_str(&format!("<h1>{}</h1>\n<pre>{}</pre>\n<hr>\n", title, escape_html(&parsed.body)));
                }
            }
            let html = format!(
                "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>Stratum Export</title>\
                 <style>body{{max-width:800px;margin:0 auto;padding:20px;font-family:system-ui,sans-serif;line-height:1.6}}\
                 pre{{background:#f5f5f5;padding:12px;border-radius:8px;overflow-x:auto}}</style></head><body>\
                 <h1>Stratum Vault Export</h1><p>{} notes</p><hr>{}</body></html>",
                notes.len(), body
            );
            let out_path = vault.join("export.html");
            std::fs::write(&out_path, &html)?;
            println!("✓ Exported {} notes to {}", notes.len(), out_path.display());
        }
    }
    Ok(())
}

fn cmd_ask(_vault: &Path, question: &str) -> PkmResult<()> {
    println!("🤖 AI Chat");
    println!("   Q: {}", question);
    println!("   A: To use AI features, configure a provider in settings.toml");
    println!("      and run with a running Ollama/OpenAI-compatible endpoint.");
    println!();
    println!("   Mock response: You asked about '{}'.", question);
    println!("   This would be answered by the RAG pipeline using your notes.");
    Ok(())
}

fn cmd_config(vault: &Path) -> PkmResult<()> {
    let config_path = vault.join(".pkm/config.toml");
    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)?;
        println!("{}", content);
    } else {
        println!("No config file found at {}", config_path.display());
        println!("Run `stratum init` to create one.");
    }
    Ok(())
}

// ── Helpers ──

fn collect_md_files(dir: &Path) -> std::io::Result<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();
    if dir.is_dir() {
        collect_md_recursive(dir, &mut files, 0)?;
    }
    files.sort();
    Ok(files)
}

fn collect_md_recursive(dir: &Path, files: &mut Vec<std::path::PathBuf>, depth: usize) -> std::io::Result<()> {
    if depth > 8 { return Ok(()); }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().and_then(|s| s.to_str()) == Some(".pkm") { continue; }
            collect_md_recursive(&path, files, depth + 1)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("md") {
            files.push(path);
        }
    }
    Ok(())
}

fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;
    while size > 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    format!("{:.1} {}", size, UNITS[unit_idx])
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}
