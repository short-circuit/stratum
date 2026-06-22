# Architecture

## Overview

Stratum is a two-tier application:

```
┌──────────────────────────────────────────┐
│              Flutter UI                   │
│  (Dart, runs on desktop + mobile)        │
├──────────────────────────────────────────┤
│      RustBackend (abstract interface)    │
│      MockBackend (in-memory for dev)     │
│      flutter_rust_bridge (FFI, prod)     │
├──────────────────────────────────────────┤
│             Rust Core                    │
│  (crates/* — all data processing)       │
├──────────────────────────────────────────┤
│           Data Layer                     │
│  (.md files + .pkm/ cache)             │
└──────────────────────────────────────────┘
```

## State Management (Flutter)

The Flutter app uses `provider` (ChangeNotifier) pattern with four providers:

- **VaultProvider** — manages notes (CRUD), graph, tags, stats. Routes all operations through `RustBackend`.
- **SearchProvider** — manages search queries, result list, mode selection (fulltext/graph/regex).
- **SyncProvider** — git sync lifecycle: load status, trigger sync, track conflicts.
- **SettingsProvider** — wraps `AppConfig`, persists to `SharedPreferences`, exposes theme and user settings.

All providers use `MockBackend` during development, which stores notes in memory with sample data. In production, swap to the real Rust backend via `flutter_rust_bridge` FFI (see `crates/pkm-frontend`).

## Rust Crate Dependency Graph

```
pkm-frontend
  ├── pkm-core (foundation types)
  ├── pkm-markdown (depends on pkm-core)
  ├── pkm-index (depends on pkm-core, pkm-markdown)
  ├── pkm-sync (depends on pkm-core, pkm-markdown)
  ├── pkm-watcher (depends on pkm-core, pkm-markdown, pkm-index)
  ├── pkm-ai (depends on pkm-core, pkm-index)
  └── pkm-plugin (depends on pkm-core)
```

## Data Flow

### Opening a Note (Desktop App)

1. User clicks note in sidebar
2. `VaultProvider.openNote(path)` calls `RustBackend.openNote(path)`
3. `MockBackend` returns the in-memory `Note` object
4. `VaultProvider` sets `_currentNote` and notifies listeners
5. `EditorScreen` rebuilds, shows note title, body, tags
6. Switch to Preview mode — `MarkdownRenderer` renders via `flutter_markdown`

### Saving a Note

1. User types in editor → `VaultProvider.updateCurrentBody()` updates the note in memory on each keystroke
2. User clicks Save → `VaultProvider.saveCurrentNote()` → `RustBackend.saveNote()`
3. `MockBackend` updates the in-memory storage
4. In production, Rust would write `.md` to disk, rebuild index, and optionally git-commit

### Graph Rendering

1. `GraphScreen` init → `VaultProvider.loadGraph()` → `MockBackend.getGraph()`
2. Backend returns `GraphLayout` (nodes + edges) built from note backlinks
3. `_GraphPainter` renders nodes/edges with force-directed layout animation
4. Graph auto-refreshes when notes change

### Sync Cycle

1. User presses "Sync" / timer triggers
2. `SyncProvider.sync()` → `RustBackend.syncVault()`
3. In production: `git stash` → `git pull --rebase` → handle conflicts → `git stash pop` → `git push`
4. `SyncProvider` updates status (up_to_date / error / conflicts)

### Settings Persistence

1. `SettingsProvider.loadSettings()` reads from `SharedPreferences` at app start
2. User changes setting → `SettingsProvider` updates `AppConfig` and persists to `SharedPreferences`
3. Watchers react to settings changes (e.g., theme mode, vault path)
