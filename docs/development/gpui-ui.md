# Native GPUI Shell

Stratum ships two front ends:

- The frozen React + Tauri application in `src/` and `src-tauri/`, which remains
  the reference implementation and is not modified by the native shell work.
- The native GPUI shell in `crates/stratum-ui`, which reproduces the React
  shell's chrome (sidebar, navigation, recents, footer, error bar and
  toaster) on top of the headless service layer.

`crates/stratum-i18n` provides the locale catalogs the native shell renders.
Both crates are workspace members and are covered by the GPUI CI workflow.

---

## Crate Layout

```
crates/
├── stratum-i18n/
│   ├── locales/            # byte-identical copies of src/i18n/locales/*.json
│   ├── src/lib.rs          # catalogs, interpolation, parity helpers
│   └── tests/              # catalog + parity gates
└── stratum-ui/
    ├── assets/             # embedded SVG icons, Inter faces, OFL notice
    ├── src/
    │   ├── lib.rs          # crate root, actions, init
    │   ├── main.rs         # binary entry point (1280×720 window), start-state flags
    │   ├── assets.rs       # embedded asset source + font registration
    │   ├── provider/
    │   │   ├── mod.rs      # ShellData seam and provider selection
    │   │   ├── assets.rs   # attachment/thumbnail seam
    │   │   ├── journal.rs  # journal date discovery, creation and DTO mapping
    │   │   ├── page.rs     # page metadata, menu actions, backlinks, suggestions seam
    │   │   ├── voice.rs    # voice-capture seam (start/stop/transcribe/insert)
    │   │   ├── fixture/    # deterministic fixture provider (pages, blocks, journal, page)
    │   │   └── tests.rs    # provider contract tests
    │   ├── router.rs       # routes, navigation, shortcuts, path codec
    │   ├── theme.rs        # shipped palette derivation mapped to gpui-kit
    │   ├── shell/          # sidebar, footer, error/toaster chrome, reminder toaster, journal/page wiring
    │   └── panels/         # panel bodies (journal, page, voice) and icon mapping
    │       ├── journal/    # JournalView: state, navigation, reload, dates, stack, header, calendar, rows, error
    │       ├── page/       # PageView: header, Note menu, move/delete dialogs, backlinks, suggestions, actions, state
    │       └── voice/      # VoiceCapture: stage machine and gpui entity
    └── tests/              # route, theme, layout, asset, journal, page and editor contracts
```

The React application is the visual ground truth. Route order, navigation
order, shortcut modifiers, palette formulas and locale keys are pinned by
tests against the shipped TypeScript implementation.

---

## Modules

### `stratum-i18n`

`stratum_i18n::Locale` enumerates the eight supported catalogs (`en`, `de`,
`el`, `es`, `fr`, `it`, `ja`, `zh`). The catalogs are embedded with
`include_str!`, flattened into dotted lookup keys and cached process-wide.

| API | Purpose |
|-----|---------|
| `t(locale, key)` | Resolve a key, returning the key itself when missing |
| `tr(locale, key, vars)` | Resolve and i18next-compatible `{{name}}` interpolate |
| `set_locale` / `current_locale` | Process-wide active locale |
| `t_global` / `tr_global` | Resolve in the active locale |
| `key_set` | Flattened user-facing keys (`_meta` excluded) |
| `missing_keys` / `extra_keys` | Parity drift helpers against English |

A missing key deliberately resolves to the raw key so an unfinished catalog is
visible in the UI rather than silently falling back to English.

### `stratum-ui`

`stratum_ui::Shell` is the root view. It owns the active route, read-mode and
sidebar state, and renders the shell chrome. The shell does not own business
logic: data comes from an `AppProvider` implementation.

| Module | Responsibility |
|--------|----------------|
| `router` | `Panel`, `ROUTES`, `NAV_ITEMS`, `SHORTCUTS`, `parse_path`, `path_for`, percent codec |
| `theme` | `ThemeConfig`, `derive_palette`, `apply_to_kit`, golden palette formulas |
| `assets` | `StratumAssets` (icons + fonts), `register_fonts` |
| `provider` | `ShellData`, `ShellSnapshot`, `FixtureProvider`, `provider_from_env`, asset seam (`provider/assets.rs`), journal seam (`provider/journal.rs`), page seam (`provider/page.rs`), voice seam (`provider/voice.rs`) and the panel data seam (`provider/panels.rs`, `provider/app_panels.rs`, `provider/fixture/panels`) |
| `shell` | header, sidebar, footer, error bar, toaster (`shell/reminder_toast.rs`: due-reminder polling, announce-once state and the toast open action), journal/page view wiring (`shell/page.rs`), search/query/ask/templates/flashcards/reminders/plugins panel wiring (`shell/panels.rs`) and start-state setters |
| `panels` | panel body rendering and icon mapping; `panels/journal` (entity, state, navigation, reload, calendar/stack/read-mode surface), `panels/page` (entity, header, Note menu, move/delete dialogs, backlinks, suggestions, actions), `panels/search` (model, entity, result rows), `panels/query` (model, entity, builder rows, toolbar, dialogs, saved list, results), `panels/ask` (entity, answer card), `panels/templates` (entity, variable model, cards), `panels/flashcards` (session model, entity), `panels/reminders` (grouping/humanize model, entity, dialog), `panels/plugins` (entity, installed/directory/themes tabs, actions), `panels/graph` (entity, model, canvas input/letterbox, layout job, scene build, settings, toolbar) and `panels/voice` (capture stage machine and entity) |

The provider seam is the only place the shell touches the service layer. The
real provider drives `stratum_app::StratumService`; `FixtureProvider` returns a
deterministic 87-page snapshot for tests and the capture harness.

`ShellData` also carries the attachment seam used by the native editor
(`read_asset_thumbnail`, `page_attachments`, `import_asset_bytes`,
`import_asset_path`, `delete_asset`). The trait defaults keep providers without
asset support compiling — no thumbnail, an empty list, a pass-through row and a
no-op delete. `AppProvider` maps the seam onto `stratum_app::asset`
(`asset_thumbnail`, `import_asset_bytes_checked`, `import_asset_path_checked`,
`get_page_asset_refs`, `delete_asset`); `FixtureProvider` keeps an in-memory
asset map with deterministic `assets/<stem>-<12 hex>.<ext>` names (v5 UUID over
the bytes) and lists the rows whose references appear in the saved page content,
so the drop/paste flow is testable without a vault.

The journal seam mirrors the shipped `JournalPanel` data flow. `journal_dates`
returns the dates that already have journal pages (newest first),
`ensure_today_journal` creates today's page if it is missing, and
`create_journal_day` creates a page for an arbitrary date. `AppProvider` maps
these onto `StratumService` page/block APIs and parses the `journals/<date>.md`
paths into sorted dates. `FixtureProvider` ships a fixed today (`2026-11-10`), ten past days
(`2026-11-09` back to `2026-10-31`, newest first) and a recording at
`assets/voice/fixture.flac`; transcription inserts two deterministic blocks so
the capture harness can exercise the full voice flow without a microphone.

The voice seam (`stt_configured`, `voice_start`, `voice_stop`,
`voice_transcribe`, `voice_insert_clip`) has neutral trait defaults so providers
without audio support still compile. The trait defaults report STT as
unconfigured, fail `voice_start`, return an empty take and skip insertion.

The page seam mirrors the shipped `PageView`: `page_info` returns the title,
slug, block count and modification time, the menu actions map onto
`reindex_page`, `normalize_file`, `move_page` and `delete_page`, `backlinks` and
`backlink_snippet` back the linked/unlinked card, and `suggest_connections` /
`insert_block` back the related-note card. `AppProvider` maps these onto
`stratum_app::StratumService`; `FixtureProvider` returns deterministic page
metadata, two linked + one unlinked backlink, one snippet and two suggestions for
`pages/fixture.md`. The pure helpers (`normalize_move_path`, `group_backlinks`,
`clamp_relevance`) live in `provider/page.rs` and are covered by window-free
tests.

### Search

`panels/search` ports the shipped desktop `SearchPanel`. `model.rs` owns the
window-free routing (`classify_query`: leading `#` selects tag mode with the
marker stripped; empty or a bare `#` is idle), the `SearchStatus` machine and
the `score_label` caption (`search._score` + two decimals). `mod.rs` renders the
header (`search.search_blocks`), the query field
(`search.search_blocks_prefix_with_to_search_tags`), the `common.search` submit
button, Enter submission, the rebuild action
(`search.rebuild_search_index` / `search.indexing`) and the `search.failed`
error prefix. `rows.rs` renders one `search-result-{index}` row per hit with the
page path, score label and snippet; clicking a row records a pending navigation
to `page_path` that the shell routes through `handle_editor_navigation`.

Provider calls stay on `PanelData`: `search_by_tag(tag)` for tag mode,
`search_blocks(query, 20)` for full-text and `rebuild_search_index()` for the
rebuild action. The fixture returns three deterministic hits and two tag hits.
The start flag `STRATUM_SEARCH=<query>` submits a query on construction; a
`#tag` value exercises tag mode. Captures: `dist/gpui-search.png` (full-text
`todo`) and `dist/gpui-search-tag.png` (`#project`).

### Query

`panels/query` ports the shipped `QueryPanel.desktop`, `BuilderMode`,
`ConditionRow` and `SavedQueryDialogs`. `model.rs` owns the `QueryTab` pair and
the `PanelState` transitions; every builder edit calls
`stratum_app::query::builder::{build_datalog, parse_datalog_to_rows}` so the
native panel emits byte-identical queries. Switching to Builder parses the
buffer (invalid input surfaces `query.builder_not_parseable`), invalid specs
surface the `BuildError::key()` message instead of running, reset restores the
shipped reset query and copy-to-editor moves the generated query into the raw
buffer with the `query.copied_to_editor` toast. `builder_rows.rs` renders one
row per `FilterSpec` with field/operator choosers, a value input, combiner,
move and remove controls plus the generated-query preview; `toolbar.rs` owns the
tab bar, raw editor and Run/Save/Reset actions; `dialogs.rs` owns the save,
rename and delete confirmations; `saved.rs` renders the saved list with
`format_updated_at` subtitles, load/rename/delete controls and refresh;
`results.rs` renders the result table.

Provider calls stay on `PanelData`: `run_query`, `saved_queries`, `save_query`,
`rename_query` and `delete_query`. The fixture seeds two saved queries and
returns a 2×3 result table. Start flags: `STRATUM_QUERY=<datalog>` prefills the
editor, `STRATUM_QUERY_TAB=builder|datalog` selects the tab and
`STRATUM_QUERY_RUN=1` runs the query after construction. Captures:
`dist/gpui-query.png` (builder), `dist/gpui-query-results.png` (run) and
`dist/gpui-query-datalog.png` (raw tab).

### Ask Notes

`panels/ask` ports the shipped desktop `AskNotesPanel`. `mod.rs` renders the
title (`ask_notes.ask_your_notes`), the caption
(`ask_notes.get_answers_grounded_in_your_vault_with_cited_sources`), the
multi-line question field, Enter submission, the `ask_notes.ask` action and the
`ask_notes.clear` reset; `answer.rs` renders the searching row
(`ask_notes.searching_your_notes`), provider and read-aloud failures, the answer
card with its read-aloud control (`editor.read_aloud` / `ask_notes.playing`), the
`ask_notes.sources` citation rows (path, snippet, two-decimal score) and the
`ask_notes.no_matching_notes_found_the_answer_is_based_on_general_knowl` caption
when the answer had no sources. Citations are informational in the shipped UI, so
the panel never records a pending navigation.

Provider calls stay on `PanelData`: `ask_notes(question)` for the query and
`ShellData::speak(text)` for read-aloud (the same playback path the editor uses).
The fixture answers questions containing "decide" with two cited sources and
returns the no-sources answer otherwise. The start flag
`STRATUM_ASK=<question>` submits the question on construction. Capture:
`dist/gpui-ask.png` (the two-source fixture answer).

### Templates

`panels/templates` ports the shipped desktop `TemplatesPanel`, extended with the
variable-placeholder chips and the starter-template installer. `mod.rs` renders
the title, the target-page field (`templates.target_page_path`), the variables
editor (`templates.variables` with `query.name` / `query.value` placeholders and
`templates.add_variable`), the `settings.install_starter_templates` action and the
inline status area; `model.rs` owns the blank-key filtering that mirrors the
shipped `variables.filter(([k]) => k.trim())`; `cards.rs` renders one card per
template with the name, description, `{{name}}` placeholder chips derived from
the template content through `stratum_app::template::template_variables`, and the
`templates.apply` action, plus the empty state.

Applying with an empty target is a no-op. On success the panel clears the target
field and records a `templates.created` toast with the target path; a provider
failure records the error toast. Installing starter templates refreshes the list
and records a toast listing the installed names. Provider calls stay on
`PanelData`: `templates()`, `apply_template(name, target, vars)` and
`install_starter_templates()`. The fixture exposes three templates (Daily note,
Meeting, Project). Capture: `dist/gpui-templates.png`.

### Flashcards

`panels/flashcards` ports the shipped desktop `FlashcardsPanel`. `model.rs` owns
the window-free `SessionState` machine (`Loading`, `Empty`, `Card`, `Complete`,
`Failed`), the reveal/rate/restart transitions and the
`flashcards.card_of_count` progress caption; `mod.rs` renders the header, the
empty-state hint (`flashcards.question` / `flashcards.answer` /
`flashcards.properties_to_generate_cards`), the clickable question/answer card
with its reveal labels, the source/ease/interval caption, the SM-2 rating row and
the session-complete view with `flashcards.start_again`.

Ratings use the shared `stratum_app::flashcards::{FlashcardRating,
rating_quality, is_correct}` table (0/2/3/5; correct is quality ≥ 3), so the
native buttons and the backend SM-2 grades cannot drift. Rating feedback uses
`flashcards.correct` / `flashcards.review_again_soon`, and the four buttons use
the catalog keys `flashcards.rating_blackout`, `_hard`, `_good` and `_easy`.
Provider calls stay on `PanelData`: `flashcards()` and
`review_card(id, quality, page_path)`. The fixture serves three cards and records
the reviewed qualities. The start flag `STRATUM_FLASHCARD_REVEAL=1` reveals the
first answer. Captures: `dist/gpui-flashcards.png` (question) and
`dist/gpui-flashcards-answer.png` (revealed answer and rating row).

Highlight-on-open lives in `panels/editor/reveal.rs`: `EditorView::reveal_block`
locates the row, scrolls to the layout offset and records a highlight that the
row renderer draws as a soft tint. The highlight clears on the next edit or when
another block is revealed. The shell exposes
`Shell::open_page_with_highlight(path, block_id)` for navigation that carries a
block anchor.

### Reminders

`panels/reminders` ports the shipped desktop `RemindersPanel`. `model.rs` owns
the window-free urgency grouping (overdue / today / upcoming), the humanized due
captions and the set/change/clear dialog state; parsing, bucketing and the
`datetime-local` conversion reuse `stratum_app::reminders::due`
(`parse_due`, `bucket`, `humanize`, `due_to_local_input`, `local_input_to_utc_iso`)
so the native panel and the React reminder utilities agree on the same rules.
`mod.rs` renders the header with the `reminders.panel.due` refresh action, the
three sections with per-row snooze/complete/open/change chips, the empty state
and the provider failure line; `dialog.rs` renders the due dialog (input, the
`reminders.dialog.in_1_hour` / `_tomorrow_0900` / `_evening_1800` quick choices,
the invalid hint, the clear/cancel/save actions and the provider error). The row
chips use the catalog labels (`reminders.panel.snooze_10min`, `snooze_1h`,
`complete`, `open`) instead of the shipped literal `10m` / `1h` / `Done` /
`Open` strings, so the reminder actions localize with the rest of the UI.

Provider calls stay on `PanelData`: `reminders()`, `snooze_reminder`,
`clear_reminder` and `set_reminder`. "Open" records a pending block-open request
that the shell drains into `open_page_with_highlight`, matching the shipped
`RemindersPanel`. The fixture serves five reminders around the local clock and
mutates the list in place. The start flag
`STRATUM_REMINDER_DIALOG=<block-id>` opens the due dialog on construction.
Captures: `dist/gpui-reminders.png` (grouped list) and
`dist/gpui-reminders-dialog.png` (dialog for the first overdue row).

The shell also owns the reminder toaster: `Shell::poll_reminders` asks the
provider for due reminders, announces each block once while it stays due, and
prunes the id when the reminder is snoozed or cleared so the next due window
toasts again. The 30 s background poll wakes the shell because render is only
driven by notifications; `maybe_poll_reminders` gates both the timer and the
render path to one poll per 30 s. The toast reuses the shipped
`reminders.toast.due_title` / `reminders.toast.open` keys and its action routes
to the reminder's page with the block revealed. `STRATUM_REMINDER_TOAST=1` makes
the fixture report due reminders. Capture: `dist/gpui-reminder-toast.png`.

### Plugins

`panels/plugins` ports the shipped desktop `PluginsPanel`. `mod.rs` owns the
`PluginsTab` state, the accessors and the status-chip mapping; `actions.rs` owns
every provider mutation and catalog refresh; `installed.rs` renders the plugin
cards (manifest metadata, status chip with expandable error detail,
enable/disable/reload/uninstall and the note-read/HTTP smoke-test row);
`directory.rs` renders the community catalog with the offline badge, fetched-at
and verified-sha256 captions and install/uninstall per entry; `themes.rs`
renders installed themes plus catalog entries and applies an installed theme
through `stratum_app::themes` and `crate::theme::apply_to_kit`, with the
built-in fallback when the active theme is uninstalled.

Provider calls stay on `PanelData`: `plugins`, `set_plugin_enabled`,
`reload_plugin`, `uninstall_plugin`, `install_plugin_path`,
`install_plugin_url`, `plugin_note_read`, `plugin_http_request`,
`plugin_catalog`, `theme_catalog`, `installed_themes`, `install_theme`,
`apply_theme` and `uninstall_theme`. Install-from-file opens the platform path
picker off the UI thread, matching the editor's attachment picker. The fixture
ships two plugins, a three-entry catalog, a two-entry theme catalog and an empty
installed-theme list. The start flag `STRATUM_PLUGINS_TAB=installed|directory|themes`
selects a tab on construction. Captures: `dist/gpui-plugins.png` (installed
tab), `dist/gpui-plugins-directory.png` (catalog) and
`dist/gpui-plugins-themes.png` (theme hub).

### Graph

`stratum-graph` is the headless graph engine; `panels/graph` is the native
surface. The crate ports the shipped `d3-force-3d` pipeline: seeded
deterministic positions, the Barnes-Hut octree, charge/link/collide forces, the
per-node z-spring (`vz += (tz - z) * 0.08`), alpha/velocity decay and the
shipped `cooldownTicks` settle window. `ForceParams` carries the canonical
`pkm_core::GraphConfig` defaults documented in
`docs/getting-started/configuration.md` (charge `-30`, link distance `100`,
alpha decay `0.02`, velocity decay `0.4`, curvature `0.15`) and `set_params`
reheats alpha like `d3ReheatSimulation()`. `SimWorker` advances the layout off
the UI thread and its tick/snapshot sequence is byte-identical to a direct
`Simulation`.

`camera` reproduces the shipped `TrackballControls` semantics: `PAN_SPEED = 0.15`
with the TrackballControls pan formula (`2 * distance * tan(fov/2) / height *
panSpeed`), `ZOOM_SPEED = 1.2` with the `0.95^zoomSpeed` dolly step, the 0.8
normalized WASD/arrow step that moves camera and target together, the 500 ms
cubic focus tween cancelled by any control interaction, and `fit_to_positions`
for the initial framing.

`render` builds wgpu pipelines for instanced nodes, curved links (the shipped
curvature setting), SDF billboard labels from the embedded Inter face and a GPU
picking pass, plus a deterministic CPU rasterizer for hosts without an adapter.
`Renderer::auto` selects the GPU backend when one exists and reports which
backend it chose.

`panels/graph` wires the shipped `GraphPanel`: provider graph load, toolbar
filters (full/component/orphans and title/tag search), visibility toggles, force
sliders, the node-cap selector with the shipped cap banner, click-to-open,
right-click focus tween, hover highlight, orbit/pan/dolly and WASD/arrow keys.
The panel loads and saves the persisted force/visibility settings through
`PanelData::graph_settings` / `save_graph_settings`; the node cap stays
in-memory exactly like the shipped panel. Layouts above 512 nodes run on the
worker so the UI thread keeps painting while the cached frame re-renders on
camera input; smaller graphs build synchronously so the fixture captures stay
deterministic. When no GPU adapter is available the panel switches to the
deterministic software rasterizer, uses the orthographic 2D camera and shows the
shipped `graph.fallback_notice_webgl` banner.

The golden runner (`cargo run --release -p stratum-graph --bin graph_golden --
--assert-fps`) writes deterministic PNG frames, the frame-time histogram, the
FPS/RSS table and the verdict to `.hermes/e6/`. On the RTX 4090 host the release
run measured the 10k-node fixture at 490 fps (p95 2.07 ms) and the uncapped
20k-node fixture at 193 fps (p95 7.55 ms), with peak RSS at 311 MB inside the
800 MB gate; the golden frame hashes are byte-identical across runs (seed `1`,
120 ticks, 120 frames). The 5k fixture renders at 543 fps (p95 2.22 ms); the
Android 30 fps-at-5k target is a projection from this renderer path (the
hardware-Vulkan measurement belongs to the mobile lane).

Known deviations from the shipped look: link particles
(`linkDirectionalParticles`) are not animated because the native panel paints a
cached frame and the golden frames must stay byte-stable; the settings panel
also shows the `use_3d` toggle that the shipped desktop panel hides (the plan
lists it in the graph settings surface); and the shipped progressive-loading
progress bar (`graph.rendering`) is not ported — the worker-backed layout keeps
the previous frame visible while it lays the graph out, so there is no
per-batch progress to report.


---

### Whiteboards

`stratum-canvas` is the headless whiteboard engine behind the native panel; the gpui
surface is thin over it and lives in `crates/stratum-ui/src/panels/canvas/`
(`mod`, `gallery`, `editor`, `canvas`, `toolbar`, `library`, `actions`, `edit`,
`runtime`, `helpers`, `model/{mod,session,scene}`).

| Concern | Module |
|---------|--------|
| Element model, unknown-field passthrough, `.excalidraw`/`.excalidrawlib` IO | `stratum-canvas::{element, document, library}` |
| Tools, snapping, undo/redo, clipboard, style patches, alignment | `stratum-canvas::{tools, geometry, history}` |
| `Board#Element` links and deep-link query parsing | `stratum-canvas::link`, `stratum_ui::router::parse_target` |
| roughr scene generation (Excalidraw's rough options, arrowheads, frames) | `stratum-canvas::render::{rough, rough_arrow, scene}` |
| SVG / PNG / WebP export and 300 px previews | `stratum-canvas::render::{svg, raster}` |
| 5k-element visibility query and perf harness | `stratum-canvas::render::{scene::SceneCache, perf}`, `canvas_perf` |

Behaviour notes:

- The gallery/editor are wired through `PanelData` (`whiteboards`, `load_whiteboard`,
  `save_whiteboard`, `create_whiteboard`, `rename_whiteboard`, `delete_whiteboards`,
  `whiteboard_library`, `save_whiteboard_library`, `write_canvas_export`) and the
  service-backed and fixture providers implement all of them.
- Autosave debounces 800 ms, flushes on back-navigation, regenerates the WebP preview
  (`preview` + `previewTheme`) and never writes a board that is not dirty.
- Pan/zoom repaints from the cached `SceneCache::visible` set; the rough scene is rebuilt
  only when the document revision changes.
- Text editing is inline through the gpui input overlay (IME-capable), positioned over the
  edited element. Enter or blur commits the text, Escape cancels the edit and returns focus
  to the canvas.
- Exports use the document's raw colours and `viewBackgroundColor`; only the gallery
  thumbnail applies the shipped preview theme adaptation.
- `canvas_perf --elements 5000 --frames 600` reports p95 0.063 ms for the per-frame
  visibility query (budget 18.18 ms) and a 261.8 ms one-time scene build on this host;
  `--text 500` builds a 500-element text board in 4.4 ms with a p95 of 0.004 ms. The
  gate is `p95_ms <= 18.18`; `crates/stratum-canvas/tests/perf.rs` asserts both scenarios.
- Known deviations from the Excalidraw reference: text renders with the bundled Inter
  face instead of Excalifont, WebP previews are lossless (VP8L) because `image-webp` has
  no lossy encoder, freedraw is stroked as a polyline rather than smoothed and filled,
  arrowheads derive from the element points instead of the roughened sketch, there is no
  laser pointer, and `roughr` 0.14's random jitter sequence differs from `rough.js`, so
  the hand-drawn wobble is a few units off while geometry, seeds, colours and element
  composition match. Legacy v1 boards are normalised the way Excalidraw's `restore()`
  does (`strokeSharpness: "round"` → `LEGACY`/`PROPORTIONAL_RADIUS` roundness), so they
  render with the same corner radius as the web app.

Start flags for captures and manual checks: `STRATUM_WHITEBOARD=<name>`,
`STRATUM_WHITEBOARD_ELEMENT=<frame-or-id>`, `STRATUM_WHITEBOARD_TOOL=<tool>`,
`STRATUM_WHITEBOARD_LIBRARY=1`, `STRATUM_WHITEBOARD_STYLE=1`,
`STRATUM_WHITEBOARD_EXPORT=svg|png`.

## Building and Running

On Linux the shell links against xkbcommon and X11, so the development host
needs `libxkbcommon-dev`, `libxkbcommon-x11-dev`, `libx11-dev` and `libxcb1-dev`
(the CI `install-linux-deps` action installs these).

```bash
# Build the shell
cargo build -p stratum-ui

# Run against the fixture (no vault required)
STRATUM_FIXTURE=1 cargo run -p stratum-ui

# Run against a real vault
STRATUM_VAULT=/path/to/vault cargo run -p stratum-ui

# Select a UI catalog
STRATUM_LOCALE=de STRATUM_FIXTURE=1 cargo run -p stratum-ui
```

The binary opens a 1280×720 window. Shortcuts are installed from the shared
`SHORTCUTS` table: `Ctrl/Cmd+Shift+R` toggles read mode, `Ctrl/Cmd+B` toggles
the sidebar and `Escape` dismisses the topmost overlay.

### Environment variables

| Variable | Effect |
|----------|--------|
| `STRATUM_FIXTURE=1` | Use the deterministic fixture provider |
| `STRATUM_VAULT=/path` | Open the real service provider for that vault |
| `STRATUM_LOCALE=<code>` | Select one of the eight catalogs |
| `STRATUM_COLLAPSED=1` | Start with the 52 px icon rail (capture harness) |
| `STRATUM_ROUTE=/journal` | Open a route through the shared router at start-up |
| `STRATUM_PAGE=pages/Welcome.md` | Open a page in the editor at start-up |
| `STRATUM_JOURNAL_DATE=2026-11-09` | Start the journal on a specific day |
| `STRATUM_CALENDAR=1` | Start with the journal calendar expanded |
| `STRATUM_READ_MODE=1` | Start in read mode |
| `STRATUM_MENU=note` | Start with the page Note menu open |
| `STRATUM_CARDS=open` | Start with the page backlinks/suggestions expanded |
| `STRATUM_HIGHLIGHT=<block-id>` | Reveal a block when the page opens |
| `STRATUM_SNIPPET=1` | Preload the first linked backlink's snippet card |
| `STRATUM_SEARCH=<query>` | Submit a search query on the search panel (`#tag` uses tag mode) |
| `STRATUM_QUERY=<datalog>` | Prefill the query panel's Datalog editor |
| `STRATUM_QUERY_TAB=builder\|datalog` | Select the query panel tab |
| `STRATUM_QUERY_RUN=1` | Run the query after construction |
| `STRATUM_ASK=<question>` | Submit a question on the Ask Notes panel |
| `STRATUM_FLASHCARD_REVEAL=1` | Reveal the first flashcard's answer |
| `STRATUM_REMINDER_DIALOG=<block-id>` | Open that reminder's due dialog |
| `STRATUM_REMINDER_TOAST=1` | Make the fixture report due reminders for the toaster |
| `STRATUM_PLUGINS_TAB=installed\|directory\|themes` | Select the plugins panel tab |
| `STRATUM_GRAPH_MODE=full\|component\|orphans` | Select the graph toolbar view mode |
| `STRATUM_GRAPH_COMPONENT=<index>` | Select a connected component for component mode |
| `STRATUM_GRAPH_SETTINGS=1` | Open the graph settings panel |
| `STRATUM_GRAPH_SEARCH=<text>` | Prefill the graph title/tag filter |
| `STRATUM_GRAPH_TICKS=<count>` | Set the deterministic layout tick count |

The start-state flags are applied by `apply_start_state` in `main.rs` and exist
for the capture harness; they do not change the interactive behaviour of the
shell.

---

## Testing

The GPUI crates are tested without opening a window:

```bash
cargo test -p stratum-i18n -p stratum-ui
cargo clippy -p stratum-i18n -p stratum-ui --all-targets -- -D warnings
```

The suites cover:

- Catalog parity, interpolation, missing-key behavior and locale round-trips.
- Route/navigation/shortcut tables against the shipped React order.
- Theme derivation goldens generated from the shipped TypeScript formulas.
- Shell geometry, navigation state, export success/failure, toaster and error
  bar behavior.
- Embedded icon/font availability and the `SHELL_I18N_KEYS` contract in all
  eight locales.
- The journal surface (`tests/journal_contract.rs`): localized date labels,
  calendar grid generation, the seven-slot week layout, the lazy stack plan and
  window growth, the voice stage machine, the fixture journal/voice data and the
  `JOURNAL_I18N_KEYS` release gate in all eight catalogs.
- The page surface (`tests/page_contract.rs`): the Note menu order/labels and
  destructive styling, move-path validation, backlink grouping, relevance
  clamping, the suggestion state machine, the fixture page seam and the
  `PAGE_I18N_KEYS` release gate in all eight catalogs. Unit tests also cover the
  highlight reveal model and the delete-confirmation message.
- The Reminders and Plugins surfaces (`tests/panel_contract.rs`): the
  `REMINDERS_I18N_KEYS` and `PLUGINS_I18N_KEYS` release gates in all eight
  catalogs, the urgency grouping/humanize/dialog model, the status-chip mapping
  and the fixture reminder, plugin, catalog and theme flows.
- The graph surface (`tests/graph_contract.rs`): the shipped force defaults,
  the `GRAPH_I18N_KEYS` release gate in all eight catalogs, the filter/cap policy
  (full/component/orphan/tag modes, explicit caps and the banner inputs), the
  fixture graph shape, the capture start-state parsing, the camera pan/zoom
  parity formulas, the frame-time histogram and the deterministic worker layout.
- Headless interaction (`tests/interaction_contract.rs`): nested controls stop
  click propagation into the parent handler (the backlink snippet chip must not
  navigate, the voice dismiss chip must not restart recording), starting the
  journal on a past day scrolls that section into view, a reminder row records
  the block-open request, the toast open action navigates to the reminder's
  page, and the plugins tabs/smoke tests/theme apply drive the provider.

`stratum_ui` unit tests use the built-in `#[test]` attribute. `gpui_kit::*`
re-exports GPUI's own `test` attribute macro, so test modules that glob-import
the kit import the built-in attribute explicitly to avoid creating a GPUI app
context.

---

## Screenshot Capture

`scripts/capture-gpui-shell.sh` starts the shell in fixture mode on a private
Xvfb screen, waits for the window, and captures it with ImageMagick. It captures
both sidebar states:

```bash
./scripts/capture-gpui-shell.sh
# → dist/gpui-shell.png            (expanded, 224 px sidebar)
# → dist/gpui-shell-collapsed.png  (52 px icon rail)
```

The expanded capture is the parity reference for the shipped shell; the
collapsed capture pins the second geometry from §3. Measure them before
accepting a change: the sidebar/main boundary must sit at x=224 expanded and
x=52 collapsed (the divider pixel included), the expanded sidebar must show 12
navigation rows in the shipped order, and the collapsed rail must show the same
12 icons with no labels and no recents list.

The output path can be overridden with the first argument or `OUTPUT` (the
collapsed file derives from it). `STRATUM_COLLAPSED=1` is used for the rail
because Xvfb runs without a window manager, so synthetic key events are not
reliably delivered; the shortcut itself is covered by the `SHORTCUTS` contract
test. The script requires `Xvfb`, `xdotool` and ImageMagick's `import`. It is
used by the GPUI CI workflow to publish a screenshot artifact, and is safe to
run locally without a desktop session.

`scripts/capture-gpui-panels.sh` captures the journal, page, search, query, Ask
Notes, Templates, Flashcards, Reminders, Plugins and Graph surfaces using the
start-state flags:

```bash
./scripts/capture-gpui-panels.sh
# → dist/gpui-journal.png           (edit mode)
# → dist/gpui-journal-calendar.png  (STRATUM_CALENDAR=1)
# → dist/gpui-journal-read.png      (past day, STRATUM_READ_MODE=1)
# → dist/gpui-page.png              (fixture page, edit mode)
# → dist/gpui-page-note-menu.png    (STRATUM_MENU=note)
# → dist/gpui-page-read.png         (STRATUM_READ_MODE=1)
# → dist/gpui-page-cards.png        (STRATUM_CARDS=open)
# → dist/gpui-page-highlight.png    (STRATUM_HIGHLIGHT=<fixture block id>)
# → dist/gpui-page-snippet.png      (STRATUM_CARDS=open STRATUM_SNIPPET=1)
# → dist/gpui-search.png            (STRATUM_ROUTE=/search STRATUM_SEARCH=todo)
# → dist/gpui-search-tag.png        (STRATUM_SEARCH=#project)
# → dist/gpui-query.png             (builder tab)
# → dist/gpui-query-results.png     (STRATUM_QUERY_RUN=1)
# → dist/gpui-query-datalog.png     (STRATUM_QUERY_TAB=datalog)
# → dist/gpui-ask.png               (STRATUM_ROUTE=/ask STRATUM_ASK=…)
# → dist/gpui-templates.png         (STRATUM_ROUTE=/templates)
# → dist/gpui-flashcards.png        (STRATUM_ROUTE=/flashcards)
# → dist/gpui-flashcards-answer.png (STRATUM_FLASHCARD_REVEAL=1)
# → dist/gpui-reminders.png         (STRATUM_ROUTE=/reminders)
# → dist/gpui-reminders-dialog.png  (STRATUM_REMINDER_DIALOG=<block id>)
# → dist/gpui-reminder-toast.png    (STRATUM_REMINDER_TOAST=1)
# → dist/gpui-plugins.png           (installed tab)
# → dist/gpui-plugins-directory.png (STRATUM_PLUGINS_TAB=directory)
# → dist/gpui-plugins-themes.png    (STRATUM_PLUGINS_TAB=themes)
# → dist/gpui-graph.png             (STRATUM_ROUTE=/graph)
# → dist/gpui-graph-component.png   (STRATUM_GRAPH_MODE=component)
# → dist/gpui-graph-settings.png    (STRATUM_GRAPH_SETTINGS=1)
```

The journal edit-mode capture is the parity reference for the shipped
`JournalPanel`: the header shows the localized day label with previous/next-day
and calendar controls, the today section renders the editor, and the stacked past
days follow the same section order as the React panel. Starting the journal on a
past day scrolls that section into view, centred like the shipped panel's
`scrollIntoView({block: 'center'})`. The read capture selects a deeper past day
(2026-11-07) so that the centred scroll is visible in the PNG instead of today's
section sitting at the top; the fixture carries ten past journal days so the
stack is taller than the capture window. The calendar capture pins the popup
geometry and weekday headers; the read capture pins the read-mode treatment of a
past day.

The page captures pin the shipped `PageView`: the header shows the title, path
chip, voice/attach/read controls and the `Note` menu; the Note-menu capture shows
the ordered actions; the read capture shows the read-mode treatment and the
edit-mode button; the cards capture shows the backlinks and suggestions cards
expanded; the highlight capture shows the revealed block tint. The search
captures pin the `SearchPanel` parity surface: the full-text capture shows the
classified mode chip, the result rows with score labels and the result count,
and the tag capture shows tag mode with the `#project` tag results. The query
captures pin the `QueryPanel` surface: the builder tab shows the condition rows,
the generated Datalog preview and the saved-query list; the run capture shows the
result table; the Datalog capture shows the raw editor tab. The Ask Notes capture
shows the answered question, the two cited source rows and the read-aloud
control; the Templates capture shows the target field, the variables editor and
the three template cards with their placeholder chips; the Flashcards captures
show the first question and, in the revealed capture, the answer with the four
SM-2 rating buttons. The Reminders captures pin the `RemindersPanel` surface: the
grouped overdue/today/upcoming rows with their snooze/complete/open/change chips,
the empty/failure states and the due dialog with its quick choices; the toast
capture shows the due-reminder toaster with its localized open action on the
journal route. The Plugins captures pin the `PluginsPanel` surface: the installed
tab with the two fixture plugin cards, status chips and smoke-test rows, the
community directory with its offline/fetched-at/verified-sha256 captions, and the
theme hub with its catalog entries. The first
argument or `OUTPUT` overrides the journal edit-mode path and every other capture
derives from its directory. Requirements are the same as the shell capture.

Deliberate deviations: the page header reuses the shared `VoiceCapture` control
instead of the React `DictationPanel` option toggles (diarize/summarize/identify),
the attachment card is rendered by the editor view rather than duplicated in the
page panel, and a backlink row opens its bounded snippet through an explicit
`backlinks.backlinked_snippet` control instead of the React Ctrl+hover timer
(hover timers are unreliable in a compositor-driven window). The plugins status
chip reuses `plugins.installed`, `plugins.disable` and `plugins.failed_on`
because no catalog carries a `plugins.ready` or `plugins.error` key (the shipped
React chip renders literal `Ready`/`Disabled`/`Error`), and the reminder row
actions use `reminders.panel.snooze_10min`, `reminders.panel.snooze_1h`,
`reminders.panel.complete` and `reminders.panel.open` instead of the shipped
literal `10m`/`1h`/`Done`/`Open` labels; no new catalog keys were added. The
reminder toaster polls on a 30 s timer gated per frame rather than the React
interval, and install-from-file uses the platform path picker instead of the
shipped file-input element. The routed page, journal, search, query, Ask Notes,
Templates, Flashcards, Reminders and Plugins panels own their header; the shell
chrome header is only rendered for the panels that are still placeholders,
matching the shipped app where the route fills the whole content column.

---

## CI

`.github/workflows/gpui-ci.yml` runs the GPUI checks on changes to the two
crates, the capture scripts or the CI wiring:

1. `gpui-shell` — formatting, build, tests and clippy on **Linux, Windows and
   macOS** (fail-fast disabled, one job per OS).
2. `gpui-capture` — Linux only: shell, journal, page, search, query, Ask Notes,
   Templates, Flashcards, Reminders, Plugins, Graph and editor-perf captures, plus
   artifact upload.

Both jobs install the Tauri Linux dependencies through
`.github/actions/install-linux-deps`, which now also carries the native shell
headers (Wayland, xkbcommon, fontconfig, freetype, Vulkan/EGL/X11). They run
`npm ci` first because `stratum-app` embeds the Mermaid bundle from
`node_modules`; without it the GPUI build fails on a clean checkout.

The main `ci.yml` keeps the GPUI crates in `cargo test --workspace` (the
workspace gate must cover every member) and publishes the shell capture through
its `gpui-capture` job.

---

## Current Scope and Limitations

The native shell reproduces the application chrome and implements the editor,
journal and page surfaces. The journal view covers the shipped `JournalPanel`
behaviour: localized day labels, previous/next-day navigation, the month
calendar with journal/today markers, a lazily loaded stack of past days, read
mode and the voice-capture flow. Stacked day editors use the editor's
auto-height mode, so each section contributes its full content height to the
journal scroll container; journal editors omit the page-level attachment panel,
matching `JournalPanel`.

The page view covers the shipped `PageView` behaviour: title/path header, the
shared voice-capture control, attachment picker, read/edit mode, the `Note` menu
(reindex, normalize, move/rename with path validation, delete with confirmation),
backlinks with linked/unlinked partitions and bounded snippets, related-note
suggestions with add-link, and highlight-on-open. Deleting returns to the journal
home; moving opens the new path with the localized success toast.

The search view covers the shipped `SearchPanel` behaviour: query
classification (full-text, tag or block-reference), full-text and tag result
lists with score labels and match counts, tag-mode query normalisation, empty
and failure states, reindex and navigation into a result. The query view covers
the shipped `QueryPanel` behaviour: a builder tab with field/operator/value
conditions, combiners and row ordering that emits the Datalog program, a raw
Datalog tab, run/reset, saved-query list with save/rename/delete/copy, build
errors and the result table. Both panels route their classification, score
formatting, builder emission/parsing and saved-query display through
`stratum-app` helpers rather than duplicating logic in the view. The Ask Notes
view covers the shipped `AskNotesPanel`: question submission (button and Enter),
clear, grounded answers with cited sources, the no-sources caption, read-aloud
through the shared speak path and provider/read-aloud error states. The Templates
view covers the shipped `TemplatesPanel` plus variable chips and the starter
installer: target-page input, blank-key filtering, template apply with
created/error toasts and list refresh. The Flashcards view covers the shipped
`FlashcardsPanel`: the reveal/rate/restart session machine, SM-2 ratings backed
by the shared `stratum-app` table, source/ease/interval captions, empty and
completion states. The Reminders view covers the shipped `RemindersPanel`:
overdue/today/upcoming grouping, humanized due captions, snooze/complete/open
row actions, the set/change/clear dialog with quick choices, and the shell
due-reminder toast with its open action. The Plugins view covers the shipped
`PluginsPanel`: installed plugin cards with enable/disable/reload/uninstall and
the note-read/HTTP smoke tests, the community directory with offline/sha256
captions and catalog installs, and the theme hub with install/apply/uninstall
and the built-in fallback. The routed
search, query, Ask Notes, Templates, Flashcards, Reminders and Plugins panels own
their header, like the page and journal panels.
The graph view covers the shipped `GraphPanel` surface listed above; the
remaining panel bodies (kanban, whiteboards and settings) are the next layer;
the route/navigation/i18n contracts are already in place for them.

The toaster dismisses manually or through `Escape`; there is no timer-based
auto-dismiss yet. This is intentional for the E2 shell and is documented in
`crates/stratum-ui/src/shell/mod.rs`.

The shell is desktop-only. Mobile remains the React/Tauri application.

---

## Adding a Panel or Locale

1. Add the panel to `Panel::ALL`, `ROUTES` and (if it appears in the sidebar)
   `NAV_ITEMS` in `crates/stratum-ui/src/router.rs`.
2. Add its title key to `Panel::title_key` and its icon to the icon table in
   `crates/stratum-ui/src/panels/mod.rs`.
3. Add any new user-facing strings to `SHELL_I18N_KEYS` in
   `crates/stratum-ui/src/shell/mod.rs`; the locale-parity test will fail until
   every catalog has the key.
4. For a new locale, copy the shipped catalog to
   `crates/stratum-i18n/locales/`, extend `Locale`, `LOCALES` and
   `Locale::ALL`, and update the parity tests.
