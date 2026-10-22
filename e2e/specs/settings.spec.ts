import { test, expect } from '@playwright/test';
import { mockTauriInvoke, DEFAULT_MOCK_CONFIG } from '../mocks/tauri';

test.describe('Settings Page', () => {
  test.beforeEach(async ({ page }) => {
    await mockTauriInvoke(page, DEFAULT_MOCK_CONFIG);
    await page.goto('/settings');
    // Wait for settings tabs to render
    await expect(page.getByText('Vault').first()).toBeVisible({ timeout: 10000 });
  });

  test('settings page renders all 8 tabs', async ({ page }) => {
    const tabLabels = ['Vault', 'Theme', 'AI', 'Research', 'Developer', 'Sync', 'Tray', 'Notifications'];
    for (const label of tabLabels) {
      await expect(page.getByText(label).first()).toBeVisible();
    }
  });

  test('theme tab shows color picker', async ({ page }) => {
    // Click the Theme tab
    await page.getByText('Theme').first().click();

    // The theme tab should show color inputs (primary and secondary)
    const colorInputs = page.locator('input[type="color"]');
    await expect(colorInputs.first()).toBeVisible({ timeout: 5000 });

    // Should also show preset color swatches
    await expect(page.getByText(/primary.*buttons|accent/i)).toBeVisible();
  });

  test('AI tab discovers local endpoints (mocked)', async ({ page }) => {
    await page.getByText('AI').first().click();
    await page.getByText('Discover local models').click();

    await expect(page.getByText('Ollama').first()).toBeVisible();
    await expect(page.getByText('llama3.2:latest').first()).toBeVisible();
    await expect(page.getByText('Unreachable').first()).toBeVisible();

    await page.getByText('Use this endpoint').first().click();
    const endpointInput = page.getByLabel('API Endpoint URL');
    await expect(endpointInput).toHaveValue('http://127.0.0.1:11434');
  });

  test('Research tab shows presets and pings', async ({ page }) => {
    await page.getByText('Research').first().click();

    await expect(page.getByText('https://searx.be').first()).toBeVisible();
    await expect(page.getByText('https://search.bus-hit.me').first()).toBeVisible();
    await expect(page.getByText('https://priv.au').first()).toBeVisible();
    await expect(page.getByText('https://searx.tiekoetter.com').first()).toBeVisible();

    await page.getByText('Test instance').first().click();
    await expect(page.getByText('Reachable — 180ms').first()).toBeVisible();
  });

  test('sync tab backup: run-now reports success', async ({ page }) => {
    await mockTauriInvoke(page, DEFAULT_MOCK_CONFIG);
    await page.goto('/settings');
    await expect(page.getByText('Vault').first()).toBeVisible({ timeout: 10000 });
    await page.getByText('Sync').first().click();
    await expect(page.getByText(/Backup/).first()).toBeVisible();
    await page.getByText('Run backup now').first().click();
    await expect(page.getByText('Backup completed.').first()).toBeVisible({ timeout: 5000 });
  });
});
