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
│   ├── pkm-ai/                 # Embeddings, RAG, LLM provider
│   ├── pkm-audio/              # Microphone capture, FLAC/WAV codecs (dictation)
│   ├── pkm-block/              # Block model, tree, ops, SQLite store
│   ├── pkm-cli/                # CLI binary (cargo run -p pkm-cli)
│   ├── pkm-core/               # Core types, config, errors
│   ├── pkm-dictation/          # Voice dictation: transcribe, summarize, render
│   ├── pkm-index/              # Backlinks, graph, search (Tantivy)
│   ├── pkm-markdown/           # Block-based markdown parser + serializer
│   ├── pkm-plugin/             # WASM plugin runtime
│   ├── pkm-query/              # Datalog query engine
│   ├── pkm-stt/                # Speech-to-text, diarization, speaker recognition
│   ├── pkm-sync/               # Git sync engine (git2)
│   ├── pkm-tests/              # Integration tests for the Rust crates
│   └── pkm-watcher/            # File system watcher
├── src/                        # React + TypeScript frontend
│   ├── main.tsx                # App bootstrap, settings load
│   ├── App.tsx                 # Root layout, routes, close handler
│   ├── global.css              # CSS variables, safe-area, overrides
│   ├── lib/
│   │   ├── types.ts            # TypeScript DTOs (matching Rust structs)
│   │   ├── commands.ts         # 64 Tauri invoke() wrappers
│   │   ├── theme.ts            # CSS variable generation (--primary-* shades)
│   │   ├── muiTheme.ts         # MUI theme creation from config
│   │   ├── wikiLinks.ts        # Wiki-link parsing/serialization
│   │   ├── libraryStore.ts     # Module-level library JSON cache
│   │   ├── useCtrlHeld.ts      # Hook: Ctrl/Meta key tracking
│   │   ├── useMathInline.tsx   # Hook: ProseMirror inline KaTeX plugin
│   │   └── hooks/              # # Custom hooks (useAsyncData, useDebounce, etc.)
│   ├── stores/
│   │   ├── appStore.ts         # Core Zustand store (vault, pages, currentPage)
│   │   ├── settingsStore.ts    # Settings + theme state
│   │   ├── graphStore.ts       # Graph data + settings
│   │   └── syncStore.ts        # Sync status + commits
│   ├── components/
│   │   ├── ui/                 # Atomic reusable UI primitives
│   │   │   ├── LoadingOverlay.tsx
│   │   │   ├── ErrorAlert.tsx
│   │   │   ├── EmptyState.tsx
│   │   │   ├── PageHeader.tsx
│   │   │   ├── ConfirmDialog.tsx
│   │   │   ├── SliderRow.tsx
│   │   │   ├── PassphraseModal.tsx
│   │   │   ├── ConflictModal.tsx
│   │   │   └── index.ts
│   │   ├── Sidebar/
│   │   │   ├── index.tsx
│   │   │   ├── NavItemList.tsx
│   │   │   ├── PageTree.tsx
│   │   │   └── SidebarFooter.tsx
│   │   ├── PageView/
│   │   ├── OutlinerEditor/
│   │   ├── GraphPanel/
│   │   ├── BacklinksPanel/
│   │   ├── SearchPanel/
│   │   ├── QueryPanel/
│   │   ├── JournalPanel/
│   │   ├── PagesHome/
│   │   ├── TemplatesPanel/
│   │   ├── FlashcardsPanel/
│   │   ├── KanbanPanel/
│   │   ├── WhiteboardPanel/
│   │   ├── SettingsPage/
│   │   ├── AISlashMenu.tsx
│   │   ├── AIFormattingToolbar.tsx
│   │   ├── DictationPanel.tsx
│   │   ├── AutocompletePopup.tsx
│   │   ├── LinkPreviewPopup.tsx
│   │   ├── MathEditorModal.tsx
│   │   ├── MathSymbolPalette.tsx
│   │   ├── MermaidBlock.tsx
│   │   ├── MarkerBadge.tsx
│   │   ├── StratumIcon.tsx
│   │   ├── SuggestedConnectionsPanel.tsx
│   │   ├── KanbanEditDialog.tsx
│   │   ├── MobileLayout.tsx
│   │   ├── MobileNav.tsx
│   │   ├── JournalCalendar.tsx
│   │   └── VaultPicker.tsx
│   └── test/
├── src-tauri/                  # Tauri v2 shell
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── src/
│   │   ├── lib.rs              # App setup, 70+ command registrations
│   │   └── commands/
│   │       ├── mod.rs
│   │       ├── vault.rs        # VaultState + IndexEngine + init
│   │       ├── page.rs         # Page CRUD + filesystem sync
│   │       ├── block.rs        # Block CRUD
│   │       ├── graph.rs        # Graph data + components + orphans
│   │       ├── search.rs       # Full-text + backlinks + autocomplete
│   │       ├── query.rs        # Datalog query
│   │       ├── sync.rs         # Git sync
│   │       ├── template.rs     # Templates
│   │       ├── export.rs       # HTML/JSON export
│   │       ├── flashcards.rs   # SRS flashcards
│   │       ├── whiteboard.rs   # Excalidraw whiteboards + library
│   │       ├── settings.rs     # App settings + AI model fetch
│   │       ├── ai.rs           # AI transform + research + interlink
│   │       └── kanban.rs       # Kanban block queries
│   └── capabilities/
├── docs/
│   ├── index.md                # MkDocs home page
│   ├── mkdocs.yml              # MkDocs configuration
│   ├── requirements.txt        # mkdocs-material
│   ├── getting-started/
│   │   ├── installation.md
│   │   ├── quickstart.md
│   │   └── configuration.md
│   ├── guide/                  # 20 user guides (one per feature)
│   │   ├── vault-management.md
│   │   ├── block-editor.md
│   │   ├── linking-and-backlinks.md
│   │   ├── tags.md
│   │   ├── tasks.md
│   │   ├── graph-view.md
│   │   ├── search.md
│   │   ├── datalog-queries.md
│   │   ├── journal.md
│   │   ├── templates.md
│   │   ├── flashcards.md
│   │   ├── kanban.md           # Kanban board guide
│   │   ├── whiteboards.md
│   │   ├── math-equations.md
│   │   ├── diagrams.md
│   │   ├── ai-features.md
│   │   ├── web-research.md
│   │   ├── git-sync.md
│   │   ├── voice-dictation.md
│   │   └── export.md
│   ├── cli/
│   │   └── command-reference.md
│   ├── advanced/
│   │   ├── file-format.md
│   │   ├── plugins.md
│   │   └── component-architecture.md  # Frontend component hierarchy
│   └── development/             # Developer documentation
│       ├── frontend-guide.md    # Component patterns, hooks, state mgmt
│       ├── rust-guide.md        # Crate organization, comment standards
│       └── mobile-guide.md     # Build targets, responsive patterns
├── .sisyphus/
│   └── refactoring-plan.md     # Long-term refactoring roadmap
├── package.json
├── tsconfig.json
├── vite.config.ts
└── README.md
```

## Crate Dependency Graph

```
pkm-core       →  (standalone foundation)
pkm-block      →  pkm-core
pkm-markdown   →  pkm-core, pkm-block
pkm-index      →  pkm-core, pkm-markdown, pkm-block
pkm-query      →  pkm-block
pkm-sync       →  pkm-core, pkm-markdown
pkm-watcher    →  pkm-core, pkm-markdown, pkm-index
pkm-ai         →  pkm-core, pkm-index
pkm-plugin     →  pkm-core
pkm-cli        →  pkm-core, pkm-markdown, pkm-index, pkm-sync, pkm-watcher, pkm-ai, pkm-plugin
src-tauri      →  pkm-core, pkm-block, pkm-markdown, pkm-index, pkm-query, pkm-sync, pkm-watcher, pkm-ai
```

**NOTE**: `src-tauri` does NOT currently depend on `pkm-plugin`. The CLI does NOT depend on `pkm-block` or `pkm-query`.

## Core Principles

1. **Plain `.md` files on disk** — readable and editable by any tool
2. **No vendor lock-in** — no proprietary format, no cloud dependency
3. **Fully offline** — all features work without internet
4. **Rust core** — all data processing, parsing, indexing, git operations in Rust
5. **Performance** — sub-100ms search at 10k notes, <80MB idle memory
6. **Block-based** — Logseq-style outliner: every paragraph is an addressable block with UUID
7. **Minimal reusable components** — no component > 400 lines, no file > 500 lines
8. **Documentation is code** — every feature change requires a corresponding docs update
9. **Platform-aware** — mobile and desktop share logic, differ in presentation

## State Management

React app uses **Zustand** for state. Domain-specific stores in `src/stores/`:

- **appStore** — vault, pages, currentPage, loading, error (the core)
- **settingsStore** — theme, AI, research, sync configuration
- **graphStore** — graph data, connected components, orphans, graph settings
- **syncStore** — sync status, commit log, conflict state

All data operations flow: `component` → `src/lib/commands.ts` (invoke) → Rust command → crate logic.

## Frontend Components

### Panel Components (route-mapped, app-level)

| Component | Route | Purpose | Guide |
|-----------|-------|---------|-------|
| `PagesHome` | `/` | Page list with block counts | — |
| `JournalPanel` | `/journal` | Calendar + daily journal creation | `docs/guide/journal.md` |
| `PageView` | `/page/:pagePath` | Block editor + backlinks + connections | `docs/guide/block-editor.md` |
| `SearchPanel` | `/search` | Full-text + tag search | `docs/guide/search.md` |
| `QueryPanel` | `/query` | Datalog query input + results table | `docs/guide/datalog-queries.md` |
| `GraphPanel` | `/graph` | 3D/2D force-directed graph (desktop/mobile) | `docs/guide/graph-view.md` |
| `TemplatesPanel` | `/templates` | Template list + apply with variables | `docs/guide/templates.md` |
| `FlashcardsPanel` | `/flashcards` | SRS card review (SM-2) | `docs/guide/flashcards.md` |
| `KanbanPanel` | `/kanban` | Drag-and-drop Kanban board | `docs/guide/kanban.md` |
| `WhiteboardPanel` | `/whiteboards` | Excalidraw spatial canvas | `docs/guide/whiteboards.md` |
| `SettingsPage` | `/settings` | 6-tab app configuration | `docs/getting-started/configuration.md` |

### Editor Sub-components

| Component | Parent | Purpose |
|-----------|--------|---------|
| `OutlinerEditor` | `PageView` | BlockNote-based outliner with auto-save, markers, wiki-links — decomposed into `index.tsx` + `dtoConverters.ts` + `markerDetection.ts` |
| `BacklinksPanel` | `PageView` | Linked references + unlinked mentions + hover preview |
| `SuggestedConnectionsPanel` | `PageView` | AI-suggested wiki-link connections |
| `MermaidBlock` | `OutlinerEditor` | Custom BlockNote block for Mermaid diagrams |
| `AISlashMenu` | `OutlinerEditor` | Slash menu with AI actions (rewrite, summarize, etc.) |
| `AIFormattingToolbar` | `OutlinerEditor` | Formatting toolbar with AI buttons |
| `AutocompletePopup` | `OutlinerEditor` | Popover for wiki-link autocomplete |
| `LinkPreviewPopup` | `OutlinerEditor` | Hover preview for wiki-links |
| `MathEditorModal` | `OutlinerEditor` | LaTeX editor with live KaTeX preview |
| `MathSymbolPalette` | `MathEditorModal` | Tabbed symbol palette (Greek, Operators, etc.) |
| `MarkerBadge` | `OutlinerEditor` | Colored chip for task markers (TODO/DOING/DONE) |
| `MarkerSuggestMenu` | `OutlinerEditor` | Autocomplete popup for markers and priorities |
| `KanbanEditDialog` | `KanbanPanel` | Edit card content, marker, priority |

### Navigation & Utility

| Component | Purpose |
|-----------|---------|
| `Sidebar` (`index.tsx`) | Drawer wrapper with collapse state, vault info, header |
| `NavItemList` | Navigation items list (Journal, Graph, Kanban, etc.) |
| `PageTree` | Page list with create/delete and new-page form |
| `SidebarFooter` | Refresh/export/version footer |
| `VaultPicker` | Landing page when no vault is configured |
| `StratumIcon` | App icon SVG renderer |
| `PagesHome` | Home route: page list with block counts |
| `MobileLayout` | Root mobile wrapper with bottom navigation |
| `MobileNav` | Bottom navigation bar for mobile |
| `JournalCalendar` | Calendar popup for date navigation |

### UI Primitives (`src/components/ui/`)

| Component | Purpose | Origin |
|-----------|---------|--------|
| `LoadingOverlay` | Centered spinner with optional message | Shared pattern |
| `ErrorAlert` | Dismissable error Alert | Shared pattern |
| `EmptyState` | Centered empty state with icon + message + action | Shared pattern |
| `PageHeader` | Consistent header bar (title + actions + back) | Shared pattern |
| `ConfirmDialog` | Reusable confirmation dialog | Shared pattern |
| `SliderRow` | Label + slider + display value | Extracted from GraphPanel |
| `PassphraseModal` | SSH key passphrase input dialog | Extracted from SettingsPage |
| `ConflictModal` | Git conflict resolution dialog | Extracted from SettingsPage |
| `ResponsiveDialog` | Full-screen dialog on mobile, normal on desktop | Shared pattern |
| `AILoadingOverlay` | Loading overlay with AI-specific styling | AI features |

### Custom Hooks (`src/lib/hooks/`)

| Hook | Purpose | Used By |
|------|---------|---------|
| `useAsyncData` | Generic async fetch (loading/error/data/refresh) | All panels |
| `useDebounce` | Debounce a value or callback | SearchPanel, OutlinerEditor |
| `useAutoSave` | Debounced auto-save with dirty tracking | OutlinerEditor, WhiteboardPanel |
| `useResponsive` | Breakpoint detection (mobile vs desktop) | Layout components |
| `useCtrlHeld` | Track Ctrl/Meta key held state | OutlinerEditor, BacklinksPanel |
| `useMathInline` | ProseMirror plugin for inline KaTeX | OutlinerEditor |
| `useLongPress` | Detect long-press gestures | MobileNav, MobileLayout |
| `useMarkerDecorations` | ProseMirror decorations for inline marker/priority badges | OutlinerEditor |

### Component Sizing Rules

| Metric | Limit | Action |
|--------|-------|--------|
| File lines | < 500 | Split into sub-modules |
| Component JSX | < 50 lines | Extract sub-components |
| Inline function | < 20 lines | Extract to module-level |
| Same logic in 2+ files | 0 duplicates | Extract to `src/lib/` or `hooks/` |
| Props interface | Required at top | Every component must define `interface Props` |

### Desktop + Mobile Component Pattern

Components that need platform-specific implementations follow this folder convention:

```
src/components/FeaturePanel/
├── index.tsx              # Desktop/web implementation (imports .shared)
├── FeaturePanel.mobile.tsx  # Mobile variant (imports .shared)
├── FeaturePanel.shared.tsx  # Shared logic/hooks/types
└── FeaturePanel.test.tsx    # Tests
```

The `index.tsx` uses a `useResponsive` hook to conditionally render mobile or desktop:

```typescript
export default function FeaturePanel() {
  const { isMobile } = useResponsive();
  if (isMobile) return <FeaturePanelMobile />;
  return <FeaturePanelDesktop />;
}
```

This keeps mobile-specific code from bloating the desktop bundle.

## Graph Engine

The graph engine (`src-tauri/src/commands/graph.rs`) builds data directly from the SQLite BlockStore — no file I/O or Tantivy index rebuild required:

- **Node/Edge graph** built from `[[wiki-links]]` stored in SQLite blocks
- **Connected components** via BFS on an adjacency list derived from block links
- **Orphaned notes** detection (notes with zero incoming/outgoing connections)
- **Slug resolution**: resolves `[[Title]]` links to note slugs via title lookup
- **Tauri commands**: `get_graph_data`, `get_connected_components`, `get_orphaned_notes`, `rebuild_graph`
- **Frontend**: `GraphPanel` renders force-directed layout (d3-force via `react-force-graph-2d` on mobile, `react-force-graph-3d` on desktop), with interactive settings panel for d3-force parameters (repulsion, link distance, alpha/friction decay), visibility toggles (connected/orphaned/tags), node search filter, component/orphan view modes, and click-to-navigate

## Sync Modes

| Mode | Description |
|------|-------------|
| Manual | User clicks "sync" — git pull, merge, push |
| Auto-commit | On file save → staged → committed (configurable interval) |
| Auto-sync | Auto-commit + periodic push/pull on a timer |
| Background | Runs as a system service / daemon |

## Rust Coding Standards

### Comment Requirements

Every Rust source file MUST have:

```rust
//! Module-level doc explaining purpose, what this module provides, how to use it.

/// Doc comment on every public function — what it does, arguments, return, errors, panics.

// Inline comments on:
// - Complex algorithms (state invariants, why this approach)
// - Non-obvious transformations (why this mapping)
// - Workarounds (which issue/limitation is being worked around)
```

Files in `pkm-index/src/` (graph, search, rebuild, tags) are currently under-commented and need prioritized attention.

### Error Handling

Use `PkmError` (from `pkm-core`) as the single error type across all crates. Use `#[from]` derives to convert crate-specific errors. Do NOT define new error types in individual crates — extend `PkmError` variants instead.

### Testing

- Unit tests in `#[cfg(test)]` modules alongside implementation
- Integration tests for command handlers (especially `src-tauri/src/commands/`)
- Use `tempfile` for filesystem tests
- Target: every public function has at least one test

### Module Organization

Command handlers (`src-tauri/src/commands/`) must be **thin glue layers**. Business logic belongs in the appropriate crate:

| Command Handler | Logic Lives In |
|----------------|---------------|
| `graph.rs` | `pkm-block` (graph building) |
| `settings.rs` | `pkm-core::Config` (DTO mapping) |
| `search.rs` | `pkm-index` (search/backlinks) |
| `sync.rs` | `pkm-sync` (git operations) |

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
npm run lint                     # ESLint (no Prettier — formatting is Rust-only)

# Run frontend tests
npm run test

# Tauri desktop app
cargo tauri dev                  # Dev mode (Rust + Vite)
cargo tauri build                # Production bundle
```

## License & Dependency Policy

Stratum is **AGPL-3.0-only**. Every new dependency (Rust crate, npm package) must be license-compatible with AGPL-3.0. The following licenses are always acceptable:
- MIT, Apache-2.0, BSD-2/3-Clause, ISC, Zlib, Unlicense, CC0-1.0, BSL-1.0
- MPL-2.0 (AGPL-compatible per MPL §3.3)
- Apache-2.0 WITH LLVM-exception
- Unicode-3.0
- Dual-licensed dependencies where at least one option is in the above list

**Reject** any dependency that is:
- GPL-2.0-only without a permissive dual-license alternative
- A proprietary or non-OSI-approved license

When in doubt, flag the dependency for review before adding it.

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

## Documentation Sync Rules

**These rules are MANDATORY for every code change.** Every agent and contributor must follow them:

| Trigger | Required Action |
|---------|----------------|
| New component added | Add to `Frontend Components` table above; add `docs/guide/` entry if user-facing |
| Component renamed | Update table above; update all user docs |
| Component > 400 lines | Must split into sub-components (file a refactoring task) |
| New hook/utility | Add to `Custom Hooks` table above; update `docs/development/frontend-guide.md` |
| Rust API changed | Update doc comments; update `docs/development/rust-guide.md` |
| New Rust crate/module | Add to `Crate Dependency Graph` and `Workspace Layout` above; add module docs |
| New feature added | Add `docs/guide/` entry; update contributing.md if dev workflow changes |
| Behavior change | Update relevant `docs/guide/` entry |
| UI changed | Verify screenshots in docs still match; update if needed |
| Mobile platform change | Update `docs/development/mobile-guide.md` |
| New dependency added | Verify AGPL-3.0 compatibility per `License & Dependency Policy` |
| Cargo.toml changed | Verify `Crate Dependency Graph` table above matches actual deps |

### CI Enforcement (planned)

A CI check should verify:
1. Component count in AGENTS.md matches actual files in `src/components/` (minus `ui/` primitives if those are documented separately)
2. All `docs/guide/` entries referenced in the component table exist
3. All `.md` files in `docs/guide/` have a corresponding component documented

## Dependency Consistency

### TypeScript ↔ Rust Type Alignment

`src/lib/types.ts` defines DTOs that must match Rust `#[derive(Serialize)]` structs in `src-tauri/src/commands/`. When either changes, the other MUST be updated in the same PR.

**Currently manual** — planned: ts-rs crate for auto-generation.

### Command API Alignment

`src/lib/commands.ts` wrappers must match Tauri command signatures in `src-tauri/src/commands/`. Every `#[tauri::command]` needs a corresponding typed wrapper in `commands.ts`.

### Code ↔ Docs Alignment

| Artifact | Must Match | Check Frequency |
|----------|-----------|-----------------|
| AGENTS.md component table | Actual files in `src/components/` | Every PR |
| AGENTS.md crate table | Actual crates in `Cargo.toml` workspace | Every PR |
| docs/guide/* | Features actually implemented in code | On feature add |
| contributing.md | Actual CI workflow | On CI change |
| Rust doc comments | Actual Rust API | Every commit |

## Refactoring Roadmap

See `.sisyphus/refactoring-plan.md` for the full phased plan covering:

1. **Frontend**: Shared hooks → UI primitives → decompose 6 monolithic components → store decomposition → mobile patterns
2. **Rust**: Comment audit → eliminate critical duplication → error type unification → test expansion → cross-crate cleanup → fix stub implementations
3. **Docs**: AGENTS.md accuracy → development guides → component architecture docs → screenshot gaps
