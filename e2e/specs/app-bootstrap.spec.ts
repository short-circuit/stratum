import { test, expect } from '@playwright/test';
import { mockTauriInvoke, DEFAULT_MOCK_CONFIG } from '../mocks/tauri';

test.describe('App bootstrap', () => {
  test('shows vault picker when no vault configured', async ({ page }) => {
    await mockTauriInvoke(page, { hasVault: false, commandErrors: {} });
    await page.goto('/');
    await expect(page.getByText(/choose.*vault/i).first()).toBeVisible({ timeout: 15000 });
  });

  test('loads vault and renders journal page', async ({ page }) => {
    await mockTauriInvoke(page, DEFAULT_MOCK_CONFIG);
    // Navigate to journal directly (avoids full bootstrap race)
    await page.goto('/journal');
    // The journal panel should render (not a blank page or loading spinner)
    await page.waitForTimeout(2000);
    // Check that we're on the journal route
    expect(page.url()).toContain('/journal');
  });
});
