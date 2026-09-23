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

[tts]
# Override endpoint for text-to-speech. Empty = use the AI endpoint above.
endpoint = ""
# Optional bearer token for protected TTS endpoints.
api_key = ""
# Voice name
voice = "alloy"
# Output audio format (mp3, opus, aac, flac, wav)
format = "mp3"
# Playback speed multiplier (0.25–4.0)
speed = 1.0

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
| TTS Endpoint | Override endpoint for text-to-speech (empty = uses AI endpoint) |
| TTS Voice | Voice name used for synthesis (e.g. `alloy`, `onyx`) |
| TTS Format | Output audio format (`mp3`, `opus`, `aac`, `flac`, `wav`) |
| TTS Speed | Playback speed multiplier (0.25–4.0) |
| Test / Play voice | Synthesizes a sample sentence through the configured endpoint and plays it |

!!! note "AI settings availability"
    The AI, Speech & Audio (STT/TTS), and model-capability settings are available on
    **both** the desktop and mobile settings screens. On mobile these appear as a
    collapsible accordion under the **AI** and **Speech & Audio** sections.

### Research Tab

| Setting | Description |
|---------|-------------|
| SearXNG Endpoint | URL of your SearXNG instance |
| Max Results | Max search results per query (1–10) |
| Research Depth | Search-read cycles per research session (1–5) |

### Sync Tab

| Setting | Description |
|---------|-------------|
| Sync Mode | `Manual`, `Auto-Commit`, `Auto-Sync`, or `Background` |
| Remote URL | Git remote (e.g. `git@github.com:user/vault.git`) |
| Branch | Git branch to work on (default `main`) |
| SSH Key Path | Path to an SSH private key; leave empty to use the SSH agent |
| Commit Interval | How often (seconds) auto-commit mode stages and commits changes |
| Commit Message Template | Template for generated commit messages (supports `{datetime}`, `{editedfiles}`, `{newfiles}`, `{deletedfiles}`, `{count}` placeholders) |
| Pull/Push Interval | How often (seconds) auto-sync mode performs a pull/push cycle |
| Sync Now | Manually trigger a pull, merge, and push |
| Start Scheduler | Start the background auto-commit/auto-sync timer |
| Recent Commits | Expandable history of recent git commits |
| Sync Status | Shows current status, branch, ahead/behind counts, and conflict count |

!!! note "Sync availability on mobile"
    Sync configuration is available on mobile under the **Sync** section of the
    settings screen — including mode selector, remote/branch, SSH key, commit
    template with placeholder insertion, sync-now/scheduler controls, and the
    commit log.

### Developer Tab

| Setting | Description |
|---------|-------------|
| Reindex All | Re-sync all pages from disk into the database. Idempotent. Useful after importing notes or recovering from corruption. |
| Repair DB from disk | Rebuild the database from the `.md` files on disk |
| Normalize All Files | Parse every `.md` file through the block parser and re-serialize to normalize indentation, block syntax, and frontmatter |
| Reindex Progress | Live progress bar shown during reindex operations |

!!! note "Mobile accessibility"
    On mobile, reindex **progress** is displayed live under the Developer section.
    The mobile settings screen mirrors the desktop settings surface; each desktop
    tab has a corresponding section on the mobile screen.

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
