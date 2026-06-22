# Stratum — PKM System

A privacy-first, offline-capable personal knowledge management system with native Git sync,
bi-directional linking, graph visualization, and AI-augmented search/chat.
Notes are stored as plain Markdown files on disk — zero vendor lock-in.

## Architecture

```
┌──────────────────────────────────────────────────┐
│                   Flutter UI                      │
│  ┌─────────┐ ┌──────────┐ ┌──────────────────┐  │
│  │  Editor  │ │  Graph   │ │  Search / Chat   │  │
│  └────┬────┘ └────┬─────┘ └────────┬─────────┘  │
│       │           │                │             │
│  ┌────┴───────────┴────────────────┴──────────┐  │
│  │    Providers (ChangeNotifier + Provider)    │  │
│  │  VaultProvider | SearchProvider | SyncProv  │  │
│  │  SettingsProvider                           │  │
│  └────────────────┬───────────────────────────┘  │
├───────────────────┼──────────────────────────────┤
│    RustBackend (abstract) / MockBackend (dev)    │
├───────────────────┼──────────────────────────────┤
│                    Rust Core                      │
│  ┌──────────┬─────┴──────┬─────────────┬──────┐  │
│  │ Markdown │   Index    │   Git       │ File │  │
│  │ Parser   │  Engine    │   Engine    │Watchr│  │
│  ├──────────┼────────────┼─────────────┼──────┤  │
│  │ Backlink │  Search    │   Sync      │Export│  │
│  │ Resolver │  (Tantivy) │   Scheduler │Engine│  │
│  └──────────┴────────────┴─────────────┴──────┘  │
│  ┌───────────────────────────────────────────┐   │
│  │       Plugin Runtime (WASM)               │   │
│  └───────────────────────────────────────────┘   │
│  ┌───────────────────────────────────────────┐   │
│  │         pkm-cli (CLI binary)              │   │
│  └───────────────────────────────────────────┘   │
├──────────────────────────────────────────────────┤
│              Data Layer (filesystem)              │
│  ┌────────────────┐  ┌──────────────────────┐    │
│  │  .md files     │  │  .pkm/               │    │
│  │  (your notes)  │  │  metadata cache      │    │
│  └────────────────┘  │  (SQLite, Tantivy    │    │
│                       │   index, config)     │    │
│                       └──────────────────────┘    │
└──────────────────────────────────────────────────┘
```

## Workspace Layout

```
stratum/
├── Cargo.toml                  # Workspace root
├── AGENTS.md                   # This file
├── crates/
│   ├── pkm-core/               # Core types, config, errors
│   ├── pkm-markdown/           # Markdown parser + renderer
│   ├── pkm-index/              # Backlinks, graph, search (Tantivy)
│   ├── pkm-sync/               # Git sync engine (git2)
│   ├── pkm-watcher/            # File system watcher
│   ├── pkm-ai/                 # Embeddings, RAG, LLM provider
│   ├── pkm-plugin/             # WASM plugin runtime
│   ├── pkm-frontend/           # Flutter FFI bridge types
│   └── pkm-cli/                # CLI binary (cargo run -p pkm-cli)
├── frontend/                   # Flutter app (separate build)
│   ├── lib/
│   │   ├── main.dart
│   │   ├── models/
│   │   │   ├── note.dart       # Note, Frontmatter, Link, Tag, Backlink
│   │   │   ├── graph.dart      # GraphNode, GraphEdge, GraphLayout, ForceNode
│   │   │   └── config.dart     # AppConfig, SyncConfig, ThemeConfig, AiConfig
│   │   ├── providers/
│   │   │   ├── vault_provider.dart   # Notes, graph, tags, stats
│   │   │   ├── search_provider.dart  # Full-text, graph, regex search
│   │   │   ├── sync_provider.dart    # Git sync status + operations
│   │   │   └── settings_provider.dart # App config + theme
│   │   ├── screens/
│   │   │   ├── home_screen.dart
│   │   │   ├── editor_screen.dart
│   │   │   ├── graph_screen.dart
│   │   │   ├── search_screen.dart
│   │   │   └── settings_screen.dart
│   │   ├── services/
│   │   │   ├── rust_backend.dart     # Abstract backend interface
│   │   │   └── mock_backend.dart     # In-memory dev backend
│   │   └── widgets/
│   │       ├── sidebar.dart
│   │       ├── markdown_renderer.dart
│   │       ├── backlinks_panel.dart
│   │       ├── note_card.dart
│   │       ├── tag_chip.dart
│   │       └── status_bar.dart
│   ├── rust/                   # Generated flutter_rust_bridge bindings
│   ├── pubspec.yaml
│   └── android/ ios/ windows/ macos/ linux/
├── docs/
│   ├── architecture.md
│   └── contributing.md
├── scripts/
│   ├── build.sh
│   └── release.sh
└── README.md
```

## Crate Dependency Graph

```
pkm-frontend  →  pkm-core, pkm-markdown, pkm-index, pkm-sync, pkm-watcher, pkm-ai, pkm-plugin
pkm-cli       →  pkm-core, pkm-markdown, pkm-index, pkm-sync, pkm-watcher, pkm-ai, pkm-plugin
pkm-markdown  →  pkm-core
pkm-index     →  pkm-core, pkm-markdown
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

## State Management

Flutter app uses `provider` (ChangeNotifier) pattern:

- **VaultProvider** — manages note list, current note, graph, tags, stats. All data operations go through `RustBackend` abstraction.
- **SearchProvider** — handles search queries, results, and mode selection (fulltext/graph/regex).
- **SyncProvider** — git sync status and operations.
- **SettingsProvider** — app configuration (theme, vault path, AI, sync settings), persisted via SharedPreferences.

During development, all providers use `MockBackend` which stores notes in memory with sample data.
For production, swap to the real Rust backend via `flutter_rust_bridge` FFI.

## Sync Modes

| Mode | Description |
|------|-------------|
| Manual | User clicks "sync" — git pull, merge, push |
| Auto-commit | On file save → staged → committed (configurable interval) |
| Auto-sync | Auto-commit + periodic push/pull on a timer |
| Background | Runs as a system service / daemon |

## Build Commands

```bash
# Build all Rust crates
cargo build --workspace

# Run all Rust tests
cargo test --workspace

# Build with all features
cargo build --workspace --all-features

# Build specific crate
cargo build -p pkm-core

# Run CLI
cargo run -p pkm-cli -- --help

# Flutter (requires Flutter SDK)
cd frontend
flutter pub get
flutter run -d linux    # or -d chrome, -d macos, etc.
flutter test
flutter analyze

# Combined development (Rust + Flutter)
# In CI, build Rust first, then Flutter
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

## Known Issues

- **Settings**: The settings screen now works with `SettingsProvider` and `SharedPreferences`.
- **Graph**: Force-directed layout with animation — refreshes when vault changes.
- **Markdown Preview**: Fixed layout conflict between `SingleChildScrollView` and `Markdown`'s internal `ListView`.
