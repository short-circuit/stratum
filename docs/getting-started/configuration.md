# Configuration

Stratum stores its configuration in `.pkm/config.toml` inside your vault directory.

## Quick Access

You can edit most settings through the **Settings** panel (:material-cog: in the sidebar).
The configuration file is also directly editable — Stratum auto-reloads on changes.

## Configuration File Reference

```toml
[vault]
# Path to the vault root directory
path = "~/StratumVault"

[sync]
# Sync mode: Manual, AutoCommit, AutoSync
mode = "AutoCommit"
# Git remote URL
remote_url = "git@github.com:user/vault.git"
# Git branch
branch = "main"
# Auto-commit interval (seconds)
auto_commit_interval_secs = 300
# Auto-sync interval (seconds)
auto_sync_interval_secs = 1800

[theme]
# Enable dark mode
dark_mode = true
# Primary accent color (hex)
primary_color = "#f97316"
# Secondary color (hex)
secondary_color = "#6b7280"
# Base font size (pixels)
font_size = 16

[ai]
# LLM provider: ollama, openai, anthropic, google, zai, custom
provider = "ollama"
# API endpoint URL (for ollama, custom, zai)
endpoint = "http://localhost:11434"
# API key (for openai, anthropic, google)
api_key = ""
# Default chat model
model = "llama3.2"
# List of models and their capabilities
models = [
  { name = "llama3.2", capabilities = ["chat"] },
  { name = "nomic-embed-text", capabilities = ["embedding"] },
]
# Enable RAG (Retrieval-Augmented Generation)
rag_enabled = true
# Number of chunks to include in RAG context
rag_chunk_count = 5

[research]
# SearXNG endpoint for web research
searxng_endpoint = "http://localhost:8888"
# Max search results per query
max_results = 3
# Research depth (search-read cycles)
max_depth = 2

[watcher]
# Enable file system watcher
enabled = true
# Debounce interval (milliseconds)
debounce_ms = 500

[graph]
# Show connected components
show_connected = true
# Show orphaned notes
show_orphaned = true
# Show tag nodes
show_tags = true
# Force-directed layout: charge strength (negative = repulsion)
charge_strength = -30
# Force-directed layout: link distance
link_distance = 100
# Force-directed layout: alpha decay rate
alpha_decay = 0.02
# Force-directed layout: velocity decay rate
velocity_decay = 0.4
```

## Settings Panel Reference

### Vault Tab

| Setting | Description |
|---------|-------------|
| Vault Path | Absolute path to your vault directory. Click Browse to pick a folder. |

### Theme Tab

| Setting | Description |
|---------|-------------|
| Dark Mode | Toggle dark/light theme |
| Primary Color | Accent color for buttons, links, active elements |
| Secondary Color | Color for backgrounds, borders, UI chrome |
| Font Size | Base editor font size (12–28px) |

### AI Tab

| Setting | Description |
|---------|-------------|
| Provider | LLM backend: Ollama (local), OpenAI, Anthropic, Google AI, Z.AI, Custom |
| API Endpoint | URL for Ollama/custom providers |
| API Key | API key for cloud providers |
| Default Chat Model | Model name for chat/transform operations |
| Fetch Models | Query the provider for available models |
| Model Capabilities | Assign capabilities to each model: `chat`, `embedding`, `tts` |
| Enable RAG | Toggle retrieval-augmented generation |
| RAG Chunk Count | Number of context chunks (1–20) |

### Research Tab

| Setting | Description |
|---------|-------------|
| SearXNG Endpoint | URL of your SearXNG instance |
| Max Results | Max search results per query (1–10) |
| Research Depth | Search-read cycles per research session (1–5) |

### Developer Tab

| Setting | Description |
|---------|-------------|
| Reindex All | Re-sync all pages from disk into the database. Idempotent. Useful after importing notes or recovering from corruption. |

## File Structure

```
your-vault/
├── note-a.md
├── note-b.md
├── journals/
│   └── 2026-06-22.md
├── whiteboards/            # Excalidraw data
├── templates/              # Reusable templates
└── .pkm/                   # Cache (rebuildable)
    ├── blocks.db           # SQLite block storage
    ├── search.idx          # Tantivy full-text index
    └── config.toml         # This configuration file
```

!!! tip "The `.pkm/` cache is rebuildable"
    You can safely delete `.pkm/` — Stratum rebuilds it from your `.md` files.
    No data loss.
