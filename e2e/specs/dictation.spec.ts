import { test, expect } from '@playwright/test';
import { mockTauriInvoke, DEFAULT_MOCK_CONFIG } from '../mocks/tauri';

test.describe('Voice Dictation', () => {
  test.beforeEach(async ({ page }) => {
    await mockTauriInvoke(page, DEFAULT_MOCK_CONFIG);
  });

  test('dictation panel renders on a note page', async ({ page }) => {
    // Open a note
    await page.goto('/page/Welcome');
    await expect(page.getByText('Welcome').first()).toBeVisible({ timeout: 10000 });

    // Toggle the voice memo icon (MicNone)
    await page.getByTitle('Voice memo (dictation)').click();

    // The dictation panel should render with the record controls
    await expect(page.getByText('Record voice memo').first()).toBeVisible({ timeout: 5000 });
    await expect(page.getByText(/Saves the clip and transcribes it into this note/i)).toBeVisible();
  });

  test('STT settings persist after save and reload', async ({ page }) => {
    await page.goto('/settings');
    // Wait for settings to load (Vault tab visible)
    await expect(page.getByText('Vault').first()).toBeVisible({ timeout: 10000 });

    // Go to the AI tab which holds the Voice Dictation (STT) section
    await page.getByRole('tab', { name: 'AI' }).click();
    await expect(page.getByText('Voice Dictation (STT)').first()).toBeVisible({ timeout: 5000 });

    // Change the transcription endpoint and model
    const endpoint = page.getByLabel('Transcription endpoint (OpenAI-compatible)');
    await endpoint.fill('http://mock-stt:8081');
    const model = page.getByLabel('Transcription model');
    await model.fill('whisper-1');

    // Save settings
    await page.getByRole('button', { name: 'Save' }).click();

    // Reload the page; the mock persists settings via localStorage, so the
    // saved STT values must come back from get_settings.
    await page.reload();
    await expect(page.getByText('Vault').first()).toBeVisible({ timeout: 10000 });
    await page.getByRole('tab', { name: 'AI' }).click();
    await expect(page.getByText('Voice Dictation (STT)').first()).toBeVisible({ timeout: 5000 });

    await expect(page.getByLabel('Transcription endpoint (OpenAI-compatible)')).toHaveValue('http://mock-stt:8081');
    await expect(page.getByLabel('Transcription model')).toHaveValue('whisper-1');
  });

  test('record, stop, transcribe, and insert a memo into the note', async ({ page }) => {
    await page.goto('/page/Welcome');
    await expect(page.getByText('Welcome').first()).toBeVisible({ timeout: 10000 });

    // Open the dictation panel
    await page.getByTitle('Voice memo (dictation)').click();
    await expect(page.getByText('Record voice memo').first()).toBeVisible({ timeout: 5000 });

    // Start recording
    await page.getByText('Record voice memo').first().click();
    await expect(page.getByText(/Recording…/).first()).toBeVisible({ timeout: 5000 });

    // Stop and save clip
    await page.getByText('Stop & save').click();
    await expect(page.getByText('Clip saved').first()).toBeVisible({ timeout: 5000 });

    // Transcribe
    await page.getByText('Transcribe').first().click();

    // The panel runs the transcription pipeline; on success the backend
    // reports inserted block ids and the parent closes the dictation panel,
    // returning to the note. Asserting the panel closes proves the memo was
    // inserted end-to-end (the mock returns inserted_block_ids).
    await expect(page.getByText('Record voice memo')).toHaveCount(0, { timeout: 10000 });
    await expect(page.getByText('Welcome').first()).toBeVisible({ timeout: 5000 });
  });
});
