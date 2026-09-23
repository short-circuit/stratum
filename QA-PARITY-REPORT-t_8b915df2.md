# QA Report — Mobile/Desktop UI Parity: Settings + Journal (t_8b915df2)

**QA Engineer run:** 2026-09-23 15:40–15:50 (Europe/Zurich)
**Branch under test:** wt/t_8b915df2 (merge of `t_d4e0a629` Settings parity + `t_4c53166a` Journal parity)
**Head commit:** f696082 (merge of e54c8ed + a18122c)
**Result:** PASS — all parity items verified green across mobile-small, mobile-large, and desktop. No regressions in adjacent screens. No feature gaps remain (both implementation parents addressed the audit findings).

---

## 1. Scope

Cross-platform parity verification of **Settings** and **Journal** between mobile and
desktop, per the task contract:
- fields
- navigation
- data persistence
- error states
- visual consistency

Viewports tested (all < 768 width exercise the true mobile `useResponsive` rendering path,
with a mobile UA so the `getPlatform` gate agrees):
- **mobile-small**: 360×740
- **mobile-large**: 428×900
- **desktop**: 1280×720

## 2. Method

Automated Playwright parity specs (headless Chromium, Tauri `invoke` mocked to mirror the
real backend contract), plus full-page screenshots per viewport for visual verification.
The parity specs were authored to lock the behavior in as regression guards.

- `e2e/specs/qq-parity-settings-qa.spec.ts` — Settings parity (12 tests)
- `e2e/specs/qj-parity-journal-qa.spec.ts` — Journal parity (12 tests)
- `e2e/specs/qq-parity-shots.spec.ts` — screenshot capture (6 shots)

Screenshots: `test-results/qa-parity-shots/*.png` (6 images, visually inspected).

## 3. Results — per item (contract)

### Settings

| Item | mobile-small | mobile-large | desktop | Verdict |
|------|:---:|:---:|:---:|:---:|
| All 6 desktop-equivalent sections reachable (Vault/Theme/AI/Research/Developer/Sync) | PASS | PASS | PASS (6 tabs) | PASS |
| Vault fields (path, Browse) + persistent Save | PASS | PASS | PASS | PASS |
| AI provider accordion (combobox, API endpoint, Fetch models) | PASS | PASS | PASS | PASS |
| Settings save round-trips through the store (persistence across reload) | PASS | PASS | PASS | PASS |
| Save failure surfaces an error message (error state) | PASS | PASS | PASS | PASS |
| Visual consistency (screenshot inspection) | PASS | PASS | PASS | PASS |
| **Settings surface on load failure** | **PASS* (documented)** | **PASS* (**)** | **PASS* (**)** | parity-equal; not a parity gap |

\* Documented current behavior, not a parity gap: when `get_settings` fails on mount, the
page stays on "Loading settings..." and the load error is NOT surfaced to the user. This is
**identical across all three viewports** — i.e. it is parity-equal — so it is not a
mobile-vs-desktop discrepancy. Tracked in the spec's comments as a known enhancement.

### Journal

| Item | mobile-small | mobile-large | desktop | Verdict |
|------|:---:|:---:|:---:|:---:|
| Prev/Next day arrows present | PASS | PASS | PASS | PASS |
| Calendar affordance (mobile full-screen dialog / desktop anchored popover) | PASS | PASS | PASS | PASS |
| Prev-day arrow creates + navigates to target day | PASS | PASS | PASS | PASS |
| Persistence: navigate away and back still renders editor | PASS | PASS | PASS | PASS |
| Error state: ensure_today_journal failure surfaces Retry + Repair database | PASS | PASS | PASS | PASS |
| Visual consistency (screenshot inspection) | PASS | PASS | PASS | PASS |

## 4. Regression check (adjacent screens / no regressions)

Full e2e suite + unit suite + build + lint:

| Gate | Result |
|------|--------|
| Full Playwright e2e suite (`node_modules/.bin/playwright test --config e2e/playwright.config.ts`) | **57 passed** (all specs incl. app-bootstrap, navigation, settings, search, graph, dictation, plugins, parity, shots) |
| Unit tests (`npm run test`) | **99 passed** (18 files) |
| Production build (`npm run build`, tsc + vite from HEAD) | OK |
| Lint (`npm run lint`) | **0 errors**, 1 pre-existing warning (`JournalPanel.shared.tsx:120` exhaustive-deps; present in parent commit `e170875^`, NOT introduced by merged work) |

## 5. Findings / defects filed

None. Both implementation parents delivered the parity work; every parity item passes on
every required viewport, and the full regression gate is green. No new cards created.

Two observations (documented, not blockers):
1. Pre-existing: settings-load failure does not surface an error UI (parity-equal on all
   viewports; enhancement, not a mobile/desktop gap).
2. Pre-existing: `JournalPanel.shared.tsx:120` exhaustive-deps lint warning (predates both
   merges).

## 6. Evidence

- Settings parity spec: 12/12 pass
- Journal parity spec: 12/12 pass
- Screenshot spec: 6/6 captured
- Full e2e: 57/57 pass
- Unit: 99/99 pass
- Build: OK; Lint: 0 errors

Screenshots: `test-results/qa-parity-shots/{settings,journal}-{mobile-small,mobile-large,desktop}.png`
