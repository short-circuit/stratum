import { test, expect } from '@playwright/test';
import { mockTauriInvoke, DEFAULT_MOCK_CONFIG } from '../mocks/tauri';

test.describe('Close to tray', () => {
  test('close_to_tray hides window and keeps app running', async ({ page }) => {
    await mockTauriInvoke(page, DEFAULT_MOCK_CONFIG);
    await page.goto('/journal');
    // Let the journal panel and the settings cache (used as tray fallback) load.
    await page.waitForTimeout(2500);

    await page.evaluate(() => window.__emitTauriEvent('tauri://close-requested', null));
    await page.waitForTimeout(500);

    const invoked = await page.evaluate(() => (window as any).__invokedCommands);
    expect(invoked).toContain('plugin:window|hide');
    expect(invoked).not.toContain('plugin:window|destroy');
  });

  test('close_to_tray disabled destroys window', async ({ page }) => {
    await mockTauriInvoke(page, { ...DEFAULT_MOCK_CONFIG, trayStatus: { enabled: true, close_to_tray: false } });
    await page.goto('/journal');
    await page.waitForTimeout(2500);

    await page.evaluate(() => window.__emitTauriEvent('tauri://close-requested', null));
    await page.waitForTimeout(500);

    const invoked = await page.evaluate(() => (window as any).__invokedCommands);
    expect(invoked).toContain('plugin:window|destroy');
    expect(invoked).not.toContain('plugin:window|hide');
  });

  test('get_tray_status failure falls back to settings cache and still hides', async ({ page }) => {
    await mockTauriInvoke(page, { ...DEFAULT_MOCK_CONFIG, commandErrors: { get_tray_status: 'boom' } });
    await page.goto('/journal');
    await page.waitForTimeout(2500);

    await page.evaluate(() => window.__emitTauriEvent('tauri://close-requested', null));
    await page.waitForTimeout(500);

    const invoked = await page.evaluate(() => (window as any).__invokedCommands);
    expect(invoked).toContain('plugin:window|hide');
    expect(invoked).not.toContain('plugin:window|destroy');
  });
});
