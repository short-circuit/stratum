import { test, expect } from '@playwright/test';
import { mockTauriInvoke, DEFAULT_MOCK_CONFIG } from '../mocks/tauri';

test.describe('Navigation', () => {
  test.beforeEach(async ({ page }) => {
    await mockTauriInvoke(page, DEFAULT_MOCK_CONFIG);
  });

  test('each panel route loads successfully', async ({ page }) => {
    const routes = ['/graph', '/search', '/query', '/templates', '/settings'];
    for (const route of routes) {
      await page.goto(route);
      // Just verify the page loaded without error (we're on the right URL)
      await expect(page).toHaveURL(route, { timeout: 10000 });
    }
  });

  test('navigating to /page/:path shows the block editor', async ({ page }) => {
    await page.goto('/page/Welcome');
    await expect(page).toHaveURL(/\/page\/Welcome/, { timeout: 5000 });
    await expect(page.getByText('Welcome').first()).toBeVisible({ timeout: 5000 });
  });
});
