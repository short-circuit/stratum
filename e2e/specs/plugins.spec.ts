import { test, expect } from '@playwright/test';
import { mockTauriInvoke, DEFAULT_MOCK_CONFIG } from '../mocks/tauri';

// Playwright mock spec for the Plugins panel (E3.F4). Runs against the built
// web app with the in-page Tauri mock — no Rust backend required. Covers the
// full CRUD surface (list, enable/disable, install, uninstall), manifest field
// rendering (author / description / hooks), the permission summary, and the
// empty state.

test.describe('Plugins Panel (mock backend)', () => {
  test.beforeEach(async ({ page }) => {
    await mockTauriInvoke(page, DEFAULT_MOCK_CONFIG);
    await page.goto('/plugins');
    // The panel fetches on mount; wait for the seeded list to render.
    await expect(page.getByText('Developer Dashboard').first()).toBeVisible({ timeout: 10000 });
  });

  test('lists installed plugins with manifest fields', async ({ page }) => {
    // Installed plugins from the mock registry.
    await expect(page.getByText('Developer Dashboard').first()).toBeVisible();
    await expect(page.getByText('Daily Summary').first()).toBeVisible();
    await expect(page.getByText('Broken Example').first()).toBeVisible();

    // Manifest identity + metadata (name, version, id, description, author).
    await expect(page.getByText('v0.1.0').first()).toBeVisible();
    await expect(page.getByText('dev-dashboard').first()).toBeVisible();
    await expect(page.getByText('Collects development telemetry and posts it to a local endpoint.').first()).toBeVisible();
    await expect(page.getByText('By stratum-team').first()).toBeVisible();
  });

  test('shows the permission summary and hooks on each plugin', async ({ page }) => {
    const dashboard = page.locator('.MuiCard-root', { hasText: 'Developer Dashboard' });
    await expect(dashboard.getByText('file:read').first()).toBeVisible();
    await expect(dashboard.getByText('network').first()).toBeVisible();
    // Declared-enabled hooks are surfaced (spec §8).
    await expect(dashboard.getByText('Hooks:').first()).toBeVisible();
    await expect(dashboard.getByText('onSave').first()).toBeVisible();
  });

  test('disables an enabled plugin from its card', async ({ page }) => {
    const dashboard = page.locator('.MuiCard-root', { hasText: 'Developer Dashboard' });
    const disableButton = dashboard.getByRole('button', { name: 'Disable' });
    await expect(disableButton).toBeVisible();
    await disableButton.click();
    // The toggle flips to Enable and the status reflects the disabled state.
    await expect(dashboard.getByRole('button', { name: 'Enable' })).toBeVisible({ timeout: 5000 });
    await expect(dashboard.getByText('Disabled').first()).toBeVisible({ timeout: 5000 });
  });

  test('enables a disabled plugin from its card', async ({ page }) => {
    const summary = page.locator('.MuiCard-root', { hasText: 'Daily Summary' });
    const enableButton = summary.getByRole('button', { name: 'Enable' });
    await expect(enableButton).toBeVisible();
    await enableButton.click();
    await expect(summary.getByRole('button', { name: 'Disable' })).toBeVisible({ timeout: 5000 });
  });

  test('installs a plugin via the Install action (file dialog mock)', async ({ page }) => {
    await page.getByRole('button', { name: 'Install' }).first().click();
    // The mock file dialog returns installed-extras.wasm -> plugin id derived.
    await expect(page.getByText('installed-extras').first()).toBeVisible({ timeout: 5000 });
  });

  test('uninstalls a plugin from its card', async ({ page }) => {
    const summary = page.locator('.MuiCard-root', { hasText: 'Daily Summary' });
    await summary.getByRole('button', { name: 'Uninstall' }).click();
    await expect(page.getByText('Daily Summary').first()).not.toBeVisible({ timeout: 5000 });
  });

  test('shows the empty state when no plugins are installed', async ({ page }) => {
    // Fresh context with no plugins seeded.
    await mockTauriInvoke(page, { ...DEFAULT_MOCK_CONFIG, emptyPlugins: true });
    await page.goto('/plugins');
    await expect(page.getByText('No plugins installed').first()).toBeVisible({ timeout: 10000 });
    await expect(page.getByRole('button', { name: 'Install' }).first()).toBeVisible();
  });

  test('surfaces an install error when the backend rejects', async ({ page }) => {
    await mockTauriInvoke(page, { ...DEFAULT_MOCK_CONFIG, commandErrors: { plugins_list: 'plugin_load_error: invalid WASM' } });
    await page.goto('/plugins');
    await expect(page.getByText(/plugin_load_error/).first()).toBeVisible({ timeout: 10000 });
  });
});
