import { test, expect } from '@playwright/test';
import { mockTauriInvoke, DEFAULT_MOCK_CONFIG } from '../mocks/tauri';

test.describe('Search Panel', () => {
  test.beforeEach(async ({ page }) => {
    await mockTauriInvoke(page, DEFAULT_MOCK_CONFIG);
    await page.goto('/search');
    await expect(page.getByPlaceholder(/search blocks/i)).toBeVisible({ timeout: 10000 });
  });

  test('search input accepts text', async ({ page }) => {
    const searchInput = page.getByPlaceholder(/search blocks/i);
    await searchInput.fill('project');
    await expect(searchInput).toHaveValue('project');
  });

  test('search results appear after searching', async ({ page }) => {
    const searchInput = page.getByPlaceholder(/search blocks/i);
    await searchInput.fill('project');

    // Press Enter to trigger search
    await searchInput.press('Enter');

    // Wait for results to appear — should show the search result items
    await expect(page.getByText(/Build the/).first()).toBeVisible({ timeout: 5000 });
    await expect(page.getByText(/Active Projects/).first()).toBeVisible();
  });
});
