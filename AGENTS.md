# Stratum — PKM System

A privacy-first, offline-capable personal knowledge management system with native Git sync,
bi-directional linking, graph visualization, and AI-augmented search/chat.
Notes are stored as plain Markdown files on disk — zero vendor lock-in.

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                    Tauri Desktop Shell                        │
│  ┌────────────────────────────────────────────────────────┐  │
│  │              React + TypeScript Frontend                │  │
│  │  ┌──────────┐ ┌──────────┐ ┌────────────────────────┐  │  │
│  │  │ BlockNote │ │  Graph   │ │  Search / Chat / AI   │  │  │
│  │  │ Outliner  │ │ (d3-force)│ │                        │  │  │
│  │  └────┬─────┘ └────┬─────┘ └───────────┬────────────┘  │  │
│  │       │            │                   │                │  │
│  │  ┌────┴────────────┴───────────────────┴───────────┐   │  │
│  │  │     Zustand Stores + Tauri invoke()              │   │  │
│  │  └─────────────────────┬───────────────────────────┘   │  │
│  └────────────────────────┼───────────────────────────────┘  │
│                            │ Tauri IPC                         │
│  ┌────────────────────────┼───────────────────────────────┐  │
│  │               Rust Backend (same process)               │  │
│  │  ┌───────────┬───────────┬───────────┬─────────────┐   │  │
│  │  │ pkm-block │ pkm-index │ pkm-query │ pkm-markdown │   │  │
│  │  │ (SQLite)  │ (Graph +  │ (Datalog) │ (Parser)     │   │  │
│  │  │           │  Tantivy) │           │              │   │  │
│  │  ├───────────┼───────────┼───────────┼─────────────┤   │  │
│  │  │ pkm-sync  │ pkm-      │ pkm-ai    │ pkm-plugin  │   │  │
│  │  │ (git2)    │ watcher   │           │ (WASM)      │   │  │
│  │  └───────────┴───────────┴───────────┴─────────────┘   │  │
│  └─────────────────────────────────────────────────────────┘  │
├───────────────────────────────────────────────────────────────┤
│                    Data Layer                                 │
│  ┌─────────────────────┐  ┌──────────────────────────────┐   │
│  │  .md files          │  │  .pkm/                        │   │
│  │  (block-based,      │  │  blocks.db (SQLite)           │   │
│  │   plain text notes) │  │  search.idx (Tantivy)         │   │
│  │                     │  │  config.toml                  │   │
│  └─────────────────────┘  └──────────────────────────────┘   │
└──────────────────────────────────────────────────────────────┘
```

## Workspace Layout

```
stratum/
├── Cargo.toml                  # Workspace root
├── Cargo.lock
├── AGENTS.md                   # This file
├── rust-toolchain.toml         # Rust toolchain pin
├── .envrc                      # direnv auto-activation
├── nix/                        # Nix flake dev environment
│   ├── flake.nix
│   └── flake.lock
├── crates/
│   ├── pkm-core/               # Core types, config, errors
│   ├── pkm-block/              # Block model, tree, ops, SQLite store
│   ├── pkm-markdown/           # Block-based markdown parser + serializer
│   ├── pkm-index/              # Backlinks, graph, search (Tantivy)
│   ├── pkm-query/              # Datalog query engine
│   ├── pkm-sync/               # Git sync engine (git2)
│   ├── pkm-watcher/            # File system watcher
│   ├── pkm-ai/                 # Embeddings, RAG, LLM provider
│   ├── pkm-plugin/             # WASM plugin runtime
│   └── pkm-cli/                # CLI binary (cargo run -p pkm-cli)
├── src/                        # React + TypeScript frontend
│   ├── main.tsx
│   ├── App.tsx
│   ├── lib/
│   │   ├── types.ts            # TypeScript DTOs
│   │   └── commands.ts         # Tauri invoke() wrappers
│   ├── stores/
│   │   └── appStore.ts         # Zustand store
│   └── components/
│       ├── Sidebar.tsx
│       ├── PageView.tsx
│       ├── BlockEditor.tsx
│       ├── GraphPanel.tsx      # Force-directed graph (d3-force)
│       ├── BacklinksPanel.tsx
│       ├── SearchPanel.tsx
│       ├── QueryPanel.tsx
│       ├── TemplatesPanel.tsx
│       ├── FlashcardsPanel.tsx
│       ├── WhiteboardPanel.tsx
│       ├── JournalPanel.tsx
│       ├── PagesHome.tsx
│       └── SettingsPage.tsx
├── src-tauri/                  # Tauri v2 shell
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── src/
│   │   ├── lib.rs              # App setup, command registration
│   │   └── commands/
│   │       ├── mod.rs
│   │       ├── vault.rs        # VaultState + IndexEngine
│   │       ├── page.rs         # Page CRUD
│   │       ├── block.rs        # Block CRUD
│   │       ├── graph.rs        # Graph data commands
│   │       ├── search.rs       # Full-text + backlinks
│   │       ├── query.rs        # Datalog query
│   │       ├── sync.rs         # Git sync
│   │       ├── template.rs     # Templates
│   │       ├── export.rs       # HTML/JSON export
│   │       ├── flashcards.rs   # SRS flashcards
│  │       ├── whiteboard.rs   # Excalidraw whiteboards
│   │       └── settings.rs     # App settings
│   └── capabilities/
├── docs/
│   ├── architecture.md
│   ├── contributing.md
│   └── master-plan.md
├── package.json
├── tsconfig.json
├── vite.config.ts
└── README.md
```

## Crate Dependency Graph

```
pkm-cli       →  pkm-core, pkm-markdown, pkm-index, pkm-sync, pkm-watcher, pkm-ai, pkm-plugin
src-tauri     →  pkm-core, pkm-block, pkm-markdown, pkm-index, pkm-query, pkm-sync, pkm-watcher
pkm-markdown  →  pkm-core
pkm-index     →  pkm-core, pkm-markdown, pkm-block
pkm-block     →  pkm-core
pkm-query     →  pkm-core, pkm-block
pkm-sync      →  pkm-core, pkm-markdown
pkm-watcher   →  pkm-core, pkm-markdown, pkm-index
pkm-ai        →  pkm-core, pkm-index
pkm-plugin    →  pkm-core
```

## Core Principles

1. **Plain `.md` files on disk** — readable and editable by any tool
2. **No vendor lock-in** — no proprietary format, no cloud dependency
3. **Fully offline** — all features work without internet
4. **Rust core** — all data processing, parsing, indexing, git operations in Rust
5. **Performance** — sub-100ms search at 10k notes, <80MB idle memory
6. **Block-based** — Logseq-style outliner: every paragraph is an addressable block with UUID

## State Management

React app uses **Zustand** for state (single store in `src/stores/appStore.ts`):

- **vault / pages / currentPage** — loaded from Rust backend via `invoke()`
- **loading / error** — global loading and error states
- All data operations go through `src/lib/commands.ts` → Tauri IPC → Rust commands

## Frontend Components

| Component | Purpose |
|-----------|---------|
| `Sidebar` | Navigation (Journal, Pages, Graph, Search, Query, Templates, Flashcards, Whiteboards, Settings) + page tree |
| `PageView` | Block editor + BacklinksPanel + PropertiesPanel |
| `BlockEditor` | Inline block editing with indent/outdent, task markers, drag reorder |
| `GraphPanel` | Force-directed graph visualization (d3-force via react-force-graph-2d): node sizing by degree, tag coloring, hover highlights, search filter, component/orphan views, click to navigate |
| `BacklinksPanel` | Linked references and unlinked mentions for current page |
| `SearchPanel` | Full-text block search via Tantivy |
| `QueryPanel` | Datalog query input with result table |
| `WhiteboardPanel` | Excalidraw spatial canvas |
| `FlashcardsPanel` | Spaced-repetition card review |
| `SettingsPage` | App configuration (vault path, theme, AI, sync, graph settings) |

## Graph Engine

The graph engine (`src-tauri/src/commands/graph.rs`) builds data directly from the SQLite BlockStore — no file I/O or Tantivy index rebuild required:

- **Node/Edge graph** built from `[[wiki-links]]` stored in SQLite blocks
- **Connected components** via BFS on an adjacency list derived from block links
- **Orphaned notes** detection (notes with zero incoming/outgoing connections)
- **Slug resolution**: resolves `[[Title]]` links to note slugs via title lookup
- **Tauri commands**: `get_graph_data`, `get_connected_components`, `get_orphaned_notes`, `rebuild_graph`
- **Frontend**: `GraphPanel` renders force-directed layout (d3-force via `react-force-graph-2d`), with interactive settings panel for d3-force parameters (repulsion, link distance, alpha/friction decay), visibility toggles (connected/orphaned/tags), node search filter, component/orphan view modes, and click-to-navigate

## Sync Modes

| Mode | Description |
|------|-------------|
| Manual | User clicks "sync" — git pull, merge, push |
| Auto-commit | On file save → staged → committed (configurable interval) |
| Auto-sync | Auto-commit + periodic push/pull on a timer |
| Background | Runs as a system service / daemon |

## Build Commands

```bash
# Nix (recommended — provides all dependencies)
nix develop ./nix                # Enter dev shell
direnv allow                     # Or auto-activate via direnv

# Build all Rust crates
cargo build --workspace

# Run all Rust tests
cargo test --workspace

# Build specific crate
cargo build -p pkm-core

# Run CLI
cargo run -p pkm-cli -- --help

# Frontend
npm install                      # Install dependencies
npm run dev                      # Vite dev server (port 5173)
npm run build                    # Production build
npm run lint                     # ESLint

# Tauri desktop app
cargo tauri dev                  # Dev mode (Rust + Vite)
cargo tauri build                # Production bundle
```

## Performance Targets

| Metric | Target |
|--------|--------|
| Cold start (empty vault) | < 500ms |
| Vault with 10k notes — index rebuild | < 30s |
| Vault with 10k notes — full-text search | < 100ms |
| Vault with 10k notes — graph load | < 2s |
| Note save latency | < 50ms (excluding git commit) |
| File watcher debounce | 500ms |
| Memory (idle, desktop) | < 80MB |
| Memory (10k notes, desktop) | < 200MB |
| Bundle size (desktop, compressed) | < 20MB |
