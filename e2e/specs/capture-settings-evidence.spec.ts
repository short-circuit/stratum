// ---------------------------------------------------------------------------
// Settings evidence capture (evidence helper, not a gate).
//
// Drives the Settings page against ANY vite server (dev or preview) and
// screenshots every desktop tab into $CAPTURE_DIR (default ./e2e/capture).
// Used by E11.F2 verification to produce matching before/after screenshots
// across two different source revisions served by separate vite instances.
//
// Usage:
//   CAPTURE_DIR=... CAPTURE_BASE_URL=http://localhost:5174 npx playwright test \
//     --config e2e/playwright.capture.config.ts
// ---------------------------------------------------------------------------

import { test, expect } from '@playwright/test';
import path from 'node:path';
import { mockTauriInvoke, DEFAULT_MOCK_CONFIG } from '../mocks/tauri';

const TABS = [
  'Vault',
  'Theme',
  'AI',
  'Research',
  'Developer',
  'Sync',
  'Tray',
  'Notifications',
] as const;

test('capture all desktop settings tabs', async ({ page }) => {
  const outDir = process.env.CAPTURE_DIR ?? path.resolve('e2e/capture');
  const prefix = process.env.CAPTURE_PREFIX ?? 'tab';
  await mockTauriInvoke(page, DEFAULT_MOCK_CONFIG);
  await page.goto('/settings');
  // Settings load gate: the Save button only renders after config resolves.
  await expect(page.getByRole('button', { name: /^Save$/ })).toBeVisible({ timeout: 20000 });
  await page.setViewportSize({ width: 1280, height: 800 });
  for (const tab of TABS) {
    const t = page.getByRole('tab', { name: tab });
    await expect(t).toBeVisible();
    await t.click();
    // Let the tab's async data (e.g. commit log) settle.
    await page.waitForTimeout(600);
    await page.screenshot({
      path: path.join(outDir, `${prefix}-${tab.toLowerCase()}.png`),
      fullPage: false,
    });
  }
});
