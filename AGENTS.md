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
│   ├── pkm-ai/                 # Embeddings, RAG, LLM provider, local-endpoint discovery (E10.F2)
│   ├── pkm-audio/              # Microphone capture, FLAC/WAV codecs (dictation)
│   ├── pkm-block/              # Block model, tree, ops, SQLite store
│   ├── pkm-cli/                # CLI binary (cargo run -p pkm-cli)
│   ├── pkm-core/               # Core types, config, errors
│   ├── pkm-dictation/          # Voice dictation: transcribe, summarize, render
│   ├── pkm-index/              # Backlinks, graph, search (Tantivy)
│   ├── pkm-import/             # One-click vault importers (Obsidian, Logseq, generic Markdown)
│   ├── pkm-markdown/           # Block-based markdown parser + serializer
│   ├── pkm-mcp/                # MCP server exposing the KB to external AI clients (stdio + Streamable HTTP)
│   ├── pkm-mcp-security/       # Security controls for the MCP server (PAT hashing, scope, rate limit)
│   ├── pkm-plugin/             # WASM plugin runtime
│   ├── pkm-query/              # Datalog query engine
│   ├── pkm-stt/                # Speech-to-text, diarization, speaker recognition
│   ├── pkm-sync/               # Git sync engine (git2) + periodic cloud/folder backup mirror
│   ├── pkm-tests/              # Integration tests for the Rust crates
│   ├── pkm-watcher/            # File system watcher
│   ├── stratum-app/            # Headless service layer behind every Tauri command (no tauri deps)
│   ├── stratum-canvas/         # Excalidraw-compatible board model: elements, tools, roughr scene, SVG/PNG/WebP export
│   ├── stratum-editor/         # Headless block-editor engine: document, spans, state, undo, menus (no UI deps)
│   ├── stratum-graph/          # Deterministic d3-force-3d physics, camera parity, headless wgpu 3D renderer + CPU fallback
│   ├── stratum-i18n/           # Embedded locale catalogs, interpolation and parity gates (native shell)
│   └── stratum-ui/             # Native GPUI desktop shell (E2) on top of stratum-app
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
│   │   ├── pageTree.ts          # Folder-tree grouping util (pages_view="tree")
│   │   ├── libraryStore.ts     # Module-level library JSON cache
│   │   ├── useCtrlHeld.ts      # Hook: Ctrl/Meta key tracking
│   │   ├── useMathInline.tsx   # Hook: ProseMirror inline KaTeX plugin
│   │   └── hooks/              # # Custom hooks (useAsyncData, useDebounce, etc.)
│   ├── i18n/                   # i18next bootstrap; locales + parity/gate tests
│   │   ├── index.ts            # initI18n, setLocale, Intl formatting helpers
│   │   ├── locales/            # en/de/it/fr/es (full) + zh/ja/el (placeholders)
│   │   ├── i18nGate.test.ts    # Static gate: no hardcoded user-facing strings
│   │   └── localeParity.test.ts# Locale key-set parity + untranslated scan
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
│   │   │   ├── settings/
│   │   │   │   ├── index.ts
│   │   │   │   ├── themeConstants.ts
│   │   │   │   ├── SettingsSection.tsx
│   │   │   │   ├── SettingsTab.tsx
│   │   │   │   ├── SettingsAccordion.tsx
│   │   │   │   ├── SettingsRow.tsx
│   │   │   │   ├── ThemeColorEditor.tsx
│   │   │   │   ├── ThemeFontSizeSlider.tsx
│   │   │   │   └── settingsPrimitives.test.tsx
│   │   │   ├── PassphraseModal.tsx
│   │   │   ├── ConflictModal.tsx
│   │   │   └── index.ts
│   │   ├── Sidebar/
│   │   │   ├── index.tsx
│   │   │   ├── NavItemList.tsx
│   │   │   ├── MovePageDialog.tsx
│   │   │   ├── MovePageDialog.test.tsx
│   │   │   ├── PageTree.tsx
│   │   │   ├── PageTreeGrouped.tsx
│   │   │   └── SidebarFooter.tsx
│   │   ├── PageView/
│   │   ├── OutlinerEditor/
│   │   │   ├── WikiLinkAutocomplete.tsx
│   │   │   ├── wikiLinkTargets.ts       # Duplicate-slug disambiguation util
│   │   │   └── wikiLinkTargets.test.ts  # Unit tests for link-target selection
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
│   │   ├── BacklinkSnippetBody.tsx
│   │   ├── BacklinkSnippetPopup.tsx
│   │   ├── MathEditorModal.tsx
│   │   ├── MathSymbolPalette.tsx
│   │   ├── MermaidBlock.tsx
│   │   ├── MarkerBadge.tsx
│   │   ├── StratumIcon.tsx
│   │   ├── SuggestedConnectionsPanel.tsx
│   │   ├── AttachmentPanel.tsx
│   │   ├── KanbanEditDialog.tsx
│   │   ├── MobileLayout.tsx
│   │   ├── MobileNav.tsx
│   │   ├── JournalCalendar.tsx
│   │   ├── VoiceCaptureButton.tsx
│   │   ├── VoiceCaptureButton.desktop.tsx
│   │   ├── VoiceCaptureButton.mobile.tsx
│   │   ├── VoiceCaptureButton.shared.tsx
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
│   │       ├── export.rs        # Export module root: HTML/JSON export, registers pdf/zip submodules
│   │       ├── export/pdf.rs    # Per-page PDF export via headless Chromium (MathML + Mermaid)
│   │       ├── export/zip.rs    # Portable vault ZIP export (manifest.json, excludes .pkm/.git)
│   │       ├── flashcards.rs   # SRS flashcards
│   │       ├── whiteboard.rs   # Excalidraw whiteboards + library
│   │       ├── settings.rs     # App settings + AI model fetch
│   │       ├── ai.rs           # AI transform + research + interlink
│   │       ├── kanban.rs       # Kanban block queries
│   │       ├── reminders.rs    # Reminder list/snooze/clear commands
│   │       └── tray.rs         # System tray + global-shortcut Quick Capture (desktop)
│   ├── reminder_scheduler.rs   # In-process block reminder scheduler (30s tick)
│   └── capabilities/
├── docs/
│   ├── index.md                # MkDocs home page
│   ├── mkdocs.yml              # MkDocs configuration
│   ├── requirements.txt        # mkdocs-material
│   ├── getting-started/
│   │   ├── installation.md
│   │   ├── quickstart.md
│   │   └── configuration.md
│   ├── guide/                  # 21 user guides (one per feature)
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
│   │   ├── export.md
│   │   └── localization.md     # UI language + i18n gates
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
pkm-import     →  pkm-core, pkm-markdown, pkm-block
pkm-query      →  pkm-block
pkm-sync       →  pkm-core, pkm-markdown
pkm-watcher    →  pkm-core, pkm-markdown, pkm-index
pkm-ai         →  pkm-core, pkm-index
pkm-plugin     →  pkm-core
pkm-mcp        →  pkm-core, pkm-block, pkm-markdown, pkm-index, pkm-query
pkm-mcp-security →  (standalone security primitives)
pkm-audio      →  pkm-core
pkm-stt        →  pkm-core, pkm-audio
pkm-dictation  →  pkm-core, pkm-audio, pkm-stt
stratum-app    →  pkm-core, pkm-block, pkm-markdown, pkm-index, pkm-import, pkm-sync, pkm-watcher, pkm-ai, pkm-audio, pkm-stt, pkm-query, pkm-plugin, pkm-dictation
pkm-tests      →  pkm-core, pkm-block, pkm-markdown, pkm-index, pkm-sync, pkm-query, pkm-ai, pkm-plugin, stratum-tauri
pkm-cli        →  pkm-core, pkm-markdown, pkm-index, pkm-sync, pkm-watcher, pkm-ai, pkm-plugin
stratum-editor →  pkm-core, pkm-block, pkm-markdown
stratum-canvas →  pkm-core
stratum-graph  →  pkm-core
stratum-i18n   →  pkm-core
stratum-ui     →  gpui-kit (external), rodio (external), stratum-i18n, stratum-app, stratum-editor, stratum-graph, stratum-canvas, pkm-core
src-tauri      →  pkm-core, pkm-block, pkm-markdown, pkm-index, pkm-import, pkm-query, pkm-sync, pkm-watcher, pkm-ai, pkm-audio, pkm-stt, pkm-dictation, pkm-plugin, stratum-app
```

**NOTE**: `stratum-app` has no dependency on `tauri` or any `tauri-plugin-*` crate — that isolation is the point of the extraction. The CLI does NOT depend on `pkm-block` or `pkm-query`.

**NOTE**: `src-tauri` additionally depends on `tauri-plugin-notification = "2"` (MIT OR Apache-2.0, AGPL-3.0 compatible) for block-reminder OS notifications.

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
- **navigationStore** — modifier-click navigation state (source-editor scroll restore on return; session-scoped)
- **onboardingStore** — first-run wizard state (step, template/sample toggles, install orchestration)

All data operations flow: `component` → `src/lib/commands.ts` (invoke) → Rust command → crate logic.

## Frontend Components

### Panel Components (route-mapped, app-level)

| Component | Route | Purpose | Guide |
|-----------|-------|---------|-------|
| `Onboarding` | (first-run gate — no route) | First-run wizard (create vault, templates, sample) | `docs/getting-started/quickstart.md` |
| `PagesHome` | `/` | Page list with block counts | — |
| `JournalPanel` | `/journal` | Calendar + daily journal creation | `docs/guide/journal.md` |
| `PageView` | `/page/:pagePath` | Block editor + backlinks + connections | `docs/guide/block-editor.md` |
| `SearchPanel` | `/search` | Full-text + tag search | `docs/guide/search.md` |
| `QueryPanel` | `/query` | Datalog query panel: visual filter builder + raw Datalog editor (BuilderMode, ConditionRow) | `docs/guide/datalog-queries.md` |
| `GraphPanel` | `/graph` | 3D/2D force-directed graph (desktop/mobile) | `docs/guide/graph-view.md` |
| `TemplatesPanel` | `/templates` | Template list + apply with variables | `docs/guide/templates.md` |
| `FlashcardsPanel` | `/flashcards` | SRS card review (SM-2) | `docs/guide/flashcards.md` |
| `KanbanPanel` | `/kanban` | Drag-and-drop Kanban board | `docs/guide/kanban.md` |
| `WhiteboardPanel` | `/whiteboards` | Excalidraw spatial canvas | `docs/guide/whiteboards.md` |
| `PluginsPanel` | `/plugins` | Installed plugins + Plugin Directory / Theme Hub (desktop/mobile; shared hook `PluginsPanel.shared.tsx`) | `docs/guide/plugins.md` |
| `RemindersPanel` | `/reminders` | Overdue/today/upcoming reminder list + snooze/complete/open | `docs/guide/reminders.md` |
| `SettingsPage` | `/settings` | 8-tab app configuration | `docs/getting-started/configuration.md` |
| `BackupSettingsPanel` | `SyncTab` (SettingsPage) | Cloud/folder backup (WebDAV + folder) config, test-connection, run-now + status | `docs/guide/git-sync.md` |
| `AiConfigSection` | `AITab` (SettingsPage) | AI provider/API/model configuration (+ local endpoint discovery) | `docs/getting-started/configuration.md` |
| `SttConfigSection` | `AITab` (SettingsPage) | Voice dictation (STT) configuration | `docs/getting-started/configuration.md` |
| `TtsConfigSection` | `AITab` (SettingsPage) | Text-to-speech configuration | `docs/getting-started/configuration.md` |
| `MobileAiSection` | `SettingsPage.mobile` | AI provider + STT/TTS + voice-capture accordion (mobile; composes the shared desktop AI sections) | `docs/getting-started/configuration.md` |
| `MobileThemeSection` | `SettingsPage.mobile` | Theme settings accordion (mobile) | `docs/getting-started/configuration.md` |
| `MobileDeveloperSection` | `SettingsPage.mobile` | Developer tools accordion (mobile) | `docs/getting-started/configuration.md` |
| `MobileSyncSection` | `SettingsPage.mobile` | Sync configuration accordion (mobile) | `docs/getting-started/configuration.md` |
| `MobileTraySection` | `SettingsPage.mobile` | Tray settings accordion (mobile) | `docs/getting-started/configuration.md` |
| `MobileNotificationsSection` | `SettingsPage.mobile` | Notification settings accordion (mobile) | `docs/getting-started/configuration.md` |
| `AskNotesPanel` | `/ask` | AI Q&A over the knowledge base (chat with notes, citations, follow-up) | `docs/guide/ai-features.md` |

### Editor Sub-components

| Component | Parent | Purpose |
|-----------|--------|---------|
| `OutlinerEditor` | `PageView` | BlockNote-based outliner with auto-save, markers, wiki-links — decomposed into `index.tsx` + `dtoConverters.ts` + `markerDetection.ts` + shared sub-components (EditorStates, SaveStatusChip, MarkerBadgeRow, DeadLinkCreatePopup) and per-platform menus/hooks |
| `BacklinksPanel` | `PageView` | Linked references + unlinked mentions + hover preview; Ctrl+hover snippet popup + Ctrl+click navigation via `useBacklinkNavigation` |
| `ProseView` | `PageView` (desktop/mobile) | Read-mode prose renderer (files `prose.tsx` + `proseRenderer.ts`); renders blocks as clean read-only prose that never modifies the file |
| `BacklinkSnippetBody` | `BacklinksPanel` | Renders backlinked block snippet (title, context, anchor highlight, loading/error) |
| `BacklinkSnippetPopup` | `BacklinksPanel` (desktop) | Viewport-clamped popup for the backlink snippet (Ctrl+hover) |
| `SuggestedConnectionsPanel` | `PageView` | AI-suggested wiki-link connections |
| `AttachmentPanel` | `PageView` | Lists page attachments (`assets/…` refs), attach/upload with compression, delete; page metadata chip |
| `MermaidBlock` | `OutlinerEditor` | Custom BlockNote block for Mermaid diagrams |
| `AISlashMenu` | `OutlinerEditor` | Slash menu with AI actions (rewrite, summarize, etc.) |
| `AIFormattingToolbar` | `OutlinerEditor` | Formatting toolbar with AI buttons |
| `AutocompletePopup` | `OutlinerEditor` | Popover for wiki-link autocomplete (legacy, unused — superseded by `WikiLinkAutocomplete`; file kept pending cleanup) |
| `WikiLinkAutocomplete` | `OutlinerEditor` | BlockNote `SuggestionMenuController` wiki-link autocomplete (`[[` → search pages → insert slug/path link; path-qualified for duplicate slugs) |
| `LinkPreviewPopup` | `OutlinerEditor` | Hover preview for wiki-links |
| `MathEditorModal` | `OutlinerEditor` | LaTeX editor with live KaTeX preview |
| `MathSymbolPalette` | `MathEditorModal` | Tabbed symbol palette (Greek, Operators, etc.) |
| `MarkerBadge` | `OutlinerEditor` | Colored chip for task markers (TODO/DOING/DONE) |
| `MarkerSuggestMenu` | `OutlinerEditor` | Autocomplete popup for markers and priorities |
| `EditorLoadingState` / `EditorErrorState` | `OutlinerEditor` (desktop & mobile) | Shared loading / error placeholders (file `EditorStates.tsx`) |
| `SaveStatusChip` | `OutlinerEditor` (desktop & mobile) | Floating save-status chip (saving / saved-at label) |
| `MarkerBadgeRow` | `OutlinerEditor` (desktop & mobile) | Page marker badge strip (task markers on the page) |
| `DeadLinkCreatePopup` | `OutlinerEditor` (desktop & mobile) | "Create page" popover for unresolved [[wiki-links]] |
| `DesktopBlockContextMenu` | `OutlinerEditor` (desktop) | Right-click block menu (read aloud / set-change-clear reminder) |
| `MobileInsertMenu` | `OutlinerEditor` (mobile) | Floating insert menu (paragraph/headings/code/mermaid/math) |
| `MobileMarkerMenu` | `OutlinerEditor` (mobile) | Floating marker menu (TODO..CANCELLED) |
| `MobileBlockActionsMenu` | `OutlinerEditor` (mobile) | Long-press block menu (cut/copy/read-aloud/delete) |
| `KanbanEditDialog` | `KanbanPanel` | Edit card content, marker, priority |
| `ReminderDialog` | `OutlinerEditor` / `RemindersPanel` | Set/change/clear a block reminder (date-time picker + quick options) |

### Navigation & Utility

| Component | Purpose |
|-----------|---------|
| `Sidebar` (`index.tsx`) | Drawer wrapper with collapse state, vault info, header |
| `NavItemList` | Navigation items list (Journal, Graph, Kanban, etc.) |
| `PageTree` | Page list with create/delete, move/rename dialog, new-page form |
| `PageTreeGrouped` | Folder-tree variant of the page list when `pages_view = "tree"` |
| `MovePageDialog` | PageTree (Sidebar) / PageView desktop | Rename/move a page by path — calls move_page; inbound wiki-links rewritten by backend | docs/guide/vault-management.md |
| `SidebarFooter` | Refresh/export/version footer |
| `VaultPicker` | Landing page when no vault is configured |
| `StratumIcon` | App icon SVG renderer |
| `PagesHome` | Home route: page list with block counts |
| `ImportDialog` | PagesHome | One-click vault import (Obsidian/Logseq/generic) with preview + report | `docs/guide/importers.md` |
| `MobileLayout` | Root mobile wrapper with bottom navigation |
| `MobileNav` | Bottom navigation bar for mobile |
| `JournalCalendar` | Calendar popup for date navigation |
| `QuickCapture` | Quick capture dialog for tray/global-shortcut captures (desktop) |
| `VoiceCaptureButton` (`index`/`.desktop`/`.mobile`/`.shared`) | One-tap voice-capture into today's journal (mobile FAB / desktop header button) |

### UI Primitives (`src/components/ui/`)

| Component | Purpose | Origin |
|-----------|---------|--------|
| `LoadingOverlay` | Centered spinner with optional message | Shared pattern |
| `ErrorAlert` | Dismissable error Alert | Shared pattern |
| `EmptyState` | Centered empty state with icon + message + action | Shared pattern |
| `PageHeader` | Consistent header bar (title + actions + back) | Shared pattern |
| `ConfirmDialog` | Reusable confirmation dialog | Shared pattern |
| `SliderRow` | Label + slider + display value | Extracted from GraphPanel |
| `SettingsSection` | Shared settings section wrapper (title/description/divider, desktop+mobile variant) | New - settings primitives task |
| `SettingsTab` | Shared settings tab bar (desktop SettingsPage) | New - settings primitives task |
| `SettingsAccordion` | Shared collapsible settings accordion (mobile) | New - settings primitives task |
| `SettingsRow` | Shared labeled settings row (toggle or generic control) | New - settings primitives task |
| `ThemeColorEditor` | Shared theme color editor (preset swatches + custom picker) | New - settings primitives task |
| `ThemeFontSizeSlider` | Shared theme font-size slider | New - settings primitives task |
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
| `useOnboarding` | First-run wizard step/install orchestration (in `Onboarding.shared.tsx`) | Onboarding desktop/mobile |
| `useEditorReminders` | Reminder dialog state + applyDue + reminder-chip click delegation | OutlinerEditor (desktop & mobile) |
| `useDesktopBlockMenu` | Desktop right-click block menu + read-aloud + reminder actions | OutlinerEditor (desktop) |
| `useMobileBlockContextMenu` | Mobile long-press context menu (cut/copy/speak/delete) | OutlinerEditor (mobile) |
| `useEditorAutoSave` | Debounced auto-save with ED-07 no-op skip + flush-on-unmount | OutlinerEditor (via useEditorData) |
| `useWikiLinkInteractions` | Dead-link detection + Ctrl-hover preview + link click delegation | OutlinerEditor (via useEditorData) |
| `useBlockLoading` | Backend block load + DTO conversion + marker extraction | OutlinerEditor (via useEditorData) |

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

## Native GPUI Shell (E2)

`crates/stratum-ui` is the native desktop shell. It reproduces the shipped
React chrome 1:1 (sidebar, navigation, recents, footer, error bar and toaster)
on top of `stratum-app`, while `src/` and `src-tauri/` remain the
frozen reference application and are never modified by shell work.

- `stratum-i18n` embeds byte-identical copies of `src/i18n/locales/*.json`,
  provides i18next-compatible `{{name}}` interpolation and the parity helpers
  (`key_set`, `missing_keys`, `extra_keys`).
- `stratum-ui` owns the route/navigation/shortcut tables (`router`), the theme
  derivation (`theme`), the embedded asset source and Inter faces (`assets`),
  the `ShellData` provider seam (`provider`, with `provider/assets.rs`,
  `provider/journal.rs`, `provider/page.rs`, `provider/voice.rs`,
  `provider/panels.rs` for the panel DTOs/seam, `provider/app_panels.rs` for the
  service-backed panel data and the `provider/fixture/` deterministic provider),
  the chrome (`shell`, including the page lifecycle in `shell/page.rs`, the panel
  lifecycle in `shell/panels.rs` and the due-reminder toaster in `shell/mod.rs`)
  and the panel bodies (`panels`).
- The native journal surface lives in `crates/stratum-ui/src/panels/journal/`:
  `mod.rs` (`JournalView` entity and render), `state.rs` (accessors and
  read-mode switch), `navigation.rs` (day/month selection), `reload.rs`
  (ensure/repair, editor recreation, stack sync), `dates.rs` (localized
  day/month labels, calendar grid), `stack.rs` (lazy past-day stack plan and
  preload window), `header.rs`, `calendar.rs`, `rows.rs` and `error.rs`.
  `panels/voice/` owns the voice surface: `stage.rs` (pure stage machine) and
  `capture.rs` (`VoiceCapture` entity). Stacked day editors use `EditorView::set_auto_height` so each
  section contributes its content height, and journal editors omit the
  page-level attachment panel to match `JournalPanel`.
  `tests/journal_contract.rs` covers dates, calendar/week layout, the stack
  plan, the voice stages, the fixture journal/voice data and the
  `JOURNAL_I18N_KEYS` catalog gate without opening a window.
- The native page surface lives in `crates/stratum-ui/src/panels/page/`:
  `mod.rs` (`PageView` entity, render and start state), `state.rs` (accessors,
  read-mode forwarding, editor reload, navigation), `header.rs` (title, path
  chip, voice/attach/read controls and Note menu trigger), `menu.rs` (Note menu
  model and popup), `move_dialog.rs` (move/rename validation and dialog),
  `delete_dialog.rs` (delete confirmation), `backlinks.rs` (linked/unlinked
  references with bounded snippets), `suggestions.rs` (related-note suggestions
  with add-link) and `actions.rs` (reindex/normalize/delete flows). The page
  header reuses `panels/voice/` for the mic control. `tests/page_contract.rs`
  covers the Note menu order, move validation, backlink grouping, relevance
  clamping, the suggestion state machine, the fixture page seam and the
  `PAGE_I18N_KEYS` catalog gate without opening a window.
  `tests/interaction_contract.rs` adds headless-window coverage for nested
  click propagation and journal start scroll.
- The native search, query, Ask Notes, Templates, Flashcards, Reminders and
  Plugins surfaces live under `crates/stratum-ui/src/panels/{search,query,ask,
  templates,flashcards,reminders,plugins}/`. Every panel renders only catalog
  keys declared in its `*_I18N_KEYS` gate list (verified in all eight catalogs)
  and routes provider calls through `PanelData`. The Reminders panel
  (`reminders/model.rs` grouping/humanize/dialog, `reminders/view.rs`,
  `reminders/dialog.rs`) reuses `stratum_app::reminders::due`; the Plugins panel
  (`plugins/actions.rs`, `plugins/{installed,directory,themes}.rs`) covers the
  installed/directory/themes tabs and applies themes through
  `stratum_app::themes` + `crate::theme::apply_to_kit`. The shell toaster
  announces due reminders once per due window with a localized open action
  (`Shell::poll_reminders`, `activate_toast_action`).
- The native graph surface lives in `crates/stratum-ui/src/panels/graph/`:
  `mod.rs` (`GraphView` entity, render and start state), `model.rs` (canonical
  `pkm_core::GraphConfig` force defaults plus the `GraphSettingsRow` mapping,
  banner and stats text), `canvas.rs` (letterbox
  geometry, orbit/pan/dolly, WASD/arrow camera steps, click-to-open and
  right-click focus), `scene.rs` (renderer selection, deterministic layout job,
  scene build and `RenderImage` conversion), `layout.rs` (worker polling and
  hover-highlight rebuilds), `settings.rs` (visibility/3D toggles, force sliders,
  node cap), `setters.rs` (settings mutators that persist through
  `PanelData::save_graph_settings`; the node cap stays in memory) and
  `toolbar.rs` (view modes, component selector, title/tag search
  and stats). `stratum-graph` supplies the physics, camera parity and the
  wgpu/software renderer; `tests/graph_contract.rs` covers the force defaults,
  the `GRAPH_I18N_KEYS` gate, the filter/cap policy, the camera pan/zoom
  formulas, the frame-time histogram and the deterministic worker layout.

  | Panel | Module | Provider methods | Capture |
  |-------|--------|------------------|---------|
  | Search | `panels/search` | `search_blocks`, `search_by_tag`, `rebuild_search_index` | `dist/gpui-search.png`, `dist/gpui-search-tag.png` |
  | Query | `panels/query` | `run_query`, `saved_queries`, `save_query`, `rename_query`, `delete_query` | `dist/gpui-query.png`, `dist/gpui-query-results.png`, `dist/gpui-query-datalog.png` |
  | Ask Notes | `panels/ask` | `ask_notes` | `dist/gpui-ask.png` |
  | Templates | `panels/templates` | `templates`, `apply_template`, `install_starter_templates` | `dist/gpui-templates.png` |
  | Flashcards | `panels/flashcards` | `flashcards`, `review_card` | `dist/gpui-flashcards.png`, `dist/gpui-flashcards-answer.png` |
  | Reminders | `panels/reminders` | `reminders`, `due_reminders`, `snooze_reminder`, `clear_reminder`, `set_reminder` | `dist/gpui-reminders.png`, `dist/gpui-reminders-dialog.png`, `dist/gpui-reminder-toast.png` |
  | Plugins | `panels/plugins` | `plugins`, `set_plugin_enabled`, `reload_plugin`, `uninstall_plugin`, `install_plugin_path`, `install_plugin_url`, `plugin_note_read`, `plugin_http_request`, `plugin_catalog`, `theme_catalog`, `installed_themes`, `install_theme`, `apply_theme`, `uninstall_theme` | `dist/gpui-plugins.png`, `dist/gpui-plugins-directory.png`, `dist/gpui-plugins-themes.png` |
  | Whiteboards | `panels/canvas` | `whiteboards`, `load_whiteboard`, `save_whiteboard`, `create_whiteboard`, `rename_whiteboard`, `delete_whiteboards`, `whiteboard_library`, `save_whiteboard_library`, `write_canvas_export` | `dist/gpui-whiteboards.png`, `dist/gpui-whiteboard-editor.png`, `dist/gpui-whiteboard-library.png`, `dist/gpui-whiteboard-style.png`, `dist/gpui-whiteboard-text.png` |
  | Graph | `panels/graph` | `graph_panel_data` | `dist/gpui-graph.png`, `dist/gpui-graph-component.png`, `dist/gpui-graph-settings.png` |

- The native whiteboards surface lives in `crates/stratum-ui/src/panels/canvas/`:
  `mod.rs` (`CanvasView` entity, render and start state), `gallery.rs` (board
  cards, previews, multi-select and create/rename/delete), `editor.rs` (header,
  save chip and status rows), `canvas.rs` (rasterized board surface, pan/zoom and
  pointer input), `toolbar.rs` (tools, style panel, align/distribute), `library.rs`
  (drawer), `actions.rs` (create/rename/delete/export/copy-link flows), `edit.rs`
  (tool and style mutators) and `model/{mod,session,scene}.rs` (gallery/session
  models, the 800 ms autosave and the visibility query). `stratum-canvas` supplies
  the element model, `.excalidraw`/`.excalidrawlib` round-trip, tools, snapping,
  undo/redo, `Board#Element` links, the `roughr` scene and the SVG/PNG/WebP export;
  `panels/canvas/tests.rs` covers the gallery/session models, byte-identical
  untouched saves, autosave, exports, library round-trip, clipboard, style/align,
  deep-link focus, the 5k-element visibility query and the `WHITEBOARD_I18N_KEYS`
  catalog gate without opening a window.
- `stratum-editor` is the headless block-editor engine the phase-B editor view
  renders (`DecoratedRow`s from `render`). It owns document canonicalization,
  inline spans, editing/undo/IME, slash and marker menus, wiki-link helpers,
  marker/reminder models, math/Mermaid/attachment-thumbnail caches,
  attachment classification/row labels (`assets`), reserved asset row heights
  and virtualization, and has no UI dependencies. Attachment rows only promote
  for whole-content `![[…]]` embeds (surrounding whitespace allowed); mixed
  content keeps the inline asset chip and the normal text height.
- The native editor view lives in `crates/stratum-ui/src/panels/editor/`:
  `mod.rs` (EditorView state, save scheduling, asset budget), `metrics.rs`
  (per-frame measurements), `view.rs` (virtual list, canvas input, overlays,
  external file drop), `rows.rs` (row/span/caret geometry), `asset_rows.rs`
  (thumbnail and file-attachment rows), `asset_state.rs` (thumbnail budget,
  attachment list, drop target), `asset_actions.rs` (import/delete/picker flows),
  `attachments.rs` (attachment panel with inline delete confirm), `input.rs`
  (EntityInputHandler, key/mouse, clipboard image paste, IME), `overlay.rs`
  (slash/wiki/marker/link/dead-link popups), `context_menu.rs`
  (right-click block menu: read aloud, copy/cut, marker, reminder) and `save.rs`
  (save-status transitions). Asset logic lives in `pkm-block::assets` and
  `stratum-app::asset` (8 MiB limit, image compression parity with
  `src/lib/assetUpload.ts`, thumbnail decode). Read-aloud goes through `ShellData::speak` →
  `StratumService::tts_speak` → base64 decode → `rodio` playback. `tests/editor_contract.rs` covers the 10k layout,
  focused-line delimiter visibility, catalogues, save transitions and UTF-16
  helpers without opening a window.
- The shell never calls Tauri commands directly; data flows through
  `ShellData`/`AppProvider`, whose real implementation drives
  `stratum_app::StratumService`.
- `STRATUM_FIXTURE=1` selects the deterministic 87-page fixture used by tests
  and the capture harness (`STRATUM_FIXTURE_BLOCKS` controls the Welcome page
  size). `STRATUM_VAULT` selects a real vault and `STRATUM_LOCALE` selects one of
  the eight catalogs. Start-state flags for the capture harness are applied by
  `apply_start_state` in `main.rs`: `STRATUM_ROUTE`, `STRATUM_PAGE`,
  `STRATUM_JOURNAL_DATE`, `STRATUM_CALENDAR`, `STRATUM_READ_MODE`,
  `STRATUM_REMINDER_DIALOG`, `STRATUM_PLUGINS_TAB`, `STRATUM_GRAPH_MODE`,
  `STRATUM_GRAPH_COMPONENT`, `STRATUM_GRAPH_SETTINGS`, `STRATUM_GRAPH_SEARCH`,
  `STRATUM_GRAPH_TICKS`, `STRATUM_WHITEBOARD`, `STRATUM_WHITEBOARD_ELEMENT`,
  `STRATUM_WHITEBOARD_TOOL`, `STRATUM_WHITEBOARD_LIBRARY`,
  `STRATUM_WHITEBOARD_STYLE`, `STRATUM_WHITEBOARD_TEXT` and
  `STRATUM_WHITEBOARD_EXPORT`; `STRATUM_REMINDER_TOAST=1` makes the fixture report
  due reminders to the shell toaster.
- `scripts/capture-gpui-shell.sh` renders the fixture shell on a private Xvfb
  screen and writes `dist/gpui-shell.png` (224 px sidebar) plus
  `dist/gpui-shell-collapsed.png` (52 px rail, `STRATUM_COLLAPSED=1`).
- `scripts/capture-gpui-panels.sh` performs thirty-two captures: the journal
  surface in edit mode, with the calendar expanded and on a past day in read mode
  (`dist/gpui-journal.png`, `dist/gpui-journal-calendar.png`,
  `dist/gpui-journal-read.png`), the fixture page in edit/read mode with the
  Note menu, expanded cards, a revealed block and a preloaded backlink snippet
  (`STRATUM_SNIPPET=1` → `dist/gpui-page-snippet.png`), the search, query,
  Ask Notes, Templates and Flashcards surfaces, the Reminders surface with the
  due dialog (`dist/gpui-reminders.png`, `dist/gpui-reminders-dialog.png`) and
  the due-reminder toast (`dist/gpui-reminder-toast.png`), and the Plugins
  installed/directory/themes tabs (`dist/gpui-plugins.png`,
  `dist/gpui-plugins-directory.png`, `dist/gpui-plugins-themes.png`), and the
  Graph surface with the full fixture graph, the component view and the
  settings panel (`dist/gpui-graph.png`, `dist/gpui-graph-component.png`,
  `dist/gpui-graph-settings.png`), and the Whiteboards surface with the
  fixture gallery, the framed board in the editor, the library drawer, the
  style panel and the inline text-edit overlay (`dist/gpui-whiteboards.png`,
  `dist/gpui-whiteboard-editor.png`, `dist/gpui-whiteboard-library.png`,
  `dist/gpui-whiteboard-style.png`, `dist/gpui-whiteboard-text.png`).
- `crates/stratum-ui/tests/interaction_contract.rs` opens headless gpui windows
  to cover click propagation (snippet/voice chips), journal start scroll, the
  reminder block-open request, the toast open action and the plugins
  tab/smoke-test/theme flows.
- `scripts/capture-gpui-editor-perf.sh` runs `stratum-editor-perf` on the 10k
  fixture, prints frame/render timings and gates only on `render_p95_ms >
  18.18` (55 fps work budget); the JSON report lands in
  `.hermes/perf/editor-10k-frames.json`.
- `.github/workflows/gpui-ci.yml` runs the GPUI format/build/test/clippy gates
  on Linux, Windows and macOS, publishes the screenshot and the editor perf
  report. The main `ci.yml` also has a `gpui-capture` job.

See `docs/development/gpui-ui.md` for the module/API map, build commands,
test coverage and current limitations, and `docs/development/gpui-editor.md`
for the headless editor engine, its byte-stability contract and the phase-B
boundary.

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

Command handlers (`src-tauri/src/commands/`) must be **thin glue layers**. Business logic belongs in `stratum-app` (the headless service layer) and the domain crate it wraps:

| Command Handler | Service Method | Domain Logic |
|----------------|----------------|--------------|
| `graph.rs` | `stratum-app::graph` | `pkm-block` (graph building) |
| `settings.rs` | `stratum-app::settings` | `pkm-core::Config` (DTO mapping) |
| `search.rs` | `stratum-app::search` | `pkm-index` (search/backlinks) |
| `sync.rs` | `stratum-app::sync` | `pkm-sync` (git operations) |

See `docs/development/service-layer.md` for the full command → service method map and the adapter rules.

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

# Native GPUI shell (fixture, no vault required)
cargo build -p stratum-ui
STRATUM_FIXTURE=1 cargo run -p stratum-ui

# GPUI crate tests + capture
cargo test -p stratum-i18n -p stratum-ui -p stratum-editor
cargo clippy -p stratum-i18n -p stratum-ui -p stratum-editor --all-targets -- -D warnings
./scripts/capture-gpui-shell.sh  # writes dist/gpui-shell.png

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

## Repo Hygiene — what gets committed

**Every agent/contributor MUST follow this. The repo contains only shipped, necessary code and user-facing docs — never implementation aids.**

| Path | Purpose | Committed? |
|------|---------|------------|
| `crates/`, `src/`, `src-tauri/`, `e2e/`, `scripts/`, `nix/`, `deploy/`, `*` config | Real code, tests, tooling | ✅ yes |
| `docs/` only `docs/guide/`, `docs/development/`, `docs/getting-started/`, `docs/advanced/`, `docs/cli/`, `docs/development/adr/`, `docs/index.md`, `docs/contributing.md`, `docs/announcement.md`, `docs/assets/`, `docs/landing/` | Shipped user/dev reference docs (each must be in a mkdocs nav entry or referenced by another shipped doc) | ✅ yes |
| `verification/cdp_probe.mjs` | Reusable live-probe reference (cited by `docs/development/mobile-guide.md`) | ✅ yes (only file under `verification/`) |
| `.hermes/` (plans, reports), `.omo/`, `.opencode/`, `.sisyphus/`, `.worktrees/`, `.screenshots/`, `e2e/**/.evidence/`, `verification/` (except cdp_probe), `docs/qa/`, `test-results/`, `dist/`, `target/`, `node_modules/` | Plans, QA reports, findings, audits, repros, evidence, screenshots, build output | ❌ never |

**Rules:**
1. **Plans, QA reports, findings, audits, repros, and evidence go to `.hermes/`** (already gitignored). Never to repo root, `docs/`, or `verification/`.
2. **Never commit an implementation aid** — no `*-plan.md`, `*-REPORT-*.md`, `*FINDINGS.md` at repo root; no `docs/fc-*.md`, `docs/audit-*.md`, `docs/qa/`; no `docs/acceptance-*.md`.
3. **`docs/` = shipped docs only.** If a doc is not reachable in the mkdocs nav and not referenced by another shipped doc, it does not belong in `docs/` — move it to `.hermes/`.
4. If a QA/report artifact is genuinely worth keeping as a shipped reference, it must be linked from the mkdocs nav (or another shipped doc) and live under `docs/development/` or `development/adr/` — otherwise it stays transient.
5. CI (`repo-hygiene` job, `scripts/check-transient-artifacts.sh`) fails if any transient artifact is tracked. If you need one tracked, add an explicit exception in that script and justify it with a comment.

**Canonical location for agent working files:** `.hermes/` — plans, task reports, evidence, scratch. `.hermes/` is gitignored so anything there is safe to write freely.

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
| New Tauri command | Implement it as a `StratumService` method in `crates/stratum-app`, add a thin adapter + `invoke.rs` entry, add a headless test, and update `docs/development/service-layer.md` |
| Command logic changed | Keep the service method (not the adapter) authoritative; update `docs/development/service-layer.md` if the mapping moves |
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

## Screenshot PII Gate (mandatory before commit)
Any screenshot/render/evidence image committed to this repo must have NO personal information:
names, emails, phones, addresses, API tokens, real usernames, IPs, `/home/<user>` paths,
`user@host` strings, hostnames. Redact (solid matching-color box) first; verify before commit.
