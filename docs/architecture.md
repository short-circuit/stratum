# Architecture

## Overview

Stratum is a Tauri v2 desktop application with a Rust backend and React + TypeScript frontend.

```
┌──────────────────────────────────────────┐
│        Tauri Desktop Shell                │
│  ┌────────────────────────────────────┐  │
│  │   React + TypeScript Frontend      │  │
│  │   (Vite, Tailwind, Zustand)        │  │
│  ├────────────────────────────────────┤  │
│  │   Tauri IPC (invoke())             │  │
│  ├────────────────────────────────────┤  │
│  │   Rust Backend (src-tauri)         │  │
│  │   ┌──────────────────────────┐     │  │
│  │   │  Commands (28 handlers)   │     │  │
│  │   │  vault | page | block |   │     │  │
│  │   │  graph | search | query | │     │  │
│  │   │  sync | template | export │     │  │
│  │   │  flashcards | whiteboard  │     │  │
│  │   │  settings                 │     │  │
│  │   └──────────┬───────────────┘     │  │
│  │              │                      │  │
│  │   ┌──────────┴───────────────┐     │  │
│  │   │  Rust Crates (core logic) │     │  │
│  │   │  pkm-core  pkm-block     │     │  │
│  │   │  pkm-markdown pkm-index  │     │  │
│  │   │  pkm-query pkm-sync      │     │  │
│  │   │  pkm-watcher pkm-ai      │     │  │
│  │   │  pkm-plugin              │     │  │
│  │   └──────────────────────────┘     │  │
│  └────────────────────────────────────┘  │
├──────────────────────────────────────────┤
│           Data Layer                      │
│  .md files + .pkm/ (SQLite + Tantivy)    │
└──────────────────────────────────────────┘
```

## State Management

The React frontend uses **Zustand** (single store in `src/stores/appStore.ts`):

```typescript
interface AppState {
  vault: VaultInfo | null;       // Vault metadata
  pages: PageDto[];              // Page list for sidebar
  currentPage: PageDto | null;   // Currently opened page
  loading: boolean;
  error: string | null;

  loadVault(): Promise<void>;
  loadPages(): Promise<void>;
  openPage(path: string): Promise<void>;
  createPage(path: string, title?: string): Promise<void>;
  deletePage(path: string): Promise<void>;
}
```

All data operations go through `src/lib/commands.ts` → `invoke()` → Rust commands.
Components are stateless where possible, reading from Zustand.

## Backend State

The Rust backend uses an `AppState = Mutex<VaultState>` managed by Tauri:

```rust
pub struct VaultState {
    pub vault_path: PathBuf,
    pub db_path: PathBuf,                  // .pkm/blocks.db
    pub index_engine: Option<IndexEngine>,  // Lazy-initialized
}
```

`IndexEngine` (`pkm-index/src/indexer.rs`) orchestrates:
- **Graph** — note-level nodes/edges from `[[wiki-links]]`
- **TantivyIndex** — full-text search index
- **TagAggregator** — hierarchical tag cloud

## Crate Dependency Graph

```
src-tauri
  ├── pkm-core        (foundation types)
  ├── pkm-block       (depends on pkm-core)
  ├── pkm-markdown    (depends on pkm-core)
  ├── pkm-index       (depends on pkm-core, pkm-markdown, pkm-block)
  ├── pkm-query       (depends on pkm-core, pkm-block)
  ├── pkm-sync        (depends on pkm-core, pkm-markdown)
  └── pkm-watcher     (depends on pkm-core, pkm-markdown, pkm-index)
```

## Data Flow

### Opening a Page

1. User clicks page in sidebar
2. `useStore().openPage(path)` calls `api.openPage(path)`
3. `invoke('open_page', { path })` → Rust `commands::page::open_page`
4. Rust reads `.md` file, parses blocks via `pkm-markdown`, returns `PageDto`
5. Zustand sets `currentPage`, `PageView` renders blocks in `BlockEditor`

### Saving Blocks

1. User types in `BlockEditor` → local state updates
2. On save, `api.saveBlocks(pagePath, blocks)` → `invoke('save_blocks', ...)`
3. Rust serializes blocks to `.md` via `pkm-markdown`, writes to disk
4. Optionally triggers git auto-commit via `pkm-sync`

### Graph Rendering

1. User navigates to `/graph` → `GraphPanel` mounts
2. `loadData()` calls `api.getGraphData()`, `api.getConnectedComponents()`, `api.getOrphanedNotes()`
3. Rust `IndexEngine::rebuild_all()` scans all `.md` files in vault
4. `Graph` builds nodes/edges from `[[wiki-links]]`
5. Tauri commands return `GraphDataDto` (nodes + edges), `ComponentDto[]`, `OrphanDto[]`
6. `GraphPanel` renders force-directed layout with `react-force-graph-2d`
7. Click a node → navigate to that page

### Full-Text Search

1. User types in `SearchPanel` → `api.searchBlocks(query)`
2. `invoke('search_blocks', { query })` → Rust `commands::search::search_blocks`
3. Rust queries Tantivy `BlockIndex` (or `TantivyIndex` for page-level)
4. Returns `SearchResultsDto` with snippets and scores

### Datalog Query

1. User enters Datalog in `QueryPanel` → `api.runQuery(datalog)`
2. `invoke('run_query', { datalog })` → Rust `commands::query::run_query`
3. `pkm-query` parses Datalog, compiles to SQL, executes against `blocks.db`
4. Returns `QueryResultDto` with columns and rows

### Git Sync

1. User clicks Sync or timer triggers
2. `api.syncVault()` → `invoke('sync_vault')` → Rust `commands::sync::sync_vault`
3. `pkm-sync` executes git operations via `git2`

### Settings Persistence

1. Settings stored in `.pkm/config.toml`
2. `get_settings` / `save_settings` commands read/write TOML
3. Theme changes apply immediately via CSS custom properties
