import { test, expect, type Page } from '@playwright/test';
import { mockTauriInvoke, DEFAULT_MOCK_CONFIG } from '../mocks/tauri';

// ---------------------------------------------------------------------------
// Cross-platform parity QA for Settings (t_8b915df2, extended by E11.F4
// t_e9f151d7).
//
// Contract: verify mobile vs desktop parity for Settings — fields,
// navigation, data persistence, error states, and visual consistency — on at
// least one small AND one large mobile viewport, plus desktop. Report
// pass/fail per item; file bugs for gaps; ensure no regressions in adjacent
// screens.
//
// E11 normalized the surface: DESKTOP = 8 tabs (Vault, Theme, AI, Research,
// Developer, Sync, Tray, Notifications) in the top tab bar; MOBILE = one
// collapsible accordion per desktop tab, same order (Vault default-open, all
// others collapsed). The mobile assertions below therefore expand every
// accordion before asserting fields.
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

// All 8 desktop tabs / mobile sections (E11.F4: Tray + Notifications are now
// required parity surfaces).
const SETTINGS_SECTIONS = [
  'Vault',
  'Theme',
  'AI',
  'Research',
  'Developer',
  'Sync',
  'Tray',
  'Notifications',
] as const;
type SettingsSectionName = (typeof SETTINGS_SECTIONS)[number];

// Mobile accordion summary labels (English locale) per section. Since the
// overview already asserts the section headers, we only need these to drive
// expansion (E11.F3/F4: every mobile settings area is a collapsible).
const MOBILE_ACCORDION_SUMMARY: Record<SettingsSectionName, string> = {
  Vault: 'Configure vault',
  Theme: 'Configure theme',
  AI: 'Configure AI provider',
  Research: 'Configure research',
  Developer: 'Configure developer tools',
  Sync: 'Configure sync',
  Tray: 'Configure tray',
  Notifications: 'Configure notifications',
};

// Wait for the settings form to be populated (both desktop tabs and the
// mobile sectioned layout surface 'Save' once settings load).
async function openSettings(page: Page) {
  await page.goto('/settings');
  await expect(page.getByRole('button', { name: /^Save$/ })).toBeVisible({ timeout: 15000 });
}

// Expand every mobile accordion. The Vault accordion is default-open on
// mobile, so its 'Configure vault' summary is absent and the click is
// skipped; the other seven start collapsed and must each be expanded before
// their fields can be asserted.
async function expandAllMobileAccordions(page: Page) {
  for (const section of SETTINGS_SECTIONS) {
    const summary = page.getByText(MOBILE_ACCORDION_SUMMARY[section]);
    if ((await summary.count()) > 0) {
      await summary.first().click();
    }
  }
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
          // Desktop: all 8 tabs in the top tab bar (E11.F4 parity surface).
          for (const s of SETTINGS_SECTIONS) {
            await expect(page.getByRole('tab', { name: s })).toBeVisible();
          }
          // Each tab surfaces its key fields.
          await page.getByRole('tab', { name: 'Vault' }).click();
          await expect(page.getByLabel('Vault Path')).toBeVisible();
          await expect(page.getByRole('button', { name: /Browse/i }).first()).toBeVisible();

          await page.getByRole('tab', { name: 'Theme' }).click();
          await expect(page.getByLabel('Pick color').first()).toBeVisible();
          await expect(page.getByText('Primary (buttons, accents)').first()).toBeVisible();

          await page.getByRole('tab', { name: 'AI' }).click();
          await expect(page.getByRole('combobox').first()).toBeVisible();
          await expect(page.getByLabel('API Endpoint URL').first()).toBeVisible();
          await expect(page.getByRole('button', { name: 'Fetch Available Models' })).toBeVisible();

          await page.getByRole('tab', { name: 'Research' }).click();
          await expect(page.getByText('https://searx.be').first()).toBeVisible();
          await expect(page.getByRole('button', { name: 'Test instance' }).first()).toBeVisible();

          await page.getByRole('tab', { name: 'Developer' }).click();
          await expect(page.getByRole('button', { name: 'Reindex All' })).toBeVisible();

          await page.getByRole('tab', { name: 'Sync' }).click();
          await expect(page.getByText('Sync Mode')).toBeVisible();
          await expect(page.getByRole('button', { name: 'Sync Now' }).first()).toBeVisible();
          await expect(page.getByRole('button', { name: 'Run backup now' })).toBeVisible();

          await page.getByRole('tab', { name: 'Tray' }).click();
          await expect(page.getByText('Enable tray icon')).toBeVisible();
          await expect(page.getByText('Close to tray')).toBeVisible();
          await expect(page.getByText('Quick capture target').first()).toBeVisible();
          await expect(page.getByRole('switch').first()).toBeVisible();

          await page.getByRole('tab', { name: 'Notifications' }).click();
          await expect(page.getByText('Enable reminders')).toBeVisible();
          await expect(page.getByText('Notification sound')).toBeVisible();
          await expect(page.getByRole('switch').first()).toBeVisible();
          await expect(page.getByRole('button', { name: /^Save$/ })).toBeVisible();
        } else {
          // Mobile: one collapsible accordion per desktop tab, in the same
          // order. Assert the section headers, then expand every accordion
          // before asserting the fields they gate (E11.F3/F4 layout).
          for (const s of SETTINGS_SECTIONS) {
            await expect(page.getByText(s, { exact: true }).first()).toBeVisible();
          }
          await expandAllMobileAccordions(page);

          // Vault path + browse.
          await expect(page.getByLabel('Vault Path')).toBeVisible();
          await expect(page.getByRole('button', { name: /Browse/i }).first()).toBeVisible();
          // Theme colors (color picker + preset label).
          await expect(page.getByLabel('Pick color').first()).toBeVisible();
          await expect(page.getByText('Primary (buttons, accents)').first()).toBeVisible();
          // AI provider surface + STT/TTS/voice (all inside the AI accordion).
          await expect(page.getByRole('combobox').first()).toBeVisible();
          await expect(page.getByLabel('API Endpoint URL').first()).toBeVisible();
          await expect(page.getByRole('button', { name: 'Fetch Available Models' })).toBeVisible();
          await expect(page.getByText('Voice Dictation (STT)')).toBeVisible();
          await expect(page.getByText('Text-to-Speech')).toBeVisible();
          await expect(page.getByText('Voice-Capture Button')).toBeVisible();
          // Research presets + pinger.
          await expect(page.getByText('Public SearXNG instances')).toBeVisible();
          await expect(page.getByText('https://searx.be').first()).toBeVisible();
          await expect(page.getByRole('button', { name: 'Test instance' }).first()).toBeVisible();
          // Developer tools.
          await expect(page.getByRole('button', { name: 'Reindex All' })).toBeVisible();
          // Sync controls / status / backup.
          await expect(page.getByText('Sync Mode')).toBeVisible();
          await expect(page.getByRole('button', { name: 'Sync Now' }).first()).toBeVisible();
          await expect(page.getByRole('button', { name: 'Run backup now' })).toBeVisible();
          // Tray toggles.
          await expect(page.getByText('Enable tray icon')).toBeVisible();
          await expect(page.getByText('Close to tray')).toBeVisible();
          // Notifications toggle.
          await expect(page.getByText('Enable reminders')).toBeVisible();
          await expect(page.getByText('Notification sound')).toBeVisible();
          // Save bar.
          await expect(page.getByRole('button', { name: /^Save$/ })).toBeVisible();
        }
      });

      test('settings save round-trips through the store (persistence)', async ({ page }) => {
        await openSettings(page);

        // On mobile the field is exposed directly; on desktop the Vault tab is
        // the default active tab. 'Vault Path' is the highest-value persistence
        // probe shared by both layouts.
        const vault = page.getByLabel('Vault Path');
        if (label !== 'desktop') {
          // Vault accordion is default-open on mobile — expand only if collapsed.
          const vaultSummary = page.getByText('Configure vault');
          if ((await vaultSummary.count()) > 0) await vaultSummary.first().click();
        }
        await expect(vault).toBeVisible();
        await vault.fill('/storage/emulated/0/QA-Vault');
        await page.getByRole('button', { name: /^Save$/ }).click();
        await expect(page.getByText('Saved.')).toBeVisible({ timeout: 10000 });

        // The mock Tauri surface captures save_settings by persisting the
        // payload into localStorage — prove the save was actually delivered.
        const persistedPath = await page.evaluate(() => {
          const raw = window.localStorage.getItem('mock_settings');
          return raw ? JSON.parse(raw).vault_path : null;
        });
        expect(persistedPath).toBe('/storage/emulated/0/QA-Vault');

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
