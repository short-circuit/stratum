import { test, expect, type Page } from '@playwright/test';
import path from 'path';
import { fileURLToPath } from 'url';
import { mockTauriInvoke, DEFAULT_MOCK_CONFIG } from '../mocks/tauri';

// ---------------------------------------------------------------------------
// README screenshots (t_5774176b — E11.F2 README / landing overhaul).
//
// Captures real UI states of the shipped v1.0 frontend against the mocked
// Tauri IPC surface (e2e/mocks/tauri.ts), headlessly, at desktop viewport.
// Output: docs/assets/images/<name>.png (repo-committed documentation assets).
//
// Usage:
//   npm run build                     # build dist/ first
//   npm run preview &                 # or reuse an existing preview server
//   SCREENSHOT_DIR=docs/assets/images npx playwright test --config e2e/playwright.config.ts e2e/specs/readme-shots.spec.ts
// ---------------------------------------------------------------------------

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const OUT = process.env.SCREENSHOT_DIR || path.resolve(__dirname, '../../docs/assets/images');
const DESKTOP = { width: 1280, height: 720 };

async function shot(page: Page, name: string) {
  await page.waitForTimeout(600);
  await page.screenshot({ path: path.join(OUT, name + '.png'), fullPage: false });
}

test.describe('README screenshots', () => {
  test.use({ viewport: DESKTOP });

  test('editor — block outliner page view', async ({ page }) => {
    await mockTauriInvoke(page, DEFAULT_MOCK_CONFIG);
    await page.goto('/page/Projects');
    await expect(page.getByText('Welcome').first()).toBeVisible({ timeout: 15000 }).catch(() => {});
    await expect(page.locator('.blocknote-editor-container, [contenteditable]').first()).toBeVisible({ timeout: 20000 });
    await page.waitForTimeout(1200);
    await shot(page, 'readme-editor');
  });

  test('graph — knowledge graph panel', async ({ page }) => {
    await mockTauriInvoke(page, DEFAULT_MOCK_CONFIG);
    await page.goto('/graph');
    await expect(page.locator('canvas').first()).toBeVisible({ timeout: 20000 });
    await page.waitForTimeout(2500);
    await shot(page, 'readme-graph');
  });

  test('kanban — kanban board', async ({ page }) => {
    await mockTauriInvoke(page, DEFAULT_MOCK_CONFIG);
    await page.goto('/kanban');
    await expect(page.getByText('To Do').first()).toBeVisible({ timeout: 15000 });
    await page.waitForTimeout(1000);
    await shot(page, 'readme-kanban');
  });

  test('reminders — reminder panel with sections', async ({ page }) => {
    await mockTauriInvoke(page, DEFAULT_MOCK_CONFIG);
    await page.goto('/reminders');
    await expect(page.getByText('Reminders').first()).toBeVisible({ timeout: 15000 });
    await expect(page.getByText('Ship release notes to the community').first()).toBeVisible({ timeout: 10000 });
    await page.waitForTimeout(800);
    await shot(page, 'readme-reminders');
  });

  test('plugins — plugin directory + theme hub', async ({ page }) => {
    await mockTauriInvoke(page, DEFAULT_MOCK_CONFIG);
    await page.goto('/plugins');
    await expect(page.getByRole('tab', { name: 'Installed' }).first()).toBeVisible({ timeout: 15000 });
    await page.getByRole('tab', { name: 'Directory' }).click();
    await expect(page.getByText('Plugin Directory').first()).toBeVisible({ timeout: 15000 });
    await expect(page.getByText('Hello World').first()).toBeVisible({ timeout: 15000 });
    await page.waitForTimeout(1000);
    await shot(page, 'readme-plugins');
  });

  test('onboarding — first-run wizard', async ({ page }) => {
    await mockTauriInvoke(page, { ...DEFAULT_MOCK_CONFIG, onboardingPending: true });
    await page.goto('/');
    await expect(page.getByText('Welcome to Stratum').first()).toBeVisible({ timeout: 15000 });
    await page.waitForTimeout(800);
    await shot(page, 'readme-onboarding');
  });

  test('journal — daily journal panel', async ({ page }) => {
    await mockTauriInvoke(page, DEFAULT_MOCK_CONFIG);
    await page.goto('/journal');
    await expect(page.locator('.blocknote-editor-container, [contenteditable]').first()).toBeVisible({ timeout: 20000 });
    await page.waitForTimeout(1200);
    await shot(page, 'readme-journal');
  });

  test('dictation — voice memo panel open', async ({ page }) => {
    await mockTauriInvoke(page, DEFAULT_MOCK_CONFIG);
    await page.goto('/page/Welcome');
    await expect(page.getByText('Welcome').first()).toBeVisible({ timeout: 15000 });
    await page.getByTitle('Voice memo (dictation)').click();
    await expect(page.getByText('Record voice memo').first()).toBeVisible({ timeout: 10000 });
    await page.waitForTimeout(800);
    await shot(page, 'readme-dictation');
  });
});
