# Stratum

[![CI](https://github.com/short-circuit/stratum/actions/workflows/ci.yml/badge.svg)](https://github.com/short-circuit/stratum/actions/workflows/ci.yml)
[![Release](https://github.com/short-circuit/stratum/actions/workflows/release.yml/badge.svg)](https://github.com/short-circuit/stratum/actions/workflows/release.yml)
[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL--3.0-blue.svg)](LICENSE)

A privacy-first, offline-capable personal knowledge management (PKM) system with
native Git sync, bi-directional linking, graph visualization, and AI-augmented
search/chat. Notes are stored as plain Markdown files on disk — zero vendor lock-in.

> **Status**: v0.3.0 — Alpha. Graph visualization with interactive settings, collapsible panels, reindex-vault command. Rust core and Tauri + React frontend under active development.

## Architecture

Stratum uses a **Rust core** with a **Tauri v2 + React + TypeScript** frontend,
communicating via Tauri IPC (`invoke()`). The Rust workspace contains these crates:

| Crate | Purpose |
|-------|---------|
| `pkm-core` | Core types, config, errors |
| `pkm-block` | Block model (UUID), tree ops, SQLite store |
| `pkm-markdown` | Block-based markdown parser + serializer |
| `pkm-index` | Note graph, backlink resolution, Tantivy full-text search, tag aggregation |
| `pkm-query` | Datalog query engine targeting blocks |
| `pkm-sync` | Git sync engine (git2), auto-commit, conflict resolution |
| `pkm-watcher` | File system watcher (notify) with debounce |
| `pkm-ai` | LLM provider abstraction, embeddings, RAG pipeline |
| `pkm-plugin` | WASM plugin runtime (wasmtime), permission system |
| `pkm-cli` | CLI binary (`stratum` command) |

## Quick Start

### Nix (recommended)

```bash
# Enter dev shell with all dependencies (Rust, Node, Tauri libs)
nix develop ./nix

# Or use direnv for auto-activation
direnv allow

# Build Rust
cargo build --workspace

# Run tests
cargo test --workspace

# Frontend
npm install
npm run dev              # Vite dev server

# Tauri desktop app
cargo tauri dev
```

### Manual Setup

Requires: Rust 1.75+, Node.js 22+, system libraries for Tauri v2 (webkitgtk, glib, gtk3, libsoup, openssl).

```bash
cargo build --workspace
cargo test --workspace
npm install
npm run tauri:dev
```

## Storage Model

Notes are **plain `.md` files** on disk — readable and editable by any text editor.
A hidden `.pkm/` directory in the vault root holds metadata cache:

| File | Purpose |
|------|---------|
| `.pkm/blocks.db` | SQLite block storage |
| `.pkm/search.idx` | Tantivy full-text search index |
| `.pkm/config.toml` | User configuration |

All cache is **rebuildable** from the `.md` files. Deleting `.pkm/` loses no data.

## Features

### Block-Based Outliner
- Every paragraph is an addressable block with UUID
- SQLite storage with O(1) insert/delete/move (parent_id + left_id model)
- Indent/outdent, task markers (TODO/DOING/DONE), priorities
- Block references `((uuid))` and page embeds `{{embed ...}}`

### Graph Engine
- Node/Edge graph built from `[[wiki-links]]` in `.md` files
- Backlink computation with unlinked mention detection
- Connected components via BFS (clusters of interlinked notes)
- Orphaned note detection
- Force-directed visualization with `react-force-graph-2d` (d3-force)
- Click any node to navigate to that note

### Markdown Engine
- YAML frontmatter parsing
- `[[Wiki-link]]` resolution with `[[Target|Display Text]]` support
- `#tag` extraction (frontmatter + inline)
- Block-based serialization with round-trip fidelity

### Full-Text Search
- Tantivy-powered search (sub-100ms at 10k notes)
- Block-level and page-level search
- Regex and full-text search modes

### Datalog Queries
- EDN and JSON dual-mode Datalog input
- Datalog→SQL compiler targeting blocks.db
- `:find` (vars + pull), `:where` patterns, page joins

### Git Sync
- Manual, auto-commit, auto-sync, and background modes
- Remote push/pull via git2
- Merge conflict detection

### AI / Chat
- Pluggable LLM providers: Ollama, OpenAI, Anthropic, Custom
- RAG pipeline: search → context → LLM answer with citations
- Fully offline mode with Ollama

### More
- **Templates** with variable substitution
- **Flashcards** with spaced repetition scheduling
- **Whiteboards** via Tldraw
- **Export** to HTML and JSON

## Project Structure

```
stratum/
├── Cargo.toml              # Workspace root
├── AGENTS.md               # AI assistant context
├── nix/                    # Nix flake dev environment
│   ├── flake.nix
│   └── flake.lock
├── crates/                 # Rust crates
│   ├── pkm-core/
│   ├── pkm-block/
│   ├── pkm-markdown/
│   ├── pkm-index/
│   ├── pkm-query/
│   ├── pkm-sync/
│   ├── pkm-watcher/
│   ├── pkm-ai/
│   ├── pkm-plugin/
│   └── pkm-cli/
├── src/                    # React + TypeScript frontend
│   ├── App.tsx
│   ├── main.tsx
│   ├── lib/
│   ├── stores/
│   └── components/
├── src-tauri/              # Tauri v2 shell + Rust commands
├── docs/
├── package.json
├── tsconfig.json
├── vite.config.ts
└── README.md
```

## Configuration

```toml
[vault]
path = "~/notes"

[sync]
mode = "AutoCommit"
remote_url = "git@github.com:user/vault.git"
branch = "main"
auto_commit_interval_secs = 300
auto_sync_interval_secs = 1800

[theme]
dark_mode = true
font_size = 16

[ai]
provider = "Ollama"
model = "llama3.2"
rag_enabled = true

[watcher]
enabled = true
debounce_ms = 500
```

## License

AGPL-3.0-only — see [LICENSE](LICENSE) for details.
