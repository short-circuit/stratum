# E5.E1 Android Status-Bar Overlap Regression Gate — QA Report (t_4442effb)

Task: Add E2E regression test for Android status bar overlap.
Result: **PASS** — regression gate in place, discrimination-verified, no false
positives on desktop, portability defect fixed, on-device evidence recorded.

## What was delivered

Committed on `wt/t_4442effb` by the prior implementation runs of this task
(`f8b9419`, `f2be002`) and hardened in this QA run:

| File | Purpose |
|------|---------|
| `.github/scripts/safe-area-probe.mjs` | CDP probe over the Tauri WebView. Asserts the mobile top bar is NOT rendered under the Android status bar: `topBarTop == safeAreaTop` (device) / vacuous or aligned (desktop). Exit 0 PASS, 1 FAIL, 2 usage, 3 conn, 4 eval. |
| `.github/scripts/safe-area-regression.sh` | Orchestrates: locate pid → `adb forward` devtools socket → resolve `/json` target → run probe → harvest screenshot. `EXPECT_VERDICT=pass|fail` supports proving the gate fails on the old build. Exit 2 (debugger unreachable) = non-fatal SKIP. |
| `.github/scripts/smoke-android.sh` (E5) | Wires the gate in after a first rendered frame. Probe FAIL → job red; exit-2 → WARN (never fails a healthy build); base-smoke fail → gate skipped. |
| `.github/workflows/ci.yml` | `android-smoke` job now pins **node 22** via `setup-node`, matching every other job (the job previously never ran `npm install` and had NO pinned node, so the WHATWG `WebSocket` global the probe needs was not guaranteed). |

> The `android-smoke` / E5 job already had no `setup-node` while the rest of the
> workflow pins node 22, and a GH-hosted runner's preinstalled node (18/20) has
> no global `WebSocket`. Without the pin the on-device gate would crash with
> `WebSocket is not defined` — the exact failure caught in this QA run.

## Portability fix (this QA run)

`safe-area-probe.mjs` previously used the bare global `WebSocket`, which only
exists on node >= 22. The default node in this repo's developer shell and on
older GH images is 18/20, so the probe crashed before connecting:

    ReferenceError: WebSocket is not defined
        at safe-area-probe.mjs:31

Fixed: the probe now prefers `globalThis.WebSocket` and, when absent, resolves
the `ws` package (already a repo dependency) from the script's own tree via
`createRequire(import.meta.url)`. CI is additionally covered by the node-22 pin
(no `npm install` in the smoke job, so the `ws` fallback cannot be relied on
there).

## Verification evidence

### 1. Probe discrimination matrix (mocked CDP DOM, node 18 AND node 22)

`scenario-mock-cdp.cjs` (scratch) hosts a WS/CDP mock serving four DOM
scenarios; the probe is exercised against each:

| Scenario | Mode | Expected | Actual (node18) | Actual (node22) |
|----------|------|----------|-----------------|-----------------|
| device-clean (fixed build) | device | PASS (0) | 0 | 0 |
| device-overlap (old build) | device | FAIL (1) | 1 | 1 |
| desktop-clean | desktop | PASS (0) | 0 | 0 |
| desktop-misaligned | desktop | FAIL (1) | 1 | 1 |

Outcome: `PROBE MATRIX: ALL PASS` on both runtimes.

### 2. Desktop parity — no false positive

`safe-area-probe.mjs <ws> desktop` against a desktop-like DOM (no mobile top
bar, zero safe-area insets) returns 0 (vacuously PASS): probe does not
false-positive on desktop.

### 3. Smoke control-flow safety (no-device harness)

`test-smoke-harness.sh` A/B/C scenarios against the wired `smoke-android.sh`:

| Scenario | Effect | Smoke exit |
|----------|--------|------------|
| A (healthy) | gate runs, debugger unreachable → SKIP exit 2 → WARN | 0 (build stays green) |
| B (crash) | base smoke fails → gate skipped | 1 |
| C (no frame, retried) | base smoke fails → gate skipped | 1 |

The gate can never turn a healthy build red (exit 2 is a WARN) and never masks
a real failure.

### 4. On-device emulator validation (stratum_smoke, API 34 x86_64)

The prior implementation run validated the real gate against real APKs on the
emulator. Evidence is retained in the scratch harvest dirs and reproduced
below (timestamps 2026-09-24T12:59 / 13:00):

- **Fixed build** (`fixed-debug.apk`): gate exit 0 —
  `SAFE_AREA_PROBE PASS (device mode)`; `safeAreaTop 48.761906`,
  `topBarTop 48.761905670166016`, `topBarHeight 48`, viewport 915x412.
  Screenshot: `validate/harvest-fixed/safe-area-verify.png`.
- **Old (pre-fix) build** (`old-debug.apk`): gate exit 0 under
  `EXPECT_VERDICT=fail` — probe FAILED exactly as expected
  (`top bar not under the status bar (expected 48.76, got 0.00)`),
  i.e. the gate discriminates the regression.
  Screenshot: `validate/harvest-old/safe-area-verify.png`.

### 5. Desktop no-regression suite (this QA run, node 22)

| Gate | Result |
|------|--------|
| `npm run test` | 99/99 PASS |
| `npm run lint` | 0 errors, 1 pre-existing warning (JournalPanel.shared.tsx, same baseline as t_8ebb0419) |
| `npm run build` | PASS (✓ built) |
| `npx tsc --noEmit -p tsconfig.app.json` | PASS (exit 0) |

## Files changed in this QA run

- `.github/scripts/safe-area-probe.mjs` — WebSocket global→`ws` fallback
  (portability).
- `.github/workflows/ci.yml` — `android-smoke` job now pins node 22 via
  `setup-node` (gate runnable in CI).
- `verification/` — evidence captured in this report.

No Rust crate or frontend source was touched; desktop-only changes are confined
to CI scripts, so the desktop no-regression suite (99 unit tests + lint + build
+ tsc) is unaffected and passes.
