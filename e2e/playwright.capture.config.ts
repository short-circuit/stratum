import { defineConfig } from '@playwright/test';

// Minimal config for screenshot capture against a manually-started vite server
// (dev or preview). Point CAPTURE_BASE_URL at the server to capture.
export default defineConfig({
  testDir: './specs',
  testMatch: /capture-settings-evidence\.spec\.ts/,
  timeout: 60000,
  workers: 1,
  use: {
    baseURL: process.env.CAPTURE_BASE_URL ?? 'http://localhost:5173',
    headless: true,
    viewport: { width: 1280, height: 800 },
  },
  webServer: undefined,
});
