# Findings: top-of-app error/toast state lifecycle and persistence bug

Task: t_642ac0fd (trace of error/toast state, feed to t_245b6981 / t_c1c5f531 / t_1556175c)
Branch: wt/t_642ac0fd
Date: 2026-09-22

## 1. The surface: `appStore.error`

The only "top-of-app error/toast" state is:

- State shape: `AppState.error: string | null` — a bare string, no epoch/id/severity/timer fields (src/stores/appStore.ts:17)
- Initial value: `null` (src/stores/appStore.ts:34)
- Store created via `create<AppState>` (zustand), no persistence middleware, no reset action, no selector helpers (src/stores/appStore.ts:29)

There is a SEPARATE, unused "ErrorAlert" component (src/components/ui/ErrorAlert.tsx) exported from the ui barrel (src/components/ui/index.ts:2). It is tested (src/components/ui/ErrorAlert.test.tsx) but is NOT imported anywhere in the app — both top-of-app surfaces render a raw MUI `<Alert severity="error">` directly (see §2). The component is dead code.

## 2. All surfaces that render `appStore.error`

1. Desktop root — src/App.tsx:162-164: `{error && <Alert severity="error" sx={{ borderRadius: 0, flexShrink: 0 }}>{error}</Alert>}` — no `onClose`, inline Alert
2. Mobile root — src/components/MobileLayout.tsx:43-47: a raw `<div>` banner at top:48 with hard-coded error colors. No close control. Layout shifts content top to `error ? 88 : 48` (MobileLayout.tsx:49)
3. Vault picker (pre-vault landing) — src/components/VaultPicker.tsx:30-34: inline `Alert severity="error"`, no `onClose`. Only reachable when no vault is set.

The GraphPanel error (`GraphCanvas.tsx:273`, `GraphCanvas2D.tsx:95-98`, `GraphPanel.mobile.tsx:181-184`) is LOCAL component state (`useState` in useGraphPanel.ts:86), cleared on every `loadData()` (useGraphPanel.ts:140). It is NOT the `appStore.error` surface and has its own auto-clear via reload. Not part of this bug.

The `settingsStore.error` (settingsStore.ts:65,159,181,188,192) is a separate per-store error and is not rendered into any app surface (no consumer subscribes — settings UI uses its own `msg` state). Not part of this bug.

## 3. Every place that SETS or CLEARS `appStore.error`

### Writes (set `error` to a non-null message)

1. src/stores/appStore.ts:43 — `loadVault` catch
2. src/stores/appStore.ts:55 — `loadPages` catch
3. src/stores/appStore.ts:67 — `openPage` catch
4. src/stores/appStore.ts:78 — `createPage` catch
5. src/stores/appStore.ts:88 — `deletePage` catch
6. src/stores/appStore.ts:99 — `pickVaultDirectory` catch
7. src/components/PageView/shared.tsx:63 — SUCCESS informational message ("Reindexed page — N block(s)")
8. src/components/PageView/shared.tsx:66 — reindex partial failure message
9. src/components/PageView/shared.tsx:70 — reindex catch (`String(e)`)
10. src/components/PageView/desktop.tsx:32 — normalize-file catch (`String(e)`)

Note: many of the `String(e)` calls set the error again even while an earlier error is still on screen (each new error overwrites state). Errors also typically don't clear `loading` before setting `error` (e.g. the `finally` in loadVault sets `loading:false` after; loadPages/openPage do clear in finally).

### Clears (set `error` to `null`)

1. src/stores/appStore.ts:39 — `loadVault` start
2. src/stores/appStore.ts:94 — `pickVaultDirectory` start
3. src/components/PageView/shared.tsx:64 — a 2000 ms `setTimeout` after the SUCCESS reindex message ONLY

There is NO other timer, NO dismissal path, NO navigation-triggered clear, NO retry-triggered clear, NO app-lifecycle clear.

### Critical observations

- The only auto-dismiss is the 2 s `setTimeout` in shared.tsx:64, and it is attached to the SUCCESS message only; the FAILURE branch (shared.tsx:66) and the catch (shared.tsx:70) set the error with no timer.
- `loadPages` (appStore.ts:49-59) does NOT clear `error` on start, so re-fetching pages never clears a stale banner. The only way a stale error clears during normal use is `loadVault` or `pickVaultDirectory` re-invocation — neither happens after initial app start.
- Navigation: opening a page clears nothing. Route changes via Sidebar/PageTree never touch `error`.
- Retries: most panels use `useAsyncData` (lib/hooks/useAsyncData.ts), which holds errors in LOCAL state (cleared on each execute, line 22), so panel-local retry works — but `appStore.error` is set by appStore actions (loadVault/loadPages/openPage/create/delete) and by PageView handlers, and these are NOT cleared by any subsequent successful action.
- Sync conflicts: the conflict flow uses a separate `syncModalStore` + `ConflictModal`; the poller in App.tsx:65-94 silently ignores poll errors and triggers a modal — the sync-conflict surface does NOT write `appStore.error`. So "persistent errors like sync conflict" are already handled via a modal, NOT via `appStore.error`. If the new design introduces a "persistent" flag, sync conflict is the existing exemplar handled elsewhere.

## 4. The exact path that leaves an error visible until restart

The bug is a class of paths, all sharing one mechanism — appStore errors are only ever cleared by (a) the 2 s reindex-success timer or (b) a subsequent `loadVault`/`pickVaultDirectory` that almost never runs again:

- PAGE-DELETE / PAGE-CREATE / OPEN-PAGE failure (appStore.ts:78,88,67) sets `error`, and since `loadPages` on the next successful operation doesn't clear it (appStore.ts:49-59 has no `error:null` on start), the banner stays until restart.
- REINDEX FAILURE or normalize failure (PageView/shared.tsx:66,70; PageView/desktop.tsx:32) sets `error` with no timer — persists until restart.
- A transient runtime error from ANY store action while the app is running lands in `error` and stays; nothing clears on retry, navigation, or later store writes (later writes only overwrite with a new message).

Reproduction summary: trigger any failing `openPage`/`createPage`/`deletePage`/reindex/normalize while a vault is active. The banner renders (App.tsx:162 / MobileLayout.tsx:43) with no close button. No timer fires. The only exit is `loadVault`/`pickVaultDirectory` (not re-run after startup) or full app restart. Hence "stays until the app is restarted."

## 5. Missing dismissal/clear mechanism (gap list)

1. No `dismissError`/`clearError` action on `appStore` (no action at all mutates `error` from UI).
2. `ErrorAlert.tsx` supports `onClose` but is unused — the real surfaces use inline `<Alert>` with no `onClose`.
3. No auto-dismiss timer for transient/action errors (only the 2 s success message timer).
4. No distinction between transient and persistent errors (shape is a bare string; no severity/flags; sync conflict is handled in a separate modal store, so nothing in `appStore.error` is meant to be persistent).
5. No navigation-based clear (nav never resets error) and no "clear on next successful action" semantics.

## 6. Complete inventory of `appStore.error` touch points (for the implementer)

WRITE (10): appStore.ts:43,55,67,78,88,99; PageView/shared.tsx:63,66,70; PageView/desktop.tsx:32
CLEAR  (3): appStore.ts:39,94; PageView/shared.tsx:64
RENDER (3): App.tsx:162-164; MobileLayout.tsx:43-47; VaultPicker.tsx:30-34
DEAD CODE: ErrorAlert.tsx + index.ts:2 export (no importer anywhere)

## 7. Recommended state-transition design (for t_245b6981)

Replace the bare-string `error` with a structured notification and enforce a single write/clear funnel:

1. Change state shape to e.g. `notifications: { id: string; message: string; kind: 'error' | 'info' | 'persistent' } | null` (single-slot for now, matching current single-surface UI), or keep `error` as the transient slot and add `persistentError` separately.
2. Add store actions: `showError(msg, opts?: { persistent?: boolean })` and `dismissError(id?)`. All 10 write sites and all surfaces must go through these actions — no stray `useStore.setState({error})` (currently PageView/* bypass the store actions entirely).
3. Auto-dismiss: `showError` for transient entries schedules a timeout (suggest 6 s, matching the task AC) that calls `dismissError`; clear the timer on manual dismiss and on unmount of the surface, and invalidate stale timers when a new notification replaces the old one (compare ids/epochs, not messages).
4. Persistent: `{ persistent: true }` entries get NO timer; the render surface must always show a close button (MUI `Alert onClose`, so `App.tsx:162` and `MobileLayout.tsx:43` adopt the already-existing `ErrorAlert` component — which already supports `onClose` and is tested).
5. Do NOT clear persistent errors on unrelated state updates: since persistent conflicts already live in `syncModalStore`/`ConflictModal`, gate the persistent-only entries to that store (or map them into the new shape) rather than mixing into the auto-dismissing `appStore.error`.
6. Clear-on-success for retry: the action catch blocks in appStore that set `error` should be balanced by clearing the transient error when the corresponding action succeeds (e.g. `loadPages`, `openPage` success clears its own prior transient error), so a successful retry removes the banner.
7. Keep `ErrorAlert`/surfaces consistent with the existing test file (ErrorAlert.test.tsx already asserts the dismiss button and `role="alert"` — extending the store behaviour is the regression surface for t_c1c5f531).

Files that must change for the fix: src/stores/appStore.ts, src/App.tsx, src/components/MobileLayout.tsx, src/components/VaultPicker.tsx, src/components/PageView/shared.tsx, src/components/PageView/desktop.tsx; optionally wire the existing src/components/ui/ErrorAlert.tsx surface. Regression tests belong in src/stores/appStore tests + existing ErrorAlert.test.tsx (qa-engineer t_c1c5f531).
