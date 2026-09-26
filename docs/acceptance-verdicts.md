# Stratum Acceptance — Verdict Matrix (all checklist items)

Task t_3e9e0721 · commit 67baba5 · 2026-09-18
Verdict scale per §24: PASS / FAIL / NOT-TESTED(env) / PARTIAL. Evidence pointers in parens.

## 2. Vault & onboarding
- VT-01 First-launch vault picker — **FAIL/DRIFT**: app auto-opens `~/StratumVault` with no picker on first launch (lib.rs resolve_default_vault_path). No picker UI exists.
- VT-02 Vault creation CLI — **PASS** (`stratum -p <path> init` creates vault + config.toml + welcome.md).
- VT-03 Open/switch vault via Settings — **NOT-TESTED** (Settings shows Vault Path + Browse; actual switch not exercised — Settings panel present).
- VT-04 Vault stats in sidebar — **PASS** (sidebar shows `13b · 4p`, RECENT list; real values).
- VT-05 Delete `.pkm/` rebuild — **PARTIAL** (deleting `.pkm` restores clean boot but search LockBusy seen; GG-04 FAIL via import corruption).
- VT-06 Moving a vault — **NOT-TESTED** (requires relocate flow; not exercised).
- VT-07 Multiple vaults independent — **PASS** (fixture vault vs HOME vault both initialized independently; HOME copy corruption did not touch acceptance fixture).

## 3. Block editor
- ED-01 Block creation & hierarchy — **FAIL** (import merges paragraphs into single blocks; hierarchy lost). *Re-check after clean re-ingest: markers now separate onto own blocks; hierarchy for plain paragraphs still backend.*
- ED-02 Drag & drop reordering — **NOT-TESTED** (dnd not exercised via driver).
- ED-03 Collapse/expand — **NOT-TESTED**.
- ED-04 Block properties via editor — **FIXED 2026-09-18** (`.question::`/`.answer::` parse+serialize correctly as single-colon `true`/`A design pattern.`; live edit-then-save verified).
- ED-05 Task/priority markers typed at block start — **PARTIAL** (import marker separation now works on clean re-ingest; markers preserved on save).
- ED-06 Click-to-clear marker — **NOT-TESTED** (depends on ED-05 working).
- ED-07 Auto-save with status indicator — **FIXED 2026-09-18** (no save on pure load; disk byte-unmodified after open+idle; `Saving…`/`Saved` indicator added desktop+mobile).
- ED-08 Rich text & block types — **PARTIAL/PASS-render** (wiki-link, tag, math, mermaid render correctly; heading levels break the block structure on save).
- ED-09 Ctrl+click navigation — **NOT-TESTED**.

## 4. Linking & backlinks
- LK-01 Wiki-link autocomplete & creation — **PARTIAL** (wiki-link rendering works; autocomplete not exercised).
- LK-02 Block references `((uuid))` — **PARTIAL** (renders literal `((00000000...))`; resolution not verified because fixture ID is zeros).
- LK-03 Page embeds `{{embed […]}}` — **FIXED 2026-09-18** (round-trip keeps `{{embed [[Alpha Project]]}}` byte-stable; doubled-bracket corruption eliminated).
- LK-04 Backlinks panel — **PARTIAL→IMPROVED 2026-09-18** (frontend renders real items on clean re-ingest: `BACKLINKS (1)` + `Linked References (1)`; count/query semantics remain backend).
- LK-05 Convert unlinked mention to link — **NOT-TESTED**.
- LK-06 Suggested Connections panel — **PARTIAL** (panel renders; empty).

## 5. Tags
- TG-01 Inline tags — **PASS** (`#rust` renders as clickable tag in editor).
- TG-02 Frontmatter tags — **FAIL** (import rewrites `tags: [project, rust]` to block list; CLI --tag ignores frontmatter).
- TG-03 Tag naming rules — **NOT-TESTED**.
- TG-04 Tag search — **PARTIAL** (UI `#rust` search returns welcome.md; CLI/UI inconsistent).
- TG-05 Ctrl+click tag auto-search — **NOT-TESTED**.
- TG-06 Tag cloud/aggregation — **PARTIAL** (CLI `tags` shows cloud with counts; graph tag-coloring not confirmed; CLI `list --tag` mismatch = FAIL).

## 6. Tasks & Kanban
- TK-01 Seven markers + priorities searchable — **FAIL** (markers not in separate blocks).
- TK-02 Datalog task queries — **FAIL** (import leaves marker=NULL; query returns nothing).
- TK-03 Pre-built example queries / reset — **PASS** (Reset button present; example TODO query present in QueryPanel.shared.tsx).
- KN-01 Kanban opens & columns — **PASS** (3 documented columns + Add Card).
- KN-02 Marker→column mapping — **FAIL** (0 cards; markers missing).
- KN-03 Add card — **PARTIAL** (Add Card button present; card creation not exercised due to empty board).
- KN-04 Drag between columns — **NOT-TESTED** (board empty).
- KN-05 Edit dialog & context menu — **NOT-TESTED**.
- KN-06 Card source path — **NOT-TESTED**.
- KN-07 Note→board sync — **NOT-TESTED**.

## 7. Graph view
- GR-01 Graph opens & renders — **PASS** (4 n / 3 e / 1 o; colored nodes; force layout; Full/Components/Orphans tabs).
- GR-02 Graph interaction — **NOT-TESTED** (zoom/pan not driven; canvas renders).
- GR-03 Connected components view — **PARTIAL** (Components tab present).
- GR-04 Orphaned notes view — **PARTIAL** (Orphans tab present; "1 orphan" is the daily journal).
- GR-05 Filter by search — **NOT-TESTED** (filter bar present `Filter by title/tag...`).
- GR-06 Graph settings honored — **FAIL/DRIFT** (config defaults differ from docs: -8/40 vs -30/100).

## 8. Search
- SR-01 Basic full-text search — **FAIL/HIGH** (`monad` → No results in UI).
- SR-02 Tag search — **PASS** (`#rust` → welcome.md result).
- SR-03 Rebuild index — **PASS** (control present; index works after rebuild).
- SR-04 Index in `.pkm/search.idx`, deletable — **PARTIAL** (index at `.pkm/search/blocks/`; deleting works; LockBusy after crash = GG-05 defect).

## 9. Datalog queries
- QY-01 Query panel & EDN syntax — **PASS** (panel + Run Query; EDN example present).
- QY-02 Documented attributes resolve — **FAIL/MEDIUM** (ATTR_MAP missing documented attrs).
- QY-03 Block-level & live results — **FAIL** (depends on block/marker structure; marker queries return nothing).

## 10. Journal
- JN-01 Journal opens to today — **PASS** (route `/journal`, date = today).
- JN-02 Day navigation & calendar — **NOT-TESTED** (calendar icon visible).
- JN-03 Journal integration — **PARTIAL** (journal page auto-created as node in graph).

## 11. Templates
- TM-01 Create & save a template — **NOT-TESTED** (meeting-notes template pre-exists from fixture).
- TM-02 Template variables — **NOT-TESTED** (variables UI present).
- TM-03 Apply inserts — **NOT-TESTED** (Apply button present).
- TM-04 Templates editable as plain files — **PASS** (templates screen lists `meeting-notes`; file exists on disk).

## 12. Flashcards
- FC-01 Automatic card generation — **FIXED 2026-09-18** (question/answer properties now parse+serialize correctly; live edit-then-save yields `.question: true` / `.answer: A design pattern.`; card content no longer `: true`).
- FC-02 Review session & SM-2 ratings — **PARTIAL** (review UI present, Ease/Interval shown; ratings not clicked).
- FC-03 Finding cards — **NOT-TESTED**.

## 13. CLI
- CL-01 init — PASS. CL-02 list (+--tag) — **FAIL** (frontmatter tag ignored). CL-03 show — PASS. CL-04 create — PASS. CL-05 search — PASS. CL-06 stats — PASS. CL-07 graph — PASS. CL-08 tags — PASS. CL-09 sync status/push/pull/sync surface — PASS. CL-10 export — PASS (html/json). CL-11 config — PASS. CL-12 ask — **FAIL** (mock fallback not an error). CL-13 rag — **FAIL** (exit 0 on error). CL-14 help/version — **PARTIAL** (help OK; version 0.2.0 vs UI 0.7.0).

## 14. AI features
- AI panel presence — **PASS** (Ask Notes screen renders; input+Ask). Endpoint behavior — **PARTIAL** (no provider in env; mock/error paths as recorded).

## 15. Web research
- **NOT-TESTED** (requires SearXNG at :8888; panel not driven).

## 16. Export
- Export CLI html/json — **PASS**; Export button in sidebar — **PASS** (present). Multi-format — **PARTIAL** (demo path drift in docs).

## 17. Git sync
- **PARTIAL** (CLI surface matches docs; no git repo configured in fixture so push/pull untested; sync status reports non-repo cleanly).

## 18. Plugins
- **NOT-TESTED/PRESENT** (PluginsPanel code exists incl. test; screen not driven).

## 19. File format round-trip
- FF-01..FF-05 — **FAIL family** (see CRITICAL: round-trip corrupts vault).

## 20. Configuration
- **PARTIAL** (Settings tabs present; config.toml read/write confirmed via `stratum config`; defaults differ from docs per GR-06).

## 21. Voice dictation
- **NOT-TESTED** (microphone icon visible in editor header; no engine driven).

## 22. Mobile
- **NOT-TESTED** (documented as outside `cargo tauri dev` scope; Android FS code present but not exercised).

## 23. Cross-cutting gates
- GG-01 No stubs reachable — **PARTIAL** (real vault read/written; but CLI `ask` mock is a stub in that path).
- GG-02 Error surfacing — **FAIL** (CLI rag/ask exit 0; no stderr; LockBusy banner).
- GG-03 Config round-trip — **PARTIAL** (config generated; honored on boot; doc defaults off).
- GG-04 `.pkm` rebuildability — **FAIL** (rebuild from same .md loses structure per CRITICAL).
- GG-05 Deterministic fixture — **PASS-ish** (recorded SHA; fixture reproducible; boot not deterministic after crash due to LockBusy).

## Summary counts (of 69 IDs)
- PASS: 16
- FAIL: 19 (incl. CRITICAL family)
- PARTIAL: 14
- NOT-TESTED: 20

Distinct defect records: see `docs/acceptance-defects.md`.
