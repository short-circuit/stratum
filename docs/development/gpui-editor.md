# Headless Block-Editor Engine

`crates/stratum-editor` (lib `stratum_editor`) is the headless editor core for
the native GPUI shell. It is deliberately UI-free: no gpui, tauri or web
dependencies. Every editing rule, inline-span parse, undo record, IME
transition, menu catalog, marker/reminder model, WYSIWYM decoration and
virtualization calculation lives here so the phase-B view can be a thin
renderer over `render::DecoratedRow` values.

The frozen React + Tauri application in `src/` and `src-tauri/` remains the
behavioural and visual reference. The crate ports the pure helpers from
`src/components/OutlinerEditor`, `src/components/AISlashMenu.tsx`,
`src/lib/wikiLinks.ts` and `src/lib/reminders/reminderUtils.ts`; it never
duplicates React state or rendering.

---

## Crate Layout

```
crates/stratum-editor/
├── src/
│   ├── lib.rs            # module map, re-exports, EDITOR_I18N_KEYS release gate
│   ├── document.rs       # EditorDocument, canonicalization, save bookkeeping
│   ├── dto.rs            # BlockDto/PageDoc, stratum_app-compatible seam
│   ├── inline.rs         # span parser + byte-exact serializer
│   ├── markers.rs        # marker detection, badge rows, reminder chips
│   ├── menus.rs          # slash catalog, marker menu, wiki autocomplete, link hits
│   ├── state/            # caret/selection, edits, undo, clipboard, IME
│   ├── render.rs         # WYSIWYM decoration model (no UI types)
│   └── virtualize.rs     # deterministic heights and viewport windows
└── tests/                # round-trip, scripted editing and i18n contract suites
```

Every module carries `//!` documentation, public items are documented, and the
non-obvious algorithms (depth-preserving serialization, history coalescing,
UTF-16 boundary rejection, paste normalization) have inline comments explaining
the invariants.

---

## Byte-Stability Contract

The editor writes the same plain Markdown block format as the shipped app.
`EditorDocument::body` and `canonicalize_body` serialize the block tree with the
shipped `pkm_markdown::block_parser::serialize_blocks` formatting rules (content,
`.marker`, `.priority`, `.heading-level`, sorted properties, `.id` last) and
preserve block depth: the serializer emits blocks in `BlockTree::depth_first_order`
order with each block's real tree depth as indentation. The editor never
re-implements serialization.

Sibling order is deterministic: `BlockTree` keeps a first-insertion order and
uses it as the tie-break when several siblings have no `left_id` (or when the
`left_id` chain is broken). An explicit chain always wins, so canonical files
serialize in their recorded order, while unordered input keeps the order the
store or parser handed to the editor. Serialization never depends on `HashMap`
iteration order.

- Canonicalization is idempotent: `canonicalize_body(canonicalize_body(x)) ==
  canonicalize_body(x)`.
- A canonical file round-trips byte-identically through `canonicalize_body`.
- Parse → serialize is **not** byte-identical for arbitrary non-canonical input:
  ids are injected and properties are reordered. Those are deliberate
  normalizations, not data loss.
- Saves re-assemble the original YAML frontmatter via
  `assemble_blocks_markdown`; the editor never rebuilds frontmatter from a title.

`tests/roundtrip.rs` pins the canonical fixtures (nested children, code fences,
Mermaid, math, wiki-links, tags, embeds, CJK) and the idempotency of the
non-canonical corpus.

---

## Inline Spans

`inline::parse_inline` recognizes emphasis, code, highlight, wiki-links and
embeds, Markdown links, tags, math and block references. The parser is
non-recursive: the first matching construct wins and its inner text is kept
verbatim, so nested markers degrade to plain text. Escapes are preserved as
text and unclosed markers degrade to text one scalar at a time.
`render_inline(parse_inline(s)) == s` is guaranteed for every accepted input.

`Span` records `start_byte`/`end_byte` so the view can map a caret or a
ctrl-hover to a span without re-parsing.

---

## Editing, Undo and IME

`state::EditorState` owns one document plus caret/selection, undo history,
clipboard payload and IME composition.

- Structural edits (`insert_text`, `insert_newline`, `backspace`,
  `delete_forward`, `indent`, `outdent`, `move_up`, `move_down`, `merge_block`,
  `set_marker`, `set_priority`, `apply_span`, `toggle_collapsed`) all go through
  `pkm_block::ops`. Caret byte offsets are clamped to UTF-8 char boundaries
  before `split_block`, which panics on a non-boundary.
- Undo is expressed as concrete forward/inverse `EditOp` pairs, never document
  snapshots. `History` keeps at most 200 entries, coalesces contiguous text
  insertions in the same block at contiguous offsets within 750 ms, clears the
  redo stack on a new edit, and applies the inverse on undo. Split/merge ops
  carry the child list so undo restores deleted subtrees.
- Clipboard: `copy_selection`/`cut_selection` build `ClipboardPayload` with
  plain text and `BlockDto`s. `normalize_paste` converts CRLF/CR to LF, strips
  NUL/control characters, expands tabs outside fenced code, strips trailing
  spaces, collapses 3+ blank lines, and caps input at 100 000 characters.
  `paste_markdown` splits paragraphs into blocks and maps leading list markers
  to indent depth.
- IME: `ime_replace` stores the marked UTF-16 range and updates content without
  pushing history; `ime_commit` pushes exactly one history entry;
  `ime_cancel` restores the pre-composition content. `utf16_to_byte` rejects
  offsets that land inside a surrogate pair.

Cross-block delete and multi-block paste are recorded as one reversible
`EditOp::SpliceBlocks` entry: it carries the full removed/inserted block
snapshots plus explicit parent/left (`LinkChange`) and content (`ContentChange`)
deltas, so a single undo restores every removed block and re-linked sibling and
redo re-applies the whole range operation. The wiki autocomplete popup lists only
the rows it actually renders (`wiki_row_count`, capped at 8), so arrow-key
selection and Enter can never reference an unrendered target; a no-match query
renders the single "create page" row and closes the `[[query` link on Enter.

---

## WYSIWYM Model and Virtualization

`render::render_page` classifies blocks (`Paragraph`, `Heading`, `Quote`,
`Code`, `Mermaid`, `Math`, `ListItem`, `Divider`) and produces `DecoratedRow`s
with depth, marker/priority badges, reminder chips, collapsed state and
decorated spans.

- `RenderMode::StageA` keeps every syntax delimiter visible (`syntax = true`).
- `RenderMode::FocusedLine` hides delimiters only for the focused block; every
  other row stays Stage A.
- A marker keyword at block start is never rendered as text: it is represented
  by the badge.

`virtualize::VirtualLayout` provides deterministic height estimates, offsets,
`visible_window`, `scroll_top_for_block`, `block_at_offset` and
`anchor_offset`. The 10k-row tests pin the window bound (viewport rows plus two
overscan windows), monotonic offsets and the inverse scroll mapping.

---

## Math and Mermaid Caches

`math::MathCache` and `mermaid::MermaidCache` are bounded LRU caches
(`with_capacity`, defaults 64 math images and 32 SVG diagrams). Math renders
through the pure-Rust RaTeX pipeline (`ratex-parser` → `ratex-layout` →
`ratex-render`) and Mermaid through `mermaid-rs-renderer`; neither cache grows
without bound. `render_page_with_assets` fills the `math` and
`mermaid` fields of `DecoratedRow`; failures leave those fields `None` so the
decorated text spans remain visible. The pure `render_page` path leaves both
fields empty and is what the contract tests and the row builder use.

---

## Native Editor View (Phase B)

The view lives in `crates/stratum-ui/src/panels/editor/` and is split so every
file stays under the repository limit:

```
crates/stratum-ui/src/panels/editor/
├── mod.rs       # EditorView state, metrics, save scheduling, asset budget
├── view.rs      # Render impl: badge strip, virtual list, canvas input, overlays
├── rows.rs      # row/span/caret geometry and elements (+ rows/tests.rs)
├── input.rs     # EntityInputHandler, key/mouse handling, clipboard, IME
├── overlay.rs   # slash, wiki, marker, link-preview and dead-link popups
├── context_menu.rs # right-click block menu (read aloud, clipboard, marker, reminder)
└── save.rs      # SaveState/SaveStatus transitions
```

`EditorView` owns the `EditorState`, the math/Mermaid caches, the
`VirtualLayout`, the `VirtualListScrollHandle`, save status, overlay state, the
focus handle and the `ShellData` provider. It delegates every editing rule to
`stratum_editor`; the view never re-parses block text.

### Rendering and virtualization

Rows are produced by `build_rows_for_state`, which calls
`stratum_editor::render_page` and therefore never re-implements decoration. The
view uses `v_virtual_list(...).track_scroll(...)` with the deterministic
`VirtualLayout` heights and materialises only the visible window plus overscan.
Math and Mermaid images are rendered lazily for the visible range with a budget
of one new asset per frame, so a window full of diagrams cannot blow the frame
work budget. Row geometry (text left, depth indent, chevron, badge and reminder
gutter) is deterministic and shared by the caret and mouse hit testing.

### Input, IME and clipboard

`input.rs` implements `EntityInputHandler` and registers it from the editor
canvas during paint (`window.handle_input`), which is the only valid time to
install an input handler in gpui. It maps UTF-16 platform ranges onto the
engine's UTF-8 byte offsets through `utf16_to_byte`/`byte_to_utf16` and forwards
marked ranges and commits to `EditorState::ime_replace`/`ime_commit`. Key
handling covers caret movement and selection, undo/redo, clipboard, marker
toggling and overlay navigation. Handled keys stop propagation; printable
characters are left to the platform so they are inserted before overlay
detection runs.

### Overlays and navigation

After a text insertion the view inspects the focused block for `/`, `[[` and `:`
tokens and opens the slash, wiki or marker overlay. The slash catalogue comes
from `stratum_editor::menus`; wiki targets resolve against the provider's recent
pages. Ctrl+hover opens a link preview; Ctrl+click on a tag or wiki-link sets a
pending navigation that the shell polls and turns into a route change.

Right-clicking a row places the caret and opens the per-block context menu
(`context_menu.rs`) anchored at the pointer. Its entries, in order, are read
aloud, copy, cut, toggle task marker, set reminder (one hour from now) and clear
reminder; labels resolve through `stratum_i18n` keys gated by
`CONTEXT_MENU_ITEMS`. Escape or a click outside the popup dismisses it. Selecting
an entry closes the menu and takes the normal dirty/debounced-save path; the
engine owns clipboard, marker and reminder mutations. Read-aloud goes through
`ShellData::speak` → `StratumService::tts_speak` → base64 decode → `rodio`
playback on a detached thread, and providers without audio support return
`Ok(())`.

### Save lifecycle

Edits mark the document dirty and schedule a debounced (400 ms) save with a
generation counter so a burst of keystrokes produces one write. The save chip
uses `editor.saving` and `settings.saved`; a failed write keeps the document
dirty and surfaces the provider error. Saves go through the same
`blocks_to_dto` seam as the service layer, so block fields stay identical.

### Phase-B boundary

The gpui view renders `DecoratedRow`s and forwards input events to
`EditorState`. It must not re-implement parsing, marker detection, undo,
clipboard normalization or height estimation; those stay in the headless crate
so they remain unit-tested and platform-independent. The `dto` module is the
seam to `stratum_app::block::BlockDto` and must stay field-compatible.

Phase B includes the per-block context menu and read-aloud through
`ShellData::speak`; block drag-and-drop is still out of phase B. Reminder
mutations go through `EditorState::set_block_reminder`, which routes the write
through `EditorDocument::mutate` so revision and dirty bookkeeping stay in the
document.

---

## 10k Performance Capture

`src/bin/editor_perf.rs` (binary `stratum-editor-perf`) opens the editor on the
deterministic 10 000-block fixture in a 1280×720 window, sweeps the virtual list
through the full content height for `STRATUM_PERF_FRAMES` frames (default 600),
records frame deltas and the editor's row-materialization durations, writes JSON
to `STRATUM_PERF_OUT` and quits.

`scripts/capture-gpui-editor-perf.sh` starts Xvfb when `DISPLAY` is unset, builds
the release binary, runs it, prints the numbers and exits non-zero only when
`render_p95_ms` exceeds the 55 fps work budget (18.18 ms). Frame pacing under
software rendering is reported but not gated. The gate deliberately measures the
release build: a debug build spends an order of magnitude more time building row
elements, so its numbers say nothing about the 55 fps target
(`STRATUM_PERF_PROFILE=debug` reproduces a debug run). Math/Mermaid asset
rendering is budgeted to one new asset per frame, cached afterwards and reported
separately as `asset_p95_ms`; it is excluded from the row-materialization figure
because a single diagram render can legitimately exceed one frame's budget.

Reference-run measurement (Xvfb, `LIBGL_ALWAYS_SOFTWARE=1`, release build,
600 frames, 10 000 blocks):

| Metric | Value |
|--------|-------|
| Blocks / rows | 10 000 / 10 000 |
| Rows materialised per frame | 2 |
| Row-build `render_p95_ms` | 2.44 ms (budget 18.18 ms) |
| Asset `asset_p95_ms` | 0.04 ms |
| Frame p50 / p95 / p99 / max | 48 / 48 / 48 / 64 ms |
| Derived frame fps p50 / p95 | 20.8 / 20.8 |

The frame deltas are dominated by the software swapchain pacing under Xvfb, not
by editor work; the gated measurement is the row-build p95, which is an order of
magnitude below the budget. The JSON report is written to
`.hermes/perf/editor-10k-frames.json` (gitignored).

---

## Translation Keys

The crate never carries English UI strings. Models expose translation keys plus
interpolation variables, and `EDITOR_I18N_KEYS` is the release gate list
mirroring `stratum_ui::SHELL_I18N_KEYS`. `tests/contract.rs` verifies every key
against `crates/stratum-i18n/locales/en.json` and checks that every slash-item
key is a member.

---

## Building and Testing

```bash
# Formatting
cargo fmt --all --check

# Clippy (warnings are errors)
cargo clippy -p stratum-editor --all-targets -- -D warnings

# Unit, integration and contract tests
cargo test -p stratum-editor

# View and window-free contract tests
cargo test -p stratum-ui

# Clippy for the view (warnings are errors)
cargo clippy -p stratum-editor -p stratum-ui --all-targets -- -D warnings

# 10k perf capture (Xvfb when DISPLAY is unset)
./scripts/capture-gpui-editor-perf.sh
```

The GPUI CI workflow includes `crates/stratum-editor/**` and
`scripts/capture-gpui-editor-perf.sh` in its path filters, runs the crate and
view build/test/clippy gates alongside `stratum-i18n`, and captures the 10k perf
report in the screenshot job.
