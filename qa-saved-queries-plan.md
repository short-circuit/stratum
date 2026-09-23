# QA Test Plan — Saved Queries (Kanban t_7b3040e6)

## Scope

Verify the Datalog saved-queries feature end-to-end: backend persistence, the
Tauri IPC command layer, and the QueryPanel UI (desktop + mobile) flows:

- Save current query under a name
- List saved queries (with count)
- Load a saved query into the editor
- Rename a saved query
- Delete a saved query (with confirmation)
- Error display (load failure, action failure) and dismissal
- Persistence: saved queries survive a restart / navigation round-trip
- Cross-platform parity: desktop and mobile variants

## Coverage inventory (pre-existing, from parent features)

| Layer | Location | Status |
|-------|----------|--------|
| Backend unit (13) | crates/pkm-query/src/saved_queries.rs `#[cfg(test)]` | Passing |
| Backend integration (9) | src-tauri/tests/saved_queries_commands.rs | Passing |
| Frontend hook (7) | src/components/QueryPanel/QueryPanel.shared.test.ts | Passing |
| Frontend desktop UI (7) | src/components/QueryPanel/QueryPanel.desktop.test.tsx | Passing |

## Gaps identified & closed by this QA pass

| Gap | Fix | Test file |
|-----|-----|-----------|
| Mobile variant has ZERO test coverage | Added 9 mobile UI tests (list/load/save/rename/delete/empty/disabled + errors) | src/components/QueryPanel/QueryPanel.mobile.test.tsx (new) |
| Component-level error display untested (hook-only coverage) | Added load-error + action-error render/dismiss tests to both variants (4 tests total) | QueryPanel.desktop.test.tsx, QueryPanel.mobile.test.tsx |
| E2E mock lacked saved-query commands | Added stateful `list/save/rename/delete_saved_query` handlers with localStorage persistence | e2e/mocks/tauri.ts |
| No UI-level E2E for the saved-query feature | Added 6 E2E scenarios through the real UI | e2e/specs/saved-queries.spec.ts (new) |

## Verification gates

1. `npx vitest run src/components/QueryPanel` — unit/component tests
2. `npm test` — full frontend suite
3. `npx tsc --noEmit -p tsconfig.json` — type check (incl. e2e mocks/specs)
4. `npm run build` — production build
5. `npm run lint` — ESLint
6. `cargo test -p pkm-query` — backend unit tests
7. `cargo test --test saved_queries_commands` — backend integration (run if crates compile)
8. Playwright E2E for saved-queries spec

## Findings & behavior notes

- **Behavior documented by test**: on a failed delete, the confirm dialog stays
  open while the warning alert appears (delete target cleared only on success).
  This is intentional (allows retry) and is now pinned by tests on both desktop
  and mobile.
- **E2E mock caveat**: `commandErrors` config applies to any command including
  the saved-query ones; the save/rename/delete handlers throw on invalid input
  (empty name, not-found, name clash) to mirror backend `PkmError` behavior.
