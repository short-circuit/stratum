import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './specs',
  timeout: 60000,
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  workers: 1,
  use: {
    baseURL: 'tauri://localhost',
    headless: true,
  },
  // tauri-driver provides the WebDriver server on port 4444
  webServer: {
    command: 'tauri-driver',
    port: 4444,
    reuseExistingServer: !process.env.CI,
    timeout: 30000,
  },
});
