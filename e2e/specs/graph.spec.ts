import { test, expect } from '@playwright/test';
import { mockTauriInvoke, DEFAULT_MOCK_CONFIG } from '../mocks/tauri';

test.describe('Graph Panel', () => {
  test.beforeEach(async ({ page }) => {
    await mockTauriInvoke(page, DEFAULT_MOCK_CONFIG);
    // Navigate directly to graph (skip root redirect which causes frame detach)
    await page.goto('/graph');
    await expect(page.getByRole('button', { name: 'Refresh' }).first()).toBeVisible({ timeout: 10000 });
  });

  test('graph panel shows correct node and edge counts', async ({ page }) => {
    await expect(page.getByText(/5.*n.*6.*e/)).toBeVisible({ timeout: 5000 });
  });

  test('graph settings panel opens and shows toggles', async ({ page }) => {
    const settingsButton = page.locator('button[title="Graph settings"]');
    await settingsButton.click();

    await expect(page.getByText('Connected notes')).toBeVisible();
    await expect(page.getByText('Orphaned notes')).toBeVisible();
    await expect(page.getByText('Tags on hover')).toBeVisible();
  });
});
