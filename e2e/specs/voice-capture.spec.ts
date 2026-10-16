import { test, expect } from '@playwright/test';
import { mockTauriInvoke, DEFAULT_MOCK_CONFIG } from '../mocks/tauri';

// ---------------------------------------------------------------------------
// E9.F4 — voice-capture big button (mobile-first).
//
// Drives the REAL app against the mocked Tauri backend. The mobile variant is
// selected by viewport width < 768 (see useResponsive). The voice-capture flow
// runs the existing pkm-dictation pipeline (mocked here) and must never lose
// data: on transcription failure a clip-reference placeholder is inserted.
// ---------------------------------------------------------------------------

test.use({ viewport: { width: 390, height: 844 } });

test.describe('Voice capture big button (E9.F4)', () => {
  test.beforeEach(async ({ page }) => {
    await mockTauriInvoke(page, { ...DEFAULT_MOCK_CONFIG });
  });

  test('mobile journal shows the FAB and inserts a memo on record → stop → transcribe', async ({ page }) => {
    await page.goto('/journal');
    const fab = page.getByRole('button', { name: 'Record voice memo' });
    await expect(fab).toBeVisible();

    // Tap to start recording.
    await fab.click();
    const stop = page.getByRole('button', { name: 'Stop & save' });
    await expect(stop).toBeVisible();

    // Stop → transcribe runs and the memo is inserted into the journal.
    await stop.click();
    await expect(page.getByText('Voice Memo', { exact: true })).toBeVisible({ timeout: 10000 });

    const calls = await page.evaluate(() => (window as any).__mockVoice);
    expect(calls.transcribeCalls).toBe(1);
    expect(calls.clipCalls).toBe(0);
  });

  test('cancel stops the recording without transcribing', async ({ page }) => {
    await page.goto('/journal');
    const fab = page.getByRole('button', { name: 'Record voice memo' });
    await fab.click();

    const cancel = page.getByRole('button', { name: 'Cancel' });
    await expect(cancel).toBeVisible();
    await cancel.click();

    // Back to idle, no memo inserted.
    await expect(page.getByRole('button', { name: 'Record voice memo' })).toBeVisible();
    const calls = await page.evaluate(() => (window as any).__mockVoice);
    expect(calls.transcribeCalls).toBe(0);
    expect(calls.clipCalls).toBe(0);
  });

  test('transcription failure inserts a clip-reference placeholder and retry succeeds', async ({ page }) => {
    await mockTauriInvoke(page, {
      ...DEFAULT_MOCK_CONFIG,
      dictationTranscribeFailures: 1,
    });
    await page.goto('/journal');

    await page.getByRole('button', { name: 'Record voice memo' }).click();
    await page.getByRole('button', { name: 'Stop & save' }).click();

    // Error fallback: placeholder inserted + retry affordance shown.
    const retry = page.getByRole('button', { name: 'Retry transcription' });
    await expect(retry).toBeVisible({ timeout: 10000 });
    const afterFail = await page.evaluate(() => (window as any).__mockVoice);
    expect(afterFail.clipCalls).toBe(1);
    expect(afterFail.transcribeCalls).toBe(1);

    // Retry transcription now succeeds — the real memo heading appears.
    await retry.click();
    await expect(page.getByText('Voice Memo', { exact: true })).toBeVisible({ timeout: 10000 });
    const calls = await page.evaluate(() => (window as any).__mockVoice);
    expect(calls.transcribeCalls).toBe(2);
    expect(calls.clipCalls).toBe(1);
  });
});
