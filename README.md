# Stratum

A privacy-first, offline-capable personal knowledge management (PKM) system with
native Git sync, bi-directional linking, graph visualization, and AI-augmented
search/chat. Notes are stored as plain Markdown files on disk — zero vendor lock-in.

> **Status**: Pre-alpha. Core engine in Rust, Flutter frontend under development.

## Architecture

Stratum uses a **Rust core** with a **Flutter (Dart) frontend**, communicating
via `flutter_rust_bridge` FFI. The Rust workspace contains these crates:

| Crate | Purpose |
|-------|---------|
| `pkm-core` | Core types, config, errors |
| `pkm-markdown` | Markdown parser, wiki-link resolver, tag extractor, renderer |
| `pkm-index` | Backlink graph, Tantivy full-text search, tag aggregation |
| `pkm-sync` | Git sync engine (git2), auto-commit, conflict resolution |
| `pkm-watcher` | File system watcher (notify) with debounce |
| `pkm-ai` | LLM provider abstraction, embeddings, RAG pipeline |
| `pkm-plugin` | WASM plugin runtime (wasmtime), permission system |
| `pkm-frontend` | FFI bridge types and API layer for Flutter |
| `pkm-cli` | CLI binary (`stratum` command) |

## Quick Start

```bash
# Setup — checks prerequisites, installs git hooks
./scripts/setup.sh

# Build Rust crates + Flutter frontend for your host platform
./scripts/build.sh

# Run all tests (Rust + Flutter + lints)
./scripts/test.sh

# Run tests quickly (skip lints)
./scripts/test.sh --quick

# Run the CLI
cargo run -p pkm-cli -- --help
```

## Build Scripts

### `./scripts/setup.sh`
Checks that all required tooling is installed: Rust 1.75+, Flutter, platform SDKs.
Optionally installs missing components and sets up git pre-commit hooks.

```bash
./scripts/setup.sh            # Full setup
./scripts/setup.sh --ci       # CI mode — check only, no installs
./scripts/setup.sh --rust-only
```

### `./scripts/build.sh`
Builds the Rust workspace and Flutter frontend for development.

```bash
./scripts/build.sh                              # Debug build for host platform
./scripts/build.sh --release                    # Release build
./scripts/build.sh --platform linux             # Build Flutter for specific target
./scripts/build.sh --rust-only                  # Build Rust only
./scripts/build.sh --release --platform macos
```

Supported Flutter targets: `linux`, `macos`, `windows`, `android`, `ios`, `web`.
Defaults to host platform if `--platform` is omitted.

### `./scripts/test.sh`
Runs the full test suite across Rust and Flutter.

```bash
./scripts/test.sh                  # Full suite
./scripts/test.sh --quick          # Skip lints and slow checks
./scripts/test.sh --coverage       # Include code coverage (requires cargo-tarpaulin)
./scripts/test.sh --rust-only
./scripts/test.sh --flutter-only
```

### `./scripts/release.sh`
Full release pipeline: test, tag, build artifacts for all platforms.

```bash
./scripts/release.sh                   # Auto-detect version, build all
./scripts/release.sh 0.2.0             # Explicit version
./scripts/release.sh --skip-tag        # Build without git tagging
./scripts/release.sh --rust-only
```

Release artifacts are placed in `release/v<version>/`.

## Storage Model

Notes are **plain `.md` files** on disk — readable and editable by any text editor.
A hidden `.pkm/` directory in the vault root holds metadata cache:

| File | Purpose |
|------|---------|
| `.pkm/config.toml` | User configuration |
| `.pkm/cache.db` | SQLite cache (backlinks, graph edges) |
| `.pkm/search.idx` | Tantivy full-text search index |
| `.pkm/history/` | Version history snapshots |

All cache is **rebuildable** from the `.md` files. Deleting `.pkm/` loses no data.

## Features

### Markdown Engine
- YAML frontmatter parsing
- `[[Wiki-link]]` resolution with `[[Target|Display Text]]` support
- `#tag` extraction (frontmatter + inline)
- Round-trip safe renderer (parse → modify → render preserves content)

### Index Engine
- Full-text search via Tantivy (sub-100ms at 10k notes)
- Backlink computation and unlinked mention detection
- Graph: force-directed layout data for visualization
- Tag aggregation with hierarchical tag tree
- Full rebuild from `.md` files

### Git Sync
- Manual, auto-commit, auto-sync, and background modes
- Remote push/pull via git2
- Merge conflict detection and `.sync-conflict.md` markers
- Commit history with diff support

### File Watcher
- Real-time filesystem monitoring via `notify`
- 500ms debounce to prevent thrashing
- Ignores `.pkm/` directory changes
- Filters to `.md` files only

### AI / Chat
- Pluggable LLM providers: Ollama, OpenAI, Anthropic, Custom
- RAG pipeline: search → context → LLM answer with citations
- Local embedding support (LLaMA.cpp / ONNX compatible)
- Fully offline mode with Ollama

### Plugin System
- WASM-compiled plugins via wasmtime
- Capability-based permission system (file read/write, network, git)
- Hook events: onSave, onOpen, onLink, onSearch
- Plugin registry with enable/disable lifecycle

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

## Development

### Prerequisites
- Rust 1.75+ (MSRV) — [rustup.rs](https://rustup.rs)
- Flutter SDK — [flutter.dev](https://docs.flutter.dev/get-started/install)

### Project Structure
```
stratum/
├── Cargo.toml          # Workspace root
├── AGENTS.md           # AI assistant context
├── crates/             # Rust crates
│   ├── pkm-core/
│   ├── pkm-markdown/
│   ├── pkm-index/
│   ├── pkm-sync/
│   ├── pkm-watcher/
│   ├── pkm-ai/
│   ├── pkm-plugin/
│   ├── pkm-frontend/
│   └── pkm-cli/
├── frontend/           # Flutter app
├── docs/               # Documentation
├── scripts/            # Build/release/test/setup scripts
│   ├── setup.sh
│   ├── build.sh
│   ├── test.sh
│   └── release.sh
└── README.md
```

## License

AGPL-3.0-only — see [LICENSE](LICENSE) for details.
