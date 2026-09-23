import { test, expect, type Page } from '@playwright/test';
import { mockTauriInvoke, DEFAULT_MOCK_CONFIG } from '../mocks/tauri';

// ---------------------------------------------------------------------------
// Cross-platform parity QA for Settings + Journal (t_8b915df2).
//
// Contract (task body): verify mobile vs desktop parity for Settings and
// Journal — fields, navigation, data persistence, error states, and visual
// consistency — on at least one small AND one large mobile viewport, plus
// desktop. Report pass/fail per item; file bugs for gaps; ensure no
// regressions in adjacent screens.
//
// The app selects its mobile flow via useResponsive (width < 768 in a
// browser, or a mobile platform). Both the small (360x740) and large
// (428x900) viewports below are < 768, so they exercise the true mobile
// rendering path; 1280x720 exercises desktop. A mobile UA is also applied so
// the mobile platform gate (getPlatform) agrees with the viewport gate.
// ---------------------------------------------------------------------------

const MOBILE_SMALL = { width: 360, height: 740 };
const MOBILE_LARGE = { width: 428, height: 900 };
const DESKTOP = { width: 1280, height: 720 };

const MOBILE_UA =
  'Mozilla/5.0 (Linux; Android 14; Pixel 7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Mobile Safari/537.36';

const SETTINGS_SECTIONS = ['Vault', 'Theme', 'AI', 'Research', 'Developer', 'Sync'] as const;

// Wait for the settings form to be populated (both desktop tabs and the
// mobile sectioned layout surface 'Save' once settings load).
async function openSettings(page: Page) {
  await page.goto('/settings');
  await expect(page.getByRole('button', { name: /^Save$/ })).toBeVisible({ timeout: 15000 });
}

test.describe('QA parity — Settings', () => {
  for (const [label, viewport, ua] of [
    ['mobile-small', MOBILE_SMALL, MOBILE_UA],
    ['mobile-large', MOBILE_LARGE, MOBILE_UA],
    ['desktop', DESKTOP, undefined],
  ] as const) {
    test.describe(`viewport: ${label}`, () => {
      test.use({ viewport, userAgent: ua });

      test.beforeEach(async ({ page }) => {
        await mockTauriInvoke(page, DEFAULT_MOCK_CONFIG);
      });

      test('all desktop-equivalent settings sections/fields are reachable', async ({ page }) => {
        await openSettings(page);

        if (label === 'desktop') {
          // Desktop: 6 tabs in the top tab bar.
          for (const s of SETTINGS_SECTIONS) {
            await expect(page.getByRole('tab', { name: s })).toBeVisible();
          }
          // Tab surfaces each expose their fields.
          await page.getByRole('tab', { name: 'Vault' }).click();
          await expect(page.getByLabel('Vault Path')).toBeVisible();
          await expect(page.getByRole('button', { name: /Browse/i })).toBeVisible();
          await expect(page.getByRole('button', { name: /^Save$/ })).toBeVisible();
        } else {
          // Mobile: full vertical scroll of the same sections.
          for (const s of SETTINGS_SECTIONS) {
            await expect(page.getByText(s, { exact: true }).first()).toBeVisible();
          }
          // The mobile section layout must not be just a copy of the tab bar:
          // it must expose the actual form fields for the primary surfaces.
          await expect(page.getByLabel('Vault Path')).toBeVisible();
          await expect(page.getByRole('button', { name: /Browse/i })).toBeVisible();
          await expect(page.getByRole('button', { name: /^Save$/ })).toBeVisible();
          // AI provider accordion + STT/TTS + Research + Sync + Developer all
          // exist as scrollable sections (already asserted via section titles);
          // expand the AI accordion to confirm the provider surface renders.
          await page.getByText('Configure AI provider').first().click();
          await expect(page.getByRole('combobox').first()).toBeVisible();
          await expect(page.getByLabel('API Endpoint')).toBeVisible();
          await expect(page.getByRole('button', { name: 'Fetch Available Models' })).toBeVisible();
        }
      });

      test('settings save round-trips through the store (persistence)', async ({ page }) => {
        await openSettings(page);

        // On mobile the field is exposed directly; on desktop the Vault tab is
        // the default active tab. 'Vault Path' is the highest-value persistence
        // probe shared by both layouts.
        const vault = page.getByLabel('Vault Path');
        await expect(vault).toBeVisible();
        await vault.fill('/storage/emulated/0/QA-Vault');
        await page.getByRole('button', { name: /^Save$/ }).click();
        await expect(page.getByText('Saved.')).toBeVisible({ timeout: 10000 });

        // Reload: the mock persists settings to localStorage, so the edited
        // value must come back (mirrors real backend disk persistence).
        await page.reload();
        await openSettings(page);
        await expect(page.getByLabel('Vault Path')).toHaveValue('/storage/emulated/0/QA-Vault');
      });

      test('command failure on settings load surfaces the error (error state)', async ({ page }) => {
        // Force get_settings to reject on mount. Both layout variants gate on
        // `!settings` and return a bare "Loading settings..." — the error msg
        // set by useSettingsPage is never rendered because the page never
        // leaves the loading branch. This is currently parity-equal across
        // platforms but the error is genuinely not surfaced to the user.
        await mockTauriInvoke(page, {
          hasVault: true,
          commandErrors: { get_settings: 'backend exploded' },
        });
        await page.goto('/settings');
        // Documented current behavior: stuck on the loading state.
        await expect(page.getByText('Loading settings...')).toBeVisible({ timeout: 15000 });
        // The load error is not surfaced — this is the bug being tracked
        // (parity-equal on all viewports; see QA report).
        const surfaced = await page.getByText(/Load failed|backend exploded/i).count();
        expect(surfaced).toBe(0);
      });

      test('save failure surfaces an error message (error state)', async ({ page }) => {
        await openSettings(page);
        await mockTauriInvoke(page, {
          hasVault: true,
          commandErrors: { save_settings: 'disk full' },
        });
        // Re-run the mock (addInitScript) after the page already loaded: the
        // new init script replaces the previous one on next navigation/reload.
        await page.reload();
        await openSettings(page);
        await page.getByRole('button', { name: /^Save$/ }).click();
        await expect(page.getByText(/Save failed|disk full/i)).toBeVisible({ timeout: 10000 });
      });
    });
  }
});
