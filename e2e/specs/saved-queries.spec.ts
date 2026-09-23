import { test, expect } from '@playwright/test';
import { mockTauriInvoke, DEFAULT_MOCK_CONFIG } from '../mocks/tauri';

test.describe('Query Panel — saved queries', () => {
  test.beforeEach(async ({ page }) => {
    await mockTauriInvoke(page, DEFAULT_MOCK_CONFIG);
    await page.goto('/query');
    await expect(page.getByPlaceholder(/enter datalog/i).first()).toBeVisible({ timeout: 10000 });
  });

  test('renders the persisted saved query list', async ({ page }) => {
    // The mock seeds a baseline list; the UI must surface it after load.
    await expect(page.getByText('Saved Queries (2)')).toBeVisible({ timeout: 10000 });
    await expect(page.getByText('All TODO')).toBeVisible();
    await expect(page.getByText('Blocked')).toBeVisible();
  });

  test('loads a saved query into the editor on click', async ({ page }) => {
    await expect(page.getByText('All TODO')).toBeVisible({ timeout: 10000 });
    await page.getByText('All TODO').click();

    const editor = page.locator('textarea').first();
    await expect(editor).toHaveValue(/:query \[:find \?b :where \[\?b :block\/marker "TODO"\]/);
  });

  test('saves the current query under a new name and lists it', async ({ page }) => {
    await expect(page.getByText('All TODO')).toBeVisible({ timeout: 10000 });

    await page.getByRole('button', { name: /Save Query/ }).click();
    await page.getByRole('textbox', { name: 'Name' }).fill('My Saved Query');
    await page.getByRole('button', { name: 'Save', exact: true }).click();

    await expect(page.getByText('My Saved Query')).toBeVisible({ timeout: 10000 });
    await expect(page.getByText('Saved Queries (3)')).toBeVisible();
  });

  test('renames a saved query through the dialog', async ({ page }) => {
    await expect(page.getByText('Blocked')).toBeVisible({ timeout: 10000 });

    await page.getByRole('button', { name: 'Rename saved query Blocked', exact: true }).click();
    await page.getByRole('textbox', { name: 'Name' }).fill('Waiting Items');
    await page.getByRole('button', { name: 'Rename', exact: true }).click();

    await expect(page.getByText('Waiting Items')).toBeVisible({ timeout: 10000 });
    await expect(page.getByText('Blocked')).not.toBeVisible();
  });

  test('deletes a saved query after confirmation', async ({ page }) => {
    await expect(page.getByText('All TODO')).toBeVisible({ timeout: 10000 });

    await page.getByRole('button', { name: 'Delete saved query All TODO', exact: true }).click();
    await expect(page.getByText(/Delete “All TODO”/)).toBeVisible();
    await page.getByRole('button', { name: 'Delete', exact: true }).click();

    await expect(page.getByText('All TODO')).not.toBeVisible({ timeout: 10000 });
    await expect(page.getByText('Saved Queries (1)')).toBeVisible();
  });

  test('persists saved queries changes across a reload (restart)', async ({ page }) => {
    await expect(page.getByText('All TODO')).toBeVisible({ timeout: 10000 });

    await page.getByRole('button', { name: /Save Query/ }).click();
    await page.getByRole('textbox', { name: 'Name' }).fill('Persisted Query');
    await page.getByRole('button', { name: 'Save', exact: true }).click();
    await expect(page.getByText('Persisted Query')).toBeVisible({ timeout: 10000 });

    // Reload the app — this exercises the restart path: a fresh JS context
    // where the mocked store (persisted in localStorage, mirroring on-disk
    // storage) must still list the saved query.
    await page.reload();
    await expect(page.getByPlaceholder(/enter datalog/i).first()).toBeVisible({ timeout: 10000 });

    await expect(page.getByText('Persisted Query')).toBeVisible({ timeout: 10000 });
    await expect(page.getByText('Saved Queries (3)')).toBeVisible();
  });
});
