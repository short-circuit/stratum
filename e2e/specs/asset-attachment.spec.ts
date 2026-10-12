import { test, expect } from '@playwright/test';
import { mockTauriInvoke, DEFAULT_MOCK_CONFIG } from '../mocks/tauri';

test.describe('Asset attachment UI (E9.F2)', () => {
  test.beforeEach(async ({ page }) => {
    await mockTauriInvoke(page, DEFAULT_MOCK_CONFIG);
  });

  test('AttachmentPanel lists page assets and the editor embeds the asset image', async ({ page }) => {
    await page.goto('/page/Welcome');
    await expect(page).toHaveURL(/\/page\/Welcome/, { timeout: 8000 });

    // The editor should render the `![[assets/foo-abc.png]]` block as an image,
    // and resolveFileUrl must have translated it to a real data: URL.
    const img = page.locator('img').first();
    await expect(img).toBeVisible({ timeout: 8000 });
    const src = await img.getAttribute('src');
    expect(src).toMatch(/^data:image\/png;base64,/);

    // The AttachmentPanel header lists the count.
    await expect(page.getByText(/Attachments \(2\)/)).toBeVisible({ timeout: 8000 });
    await expect(page.getByText('foo-abc.png')).toBeVisible();
    await expect(page.getByText('doc-123.pdf')).toBeVisible();

    // The header "Attach" button is present.
    await expect(page.getByTitle('Attach').first()).toBeVisible();

    await page.screenshot({ path: 'e2e/capture/asset-attachment.png', fullPage: true });
  });
});
