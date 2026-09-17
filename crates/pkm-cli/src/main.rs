use clap::{Parser, Subcommand};
use commands::{
    cmd_ask, cmd_config, cmd_create, cmd_export, cmd_graph, cmd_index, cmd_init, cmd_list,
    cmd_search, cmd_show, cmd_stats, cmd_sync, cmd_tags,
};
use pkm_ai::provider::{ChatConfig, ProviderFactory};
use pkm_core::PkmResult;
use pkm_index::indexer::IndexEngine;
use std::path::Path;

mod commands;

/// Stratum — a privacy-first, offline-capable PKM system.
#[derive(Parser)]
#[command(
    name = "stratum",
    version = "0.2.0",
    about = "Personal Knowledge Management"
)]
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
    /// RAG query: retrieve relevant notes and answer using the AI endpoint
    Rag {
        /// Your question
        question: String,
        /// Rebuild the vault search index from .md files before querying
        #[arg(long)]
        index: bool,
        /// Number of chunks to retrieve (default: 5)
        #[arg(long, default_value_t = 5)]
        top_k: usize,
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
    let vault_path = cli
        .vault
        .unwrap_or_else(|| std::env::current_dir().unwrap().display().to_string());
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
        Commands::Rag {
            question,
            index,
            top_k,
        } => cmd_rag(vault, question, *index, *top_k),
        Commands::Config => cmd_config(vault),
    }
}

#[tokio::main]
async fn cmd_rag(vault: &Path, question: &str, index: bool, top_k: usize) -> PkmResult<()> {
    if index {
        cmd_index(vault)?;
    }

    let config_path = vault.join(".pkm").join("config.toml");
    let config = if config_path.exists() {
        pkm_core::Config::load(&config_path).map_err(|e| {
            pkm_core::PkmError::Config(format!("Failed to load {}: {}", config_path.display(), e))
        })?
    } else {
        eprintln!("No config found at {}", config_path.display());
        eprintln!("Run `stratum init` to create a vault, then configure the AI endpoint.");
        return Ok(());
    };

    if !config.ai.rag_enabled {
        eprintln!("RAG is disabled in the vault config (set rag_enabled = true under [ai]).");
    }

    let embedding = pkm_ai::embedding::OpenAIEmbeddingClient::from_ai_config(&config.ai)?;
    let provider = ProviderFactory::create(&config.ai)?;
    let engine = IndexEngine::new(vault)?;

    let rag = pkm_ai::rag::RagEngine::new(engine, Box::new(embedding), provider);

    let chat_config = ChatConfig::new(&config.ai.model);
    let response = rag.query(question, &chat_config, top_k).await?;

    println!("🤖 Answer:\n{}", response.answer);
    if !response.citations.is_empty() {
        println!("\nSources:");
        for (i, c) in response.citations.iter().enumerate() {
            println!("  [{}] {} (score: {:.3})", i + 1, c.path, c.score);
        }
    }
    if response.usage.total() > 0 {
        println!(
            "\nTokens: {} prompt / {} completion",
            response.usage.prompt_tokens, response.usage.completion_tokens
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests;
