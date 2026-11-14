# Stratum — PKM System

A privacy-first, offline-capable personal knowledge management system with native Git sync,
bi-directional linking, graph visualization, and AI-augmented search/chat.
Notes are stored as plain Markdown files on disk — zero vendor lock-in.

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                Native Stratum shell (gpui)                    │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  stratum-ui: sidebar, router, shell chrome, panels      │  │
│  │  ┌──────────┐ ┌──────────┐ ┌────────────────────────┐  │  │
│  │  │ Editor   │ │  Graph   │ │  Search / Ask / AI     │  │  │
│  │  │ (native) │ │ (wgpu 3D)│ │                        │  │  │
│  │  └────┬─────┘ └────┬─────┘ └───────────┬────────────┘  │  │
│  │  ┌────┴────────────┴───────────────────┴───────────┐   │  │
│  │  │  ShellData / AppProvider (in-process service)    │   │  │
│  │  └─────────────────────┬───────────────────────────┘   │  │
│  └────────────────────────┼───────────────────────────────┘  │
│                            │ direct calls (no IPC)             │
│  ┌────────────────────────┼───────────────────────────────┐  │
│  │            Rust service layer (same process)            │  │
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
│   ├── pkm-ai/                 # Embeddings, RAG, LLM provider, local-endpoint discovery
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
│   ├── stratum-app/            # Headless service layer behind every shell surface (no UI deps)
│   ├── stratum-canvas/         # Excalidraw-compatible board model: elements, tools, roughr scene, SVG/PNG/WebP export
│   ├── stratum-editor/         # Headless block-editor engine: document, spans, state, undo, menus (no UI deps)
│   ├── stratum-graph/          # Deterministic d3-force-3d physics, camera parity, headless wgpu 3D renderer + CPU fallback
│   ├── stratum-i18n/           # Embedded locale catalogs, interpolation and parity gates
│   ├── stratum-mobile/         # Android host: NativeActivity entry, IME/insets/lifecycle glue, Gradle project
│   ├── stratum-platform/       # OS glue: tray, global shortcut, notifications, single instance, deep links, opener, drop classification
│   └── stratum-ui/             # Native GPUI application: shell, router, panels, mobile layout (the shipped desktop app)
├── vendor/
│   └── gpui-mobile/            # Vendored gpui-pre Android/iOS platform layer (excluded from the workspace)
├── goldens/
│   └── gpui/                   # Committed golden screenshots for the parity gate
├── docs/
│   ├── index.md                # MkDocs home page
│   ├── mkdocs.yml              # MkDocs configuration
│   ├── getting-started/        # installation, quickstart, configuration
│   ├── guide/                  # one user guide per shipped feature
│   ├── cli/command-reference.md
│   ├── advanced/               # file format, plugins, MCP
│   └── development/            # rust-guide, service-layer, gpui-ui, gpui-editor,
│                               # mobile-guide, packaging, parity, branch-policy, ADRs
├── packaging/                  # Desktop installer assets (shared by deb/AppImage/rpm)
├── scripts/                    # capture/verify/package/build tooling
├── .github/
│   ├── actions/install-linux-deps/
│   ├── scripts/smoke-android.sh
│   └── workflows/{ci.yml,gpui-release.yml}
├── .sisyphus/refactoring-plan.md
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
pkm-tests      →  pkm-core, pkm-block, pkm-markdown, pkm-index, pkm-sync, pkm-query, pkm-ai, pkm-plugin, stratum-app
pkm-cli        →  pkm-core, pkm-markdown, pkm-index, pkm-sync, pkm-watcher, pkm-ai, pkm-plugin
stratum-editor →  pkm-core, pkm-block, pkm-markdown
stratum-canvas →  pkm-core
stratum-graph  →  pkm-core
stratum-i18n   →  pkm-core
stratum-platform →  pkm-core (desktop-only OS crates behind target cfgs: tray-icon, global-hotkey, notify-rust, interprocess)
stratum-ui     →  gpui-kit (external), rodio (external), stratum-i18n, stratum-platform, stratum-app, stratum-editor, stratum-graph, stratum-canvas, pkm-core
stratum-mobile →  stratum-ui, stratum-i18n, gpui-kit (external), gpui-mobile (vendored, Android-only), android-activity, android_logger, libc
```

**NOTE**: `stratum-app` has no dependency on `tauri` or any `tauri-plugin-*` crate — that isolation is the point of the extraction. The CLI does NOT depend on `pkm-block` or `pkm-query`.

**NOTE**: `stratum-platform` is the OS glue behind `cfg(desktop)`: `tray-icon`, `global-hotkey`, `notify-rust` and `interprocess` (all MIT OR Apache-2.0, AGPL-3.0 compatible) for the tray menu, the Quick Capture shortcut, reminder notifications and single-instance argv forwarding.

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

## Application State

The shell owns its state in-process; there is no IPC layer and no client-side
store library.

- `Shell` (`crates/stratum-ui/src/shell/`) owns navigation, the active route,
  panel lifecycle, the toaster and the platform bridge watch.
- Per-surface entities (`JournalView`, `PageView`, `SearchView`, `QueryView`,
  `AskView`, `TemplatesView`, `FlashcardsView`, `KanbanView`, `RemindersView`,
  `PluginsView`, `CanvasView`, `GraphView`, `SettingsView`) hold their own view
  state and load data through `PanelData`.
- Persistent state (settings, graph config, theme) is read and written through
  `stratum_app::StratumService`; nothing is cached outside the vault.

All data operations flow: `panel entity` → `ShellData`/`PanelData` →
`stratum_app::StratumService` → domain crate.

## Native shell surfaces

Every shipped surface is a panel under `crates/stratum-ui/src/panels/<name>/`
(desktop) with a mobile wrapper in `crates/stratum-ui/src/mobile/variants/`.
The module/API map, provider methods and capture flags live in
`docs/development/gpui-ui.md`; each row below also has a golden screenshot under
`goldens/gpui/` (checked by `scripts/verify-gpui-goldens.sh`) and a guide.

| Panel | Route | Module | Guide | Golden |
|---|---|---|---|---|
| Onboarding | first-run gate | `shell/onboarding.rs` | `docs/getting-started/quickstart.md` | `gpui-onboarding.png` |
| Journal | `/journal` | `panels/journal` | `docs/guide/journal.md` | `gpui-journal.png` |
| Page | `page/:path` | `panels/page` | `docs/guide/block-editor.md` | `gpui-page.png` |
| Editor | inside Page/Journal | `panels/editor` | `docs/guide/block-editor.md` | `gpui-page.png` |
| Search | `/search` | `panels/search` | `docs/guide/search.md` | `gpui-search.png` |
| Query | `/query` | `panels/query` | `docs/guide/datalog-queries.md` | `gpui-query.png` |
| Ask Notes | `/ask` | `panels/ask` | `docs/guide/ai-features.md` | `gpui-ask.png` |
| Templates | `/templates` | `panels/templates` | `docs/guide/templates.md` | `gpui-templates.png` |
| Flashcards | `/flashcards` | `panels/flashcards` | `docs/guide/flashcards.md` | `gpui-flashcards.png` |
| Kanban | `/kanban` | `panels/kanban` | `docs/guide/kanban.md` | `gpui-kanban.png` |
| Reminders | `/reminders` | `panels/reminders` | `docs/guide/reminders.md` | `gpui-reminders.png` |
| Whiteboards | `/whiteboards` | `panels/canvas` | `docs/guide/whiteboards.md` | `gpui-whiteboards.png` |
| Plugins | `/plugins` | `panels/plugins` | `docs/guide/plugins.md` | `gpui-plugins.png` |
| Graph | `/graph` | `panels/graph` | `docs/guide/graph-view.md` | `gpui-graph.png` |
| Settings | `/settings` | `panels/settings` | `docs/getting-started/configuration.md` | `gpui-settings-vault.png` |
| Quick Capture | dialog | `shell/quick_capture.rs` | `docs/guide/quick-capture.md` | `gpui-quick-capture.png` |
| Mobile shell | — | `mobile/` | `docs/development/mobile-guide.md` | `gpui-mobile-journal.png` |

### Module sizing rules

| Metric | Limit | Action |
|--------|-------|--------|
| File lines | < 500 | Split into sub-modules |
| Render body | < 400 lines | Extract rows/widgets into sibling modules |
| Inline function | < 20 lines | Extract to module-level |
| Same logic in 2+ files | 0 duplicates | Extract to `stratum-editor`/`stratum-app` or a shared module |
| Public item | Documented | Every public fn/struct carries a doc comment |

### Desktop + mobile pattern

Mobile reuses the live desktop entity and adds only chrome:

```
crates/stratum-ui/src/mobile/variants/
├── mod.rs                 # router::Panel → wrapper mapping
├── journal.rs             # mobile chrome around JournalView
└── ...
```

`Shell::new_mobile` selects the layout and `Shell::set_mobile_services` installs
the `MobileServices` host implementation the Android crate provides.

## Graph Engine

The graph surface is split across three crates: `stratum-graph` owns the
deterministic d3-force-3d physics, camera parity and the wgpu/software renderer;
`stratum-app::graph` builds the node/edge graph from the SQLite BlockStore (no
file I/O or Tantivy index rebuild); `crates/stratum-ui/src/panels/graph` renders
and drives it.

- **Node/Edge graph** built from `[[wiki-links]]` stored in SQLite blocks
- **Connected components** via BFS on an adjacency list derived from block links
- **Orphaned notes** detection (notes with zero incoming/outgoing connections)
- **Slug resolution**: resolves `[[Title]]` links to note slugs via title lookup
- **Provider methods**: `graph_panel_data`, `save_graph_settings`
- **Frontend**: `GraphView` renders the 3D scene on desktop and the 2D layout on
  mobile, with visibility toggles (connected/orphaned/tags), force sliders,
  component/orphan view modes, node search and click-to-navigate.

## Native shell (shipped application)

`crates/stratum-ui` is the shipped desktop application. The React/Tauri
frontend was deleted at the E12 cutover (see `docs/development/parity.md`), so
there is no IPC layer, no web view and no npm toolchain: the shell renders the
sidebar, navigation, recents, footer, error bar, toaster and every panel on top
of `stratum-app` in the same process. `crates/stratum-mobile` is the Android
host of the same shell.

- `stratum-i18n` owns the eight locale catalogs (en/de/it/fr/es complete,
  zh/ja/el placeholder), provides `{{name}}` interpolation and the parity
  helpers (`key_set`, `missing_keys`, `extra_keys`). `crates/stratum-i18n/tests/raw_keys.rs`
  fails when a shell string uses a key that is not in the catalog.
- `stratum-ui` owns the route/navigation/shortcut tables (`router`), the theme
  derivation (`theme`), the embedded asset source and Inter faces (`assets`),
  the `ShellData` provider seam (`provider`, with `provider/assets.rs`,
  `provider/journal.rs`, `provider/page.rs`, `provider/voice.rs`,
  `provider/settings.rs`/`provider/app_settings.rs` for the settings contract,
  `provider/panels.rs` for the panel DTOs/seam, `provider/app_panels.rs` for the
  service-backed panel data and the `provider/fixture/` deterministic provider),
  the chrome (`shell`, including the page lifecycle in `shell/page.rs`, the panel
  lifecycle in `shell/panels.rs`, the due-reminder toaster in `shell/mod.rs`, the
  platform bridge watch in `shell/platform.rs` and the Quick Capture dialog in
  `shell/quick_capture.rs`) and the panel bodies (`panels`).
- `stratum-platform` is the OS glue crate the shell drives through
  `stratum_ui::platform::PlatformBridge`: tray menu + close-to-tray (`tray.rs`,
  KSNI on Linux), global Quick Capture shortcut (`hotkey.rs`, `global-hotkey`),
  reminder notifications (`notify.rs`, `notify-rust`), single instance and argv
  forwarding (`instance.rs`), `stratum://` deep links (`deep_link.rs`, scheme
  registration), reveal/open (`opener.rs`) and drop classification
  (`dragdrop.rs`), with `runtime.rs` owning the event channel. It replaces the
  seven Tauri plugins; `docs/guide/platform-integration.md` is the capability
  matrix. The pure models build on every target (Android reuses them) and only
  the OS halves sit behind `cfg(desktop)`.
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
  | Kanban | `panels/kanban` | `kanban_cards`, `create_kanban_card`, `update_kanban_card`, `delete_kanban_card` | `dist/gpui-kanban.png` |
  | Whiteboards | `panels/canvas` | `whiteboards`, `load_whiteboard`, `save_whiteboard`, `create_whiteboard`, `rename_whiteboard`, `delete_whiteboards`, `whiteboard_library`, `save_whiteboard_library`, `write_canvas_export` | `dist/gpui-whiteboards.png`, `dist/gpui-whiteboard-editor.png`, `dist/gpui-whiteboard-library.png`, `dist/gpui-whiteboard-style.png`, `dist/gpui-whiteboard-text.png` |
  | Graph | `panels/graph` | `graph_panel_data` | `dist/gpui-graph.png`, `dist/gpui-graph-component.png`, `dist/gpui-graph-settings.png` |
  | Settings | `panels/settings` | `settings`, `save_settings`, `set_vault_path`, `fetch_models`, `discover_local_ai`, `reindex_vault`, `repair_db_from_disk`, `normalize_all_files`, `sync_status`, `sync_now`, `sync_with_passphrase`, `resolve_conflict`, `abort_merge`, `commit_log`, `backup_status`, `run_backup`, `test_webdav`, `stt_test`, `tts_test`, `ping_search_instance`, `set_onboarding_pending`, `preview_import`, `run_import`, `export_html_to`, `export_json_to`, `export_pdf_to`, `export_zip_to` | `dist/gpui-settings-vault.png`, `dist/gpui-settings-theme.png`, `dist/gpui-settings-ai.png`, `dist/gpui-settings-research.png`, `dist/gpui-settings-developer.png`, `dist/gpui-settings-sync.png`, `dist/gpui-settings-sync-conflicts.png`, `dist/gpui-settings-tray.png`, `dist/gpui-settings-notifications.png` |

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
- The native settings surface lives in `crates/stratum-ui/src/panels/settings/`:
  `mod.rs` (`SettingsView` entity, eight-tab dispatch, start state and render),
  `state.rs` (accessors/status mutators), `keys.rs` (`SETTINGS_I18N_KEYS`),
  `form.rs`/`raw.rs` (editable `SettingsDto` copy, raw numeric text and export
  paths), `validate.rs` (the validation matrix), `inputs.rs` (input entities and
  subscriptions), `actions.rs`/`actions_ai.rs`/`actions_ops.rs` (provider
  mutations, probes, exports and navigation requests), `controls.rs` (palette
  snapshot and styled widgets) and `tabs/{vault,theme,ai,ai_discovery,ai_voice,
  research,developer,sync,backup,tray,notifications}.rs` plus the shared
  `tabs/commit_log.rs` body rendered in the developer tab. Saving validates the
  whole form, blocks on invalid fields with `settings.validation.save_blocked`,
  and on success persists through `SettingsData::save_settings`, reloads and
  re-applies the theme; theme edits apply live and startup loads the persisted
  theme through `ThemeConfig::from_settings`. The vault picker switches the
  active vault through `SettingsData::set_vault_path`, and the sync tab resolves
  merge conflicts inline (Accept/Resolve All/Abort Merge).
  `panels/settings/tests.rs` and `panels/settings/tests_acceptance.rs` cover the
  field round-trip, validation matrix, theme fallback, fixture persistence,
  conflict resolution, tab env parsing and the `SETTINGS_I18N_KEYS` catalog gate
  without opening a window.
- The native mobile shell lives in `crates/stratum-ui/src/mobile/`:
  `insets.rs` (`MobileFrame` with top/right/bottom/left/ime, `frame_from_insets`,
  `frame_for_window` and the `STRATUM_INSETS` capture override; safe-area edges
  stay separate from the IME height), `nav.rs` (the four bottom tabs Journal /
  Search / Graph / More, the ten-row More sheet, the ≥44 px touch-target contract
  and the back stack), `services.rs` (the `MobileServices` host seam with
  `NoopServices`) and `variants/` (one wrapper per `router::Panel` that reuses
  the live desktop entities and adds only mobile chrome). `Shell::new_mobile` and
  `Shell::with_platform_mobile` select the layout, `Shell::set_mobile_services`
  installs the Android host implementation and `Shell::mobile_back` handles the
  back gesture (`MobileBack` action). `MOBILE_I18N_KEYS` gates the mobile chrome
  in every catalog.
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
  `stratum-app::asset` (8 MiB limit, image compression, thumbnail decode). Read-aloud goes through `ShellData::speak` →
  `StratumService::tts_speak` → base64 decode → `rodio` playback. `tests/editor_contract.rs` covers the 10k layout,
  focused-line delimiter visibility, catalogues, save transitions and UTF-16
  helpers without opening a window.
- The shell never crosses an IPC boundary; data flows through
  `ShellData`/`AppProvider`, whose real implementation drives
  `stratum_app::StratumService` in-process.
- `STRATUM_FIXTURE=1` selects the deterministic 87-page fixture used by tests
  and the capture harness (`STRATUM_FIXTURE_BLOCKS` controls the Welcome page
  size). `STRATUM_VAULT` selects a real vault and `STRATUM_LOCALE` selects one of
  the eight catalogs. Start-state flags for the capture harness are applied by
  `apply_start_state` in `main.rs`: `STRATUM_ROUTE`, `STRATUM_PAGE`,
  `STRATUM_HIGHLIGHT`, `STRATUM_MENU`, `STRATUM_CARDS`, `STRATUM_SNIPPET`,
  `STRATUM_JOURNAL_DATE`, `STRATUM_CALENDAR`, `STRATUM_READ_MODE`,
  `STRATUM_REMINDER_DIALOG`, `STRATUM_SEARCH`, `STRATUM_QUERY_RUN`,
  `STRATUM_QUERY_TAB`, `STRATUM_ASK`, `STRATUM_FLASHCARD_REVEAL`,
  `STRATUM_KANBAN_COLLAPSED`, `STRATUM_KANBAN_NEW`, `STRATUM_KANBAN_EDIT`,
  `STRATUM_KANBAN_MENU`, `STRATUM_ONBOARDING`, `STRATUM_ONBOARDING_STEP`,
  `STRATUM_PLUGINS_TAB`, `STRATUM_GRAPH_MODE`, `STRATUM_GRAPH_COMPONENT`,
  `STRATUM_GRAPH_SETTINGS`, `STRATUM_GRAPH_SEARCH`, `STRATUM_GRAPH_TICKS`,
  `STRATUM_WHITEBOARD`, `STRATUM_WHITEBOARD_ELEMENT`, `STRATUM_WHITEBOARD_TOOL`,
  `STRATUM_WHITEBOARD_LIBRARY`, `STRATUM_WHITEBOARD_STYLE`,
  `STRATUM_WHITEBOARD_TEXT`, `STRATUM_WHITEBOARD_EXPORT`, `STRATUM_SETTINGS_TAB`,
  `STRATUM_SETTINGS_COMMITS`, `STRATUM_SETTINGS_CONFLICTS`,
  `STRATUM_QUICK_CAPTURE`, `STRATUM_QUICK_CAPTURE_VOICE`, `STRATUM_DEEP_LINK`,
  `STRATUM_MOBILE`, `STRATUM_MOBILE_MORE` and `STRATUM_INSETS`;
  `STRATUM_REMINDER_TOAST=1` makes the fixture report due reminders to the shell
  toaster. `STRATUM_MOBILE=1`
  opens the 1080×2400 mobile shell instead of the desktop shell.
- `scripts/capture-gpui-shell.sh` renders the fixture shell on a private Xvfb
  screen and writes `dist/gpui-shell.png` (224 px sidebar) plus
  `dist/gpui-shell-collapsed.png` (52 px rail, `STRATUM_COLLAPSED=1`).
- `scripts/capture-gpui-panels.sh` performs forty-seven captures: the journal
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
  `dist/gpui-whiteboard-style.png`, `dist/gpui-whiteboard-text.png`), and the
  Settings surface with the eight tabs (`dist/gpui-settings-vault.png`,
  `dist/gpui-settings-theme.png`, `dist/gpui-settings-ai.png`,
  `dist/gpui-settings-research.png`, `dist/gpui-settings-developer.png`,
  `dist/gpui-settings-sync.png`, `dist/gpui-settings-sync-conflicts.png`,
  `dist/gpui-settings-tray.png` and `dist/gpui-settings-notifications.png`), and
  the Quick Capture dialog in text and voice mode (`dist/gpui-quick-capture.png`,
  `dist/gpui-quick-capture-voice.png`), and the Kanban board with its Done
  collapse and inline new-card row plus the first-run onboarding wizard
  (`dist/gpui-kanban.png`, `dist/gpui-kanban-done.png`,
  `dist/gpui-kanban-new.png`, `dist/gpui-onboarding.png`).
- `scripts/capture-gpui-mobile.sh` owns a private 1080×2400 Xvfb screen and
  performs fifteen mobile captures with `STRATUM_MOBILE=1` and
  `STRATUM_INSETS=48,0,72,0,0`: journal, calendar, page, search, graph, More
  sheet, Ask Notes, Query, Templates, Flashcards, Kanban, Reminders,
  Whiteboards, Plugins and Settings (`dist/gpui-mobile-*.png`). The script fails
  unless every PNG is exactly 1080×2400.
- `crates/stratum-ui/tests/interaction_contract.rs` opens headless gpui windows
  to cover click propagation (snippet/voice chips), journal start scroll, the
  reminder block-open request, the toast open action and the plugins
  tab/smoke-test/theme flows.
- `scripts/capture-gpui-editor-perf.sh` runs `stratum-editor-perf` on the 10k
  fixture, prints frame/render timings and gates only on `render_p95_ms >
  18.18` (55 fps work budget); the JSON report lands in
  `.hermes/perf/editor-10k-frames.json`.
- `crates/stratum-mobile` is the Android host. `android_main` boots the same
  shell through the vendored `gpui-mobile` NativeActivity glue
  (`dev.gpui.mobile.GpuiActivity`), `AndroidServices` implements
  `stratum_ui::mobile::MobileServices` (microphone, file picker, notifications,
  clipboard, URL opener), `StratumVault` resolves the on-device vault under the
  app documents directory, and `scripts/build-android.sh` builds
  `libstratum_mobile.so` for arm64-v8a + x86_64 and packages the debug APK. The
  crate is a stub on non-Android hosts so the workspace gates stay green. See
  `docs/development/gpui-ui.md#android-host`.
- `.github/workflows/ci.yml` is the single pipeline: workspace
  fmt/clippy/test, the Windows/macOS shell build, the parity harness (goldens,
  perf, a11y), the Android APK + emulator smoke test, the MCP coverage gate and
  `cargo audit`. The parity job uploads `.hermes/parity/` reports and the
  rendered surfaces as artifacts.
- `.github/workflows/gpui-release.yml` packages the native shell: deb, AppImage
  and rpm on ubuntu-22.04 (verified by install + headless launch in throwaway
  containers), MSI/NSIS on Windows, the arm64/x86_64 `.app` bundles on macOS and
  the signed APK/AAB, then publishes `stratum-gpui-*` assets with one
  `sha256sums.txt`. `scripts/package-desktop.sh` and `scripts/build-android.sh
  --release` are the same commands CI runs; `docs/development/packaging.md` is
  the configuration, signing and verification reference.

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
- Integration tests for the service layer (`crates/stratum-app/tests/`) and the shell contracts (`crates/stratum-ui/tests/`)
- Use `tempfile` for filesystem tests
- Target: every public function has at least one test

### Module Organization

Business logic lives in `stratum-app` (the headless service layer) and the
domain crate it wraps; the shell is a thin renderer over `ShellData`/`PanelData`.

| Surface | Service Method | Domain Logic |
|---------|----------------|--------------|
| `panels/graph` | `stratum-app::graph` | `pkm-block` (graph building), `stratum-graph` (physics) |
| `panels/settings` | `stratum-app::settings` | `pkm-core::Config` (DTO mapping) |
| `panels/search` | `stratum-app::search` | `pkm-index` (search/backlinks) |
| `panels/settings` (sync) | `stratum-app::sync` | `pkm-sync` (git operations) |

See `docs/development/service-layer.md` for the full command → service method
map and the inventory gate that keeps it honest.

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

# Native shell (fixture, no vault required)
cargo build -p stratum-ui
STRATUM_FIXTURE=1 cargo run -p stratum-ui

# Shell tests, contracts and lint
cargo test -p stratum-i18n -p stratum-platform -p stratum-ui -p stratum-editor
cargo clippy -p stratum-i18n -p stratum-platform -p stratum-ui -p stratum-editor --all-targets -- -D warnings

# Parity harness (golden screenshots, perf budgets, accessibility probe)
./scripts/verify-gpui-goldens.sh
./scripts/verify-gpui-perf.sh
./scripts/verify-gpui-a11y.sh

# Screenshots
./scripts/capture-gpui-shell.sh    # writes dist/gpui-shell.png
./scripts/capture-gpui-panels.sh   # writes the desktop panel matrix
./scripts/capture-gpui-mobile.sh   # writes dist/gpui-mobile-*.png

# Native installers (deb/AppImage/rpm; --formats for one platform)
scripts/package-desktop.sh

# Verify installers install and launch in clean containers
scripts/verify-packages.sh --dir dist/packages

# Android host (arm64-v8a + x86_64, debug APK)
scripts/build-android.sh
```
## License & Dependency Policy

Stratum is **AGPL-3.0-only**. Every new dependency (Rust crate) must be license-compatible with AGPL-3.0. The following licenses are always acceptable:
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
| Installer size (desktop, compressed) | < 20MB |

## Repo Hygiene — what gets committed

**Every agent/contributor MUST follow this. The repo contains only shipped, necessary code and user-facing docs — never implementation aids.**

| Path | Purpose | Committed? |
|------|---------|------------|
| `crates/`, `scripts/`, `nix/`, `deploy/`, `packaging/`, `goldens/`, `vendor/gpui-mobile/`, `*` config | Real code, tests, tooling, golden fixtures | ✅ yes |
| `docs/` only `docs/guide/`, `docs/development/`, `docs/getting-started/`, `docs/advanced/`, `docs/cli/`, `docs/development/adr/`, `docs/index.md`, `docs/contributing.md`, `docs/announcement.md`, `docs/assets/`, `docs/landing/` | Shipped user/dev reference docs (each must be in a mkdocs nav entry or referenced by another shipped doc) | ✅ yes |
| `.hermes/` (plans, reports), `.omo/`, `.opencode/`, `.sisyphus/`, `.worktrees/`, `.screenshots/`, `docs/qa/`, `test-results/`, `dist/`, `target/` | Plans, QA reports, findings, audits, repros, evidence, screenshots, build output | ❌ never |

**Rules:**
1. **Plans, QA reports, findings, audits, repros, and evidence go to `.hermes/`** (already gitignored). Never to repo root or `docs/`.
2. **Never commit an implementation aid** — no `*-plan.md`, `*-REPORT-*.md`, `*FINDINGS.md` at repo root; no `docs/fc-*.md`, `docs/audit-*.md`, `docs/qa/`; no `docs/acceptance-*.md`.
3. **`docs/` = shipped docs only.** If a doc is not reachable in the mkdocs nav and not referenced by another shipped doc, it does not belong in `docs/` — move it to `.hermes/`.
4. If a QA/report artifact is genuinely worth keeping as a shipped reference, it must be linked from the mkdocs nav (or another shipped doc) and live under `docs/development/` or `development/adr/` — otherwise it stays transient.
5. CI (`repo-hygiene` job, `scripts/check-transient-artifacts.sh`) fails if any transient artifact is tracked. If you need one tracked, add an explicit exception in that script and justify it with a comment.

**Canonical location for agent working files:** `.hermes/` — plans, task reports, evidence, scratch. `.hermes/` is gitignored so anything there is safe to write freely.

## Documentation Sync Rules

**These rules are MANDATORY for every code change.** Every agent and contributor must follow them:

| Trigger | Required Action |
|---------|----------------|
| New panel added | Add a row to `Native shell surfaces` above, a `docs/guide/` entry, a `*_I18N_KEYS` catalog gate, a capture state in `scripts/capture-gpui-panels.sh` and a golden in `goldens/gpui/` |
| Panel renamed | Update the table above, the guide, the router entry and the goldens |
| Panel module > 500 lines | Must split into sub-modules (file a refactoring task) |
| New shell module/hook | Update `docs/development/gpui-ui.md` |
| New Rust crate/module | Add to `Crate Dependency Graph` and `Workspace Layout` above; add module docs |
| Service method added or changed | Keep `stratum-app` authoritative, add a headless test, update `docs/development/service-layer.md` and the inventory gate |
| New feature added | Add `docs/guide/` entry; update contributing.md if dev workflow changes |
| Behavior change | Update relevant `docs/guide/` entry |
| UI changed | Regenerate the golden (`scripts/verify-gpui-goldens.sh --update`) and review the image diff |
| Mobile platform change | Update `docs/development/mobile-guide.md` |
| New dependency added | Verify AGPL-3.0 compatibility per `License & Dependency Policy` |
| Cargo.toml changed | Verify `Crate Dependency Graph` table above matches actual deps |
| Any user-facing string | Add it to all eight locale catalogs in `stratum-i18n` (the raw-key scan fails otherwise) |

## Inventory & Docs Alignment

### Command inventory gate

`docs/development/service-layer.md` is the shipped inventory of the command
surface. `crates/stratum-ui/tests/command_parity.rs` parses it and asserts every
row resolves to a live `stratum-app` method, so a removed or renamed service
method cannot silently orphan a feature. Adding or removing a row requires
updating the gate's `SHIPPED_COMMANDS` constant in the same PR.

### Code ↔ Docs Alignment

| Artifact | Must Match | Check Frequency |
|----------|-----------|-----------------|
| AGENTS.md panel table | Actual modules in `crates/stratum-ui/src/panels/` | Every PR |
| AGENTS.md crate table | Actual crates in `Cargo.toml` workspace | Every PR |
| `goldens/gpui/` | The current shell rendering | Every PR (`parity` job) |
| docs/guide/* | Features actually implemented in code | On feature add |
| contributing.md | Actual CI workflow | On CI change |
| Rust doc comments | Actual Rust API | Every commit |

## Refactoring Roadmap

See `.sisyphus/refactoring-plan.md` for the full phased plan covering:

1. **Shell**: panel decomposition, shared widget primitives, mobile chrome parity, golden coverage for every state
2. **Rust**: comment audit → eliminate critical duplication → error type unification → test expansion → cross-crate cleanup → fix stub implementations
3. **Docs**: AGENTS.md accuracy → development guides → panel architecture docs → screenshot gaps

## Screenshot PII Gate (mandatory before commit)
Any screenshot/render/evidence image committed to this repo must have NO personal information:
names, emails, phones, addresses, API tokens, real usernames, IPs, `/home/<user>` paths,
`user@host` strings, hostnames. Redact (solid matching-color box) first; verify before commit.
