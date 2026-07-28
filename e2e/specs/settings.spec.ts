import { test, expect } from '@playwright/test';
import { mockTauriInvoke, DEFAULT_MOCK_CONFIG } from '../mocks/tauri';

test.describe('Settings Page', () => {
  test.beforeEach(async ({ page }) => {
    await mockTauriInvoke(page, DEFAULT_MOCK_CONFIG);
    await page.goto('/settings');
    // Wait for settings tabs to render
    await expect(page.getByText('Vault').first()).toBeVisible({ timeout: 10000 });
  });

  test('settings page renders all 6 tabs', async ({ page }) => {
    const tabLabels = ['Vault', 'Theme', 'AI', 'Research', 'Developer', 'Sync'];
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
});
