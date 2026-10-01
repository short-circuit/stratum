import { defineConfig } from '@playwright/test';

// Focused QA config for the mobile 3D graph toggle verification pass.
// Runs ONLY the mobile-specific spec at mobile viewport sizes so the
// GraphPanel mobile variant (selected by width < 768) is exercised.
// Desktop-unchanged assertions are covered inside this spec because the
// default desktop viewport is applied to the desktop tests below.
export default defineConfig({
  testDir: './specs',
  timeout: 60000,
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  workers: 1,
  testMatch: /graph3d-mobile\.spec\.ts/,
  use: {
    baseURL: 'http://localhost:4173',
    trace: 'on-first-retry',
    headless: true,
    // Mobile viewport + mobile UA so useResponsive's platform detection
    // (getPlatform().isMobile via android/ios UA) selects the mobile variant.
    // NB: with isMobile:true Playwright clamps innerWidth to ~980, so the UA
    // is the ONLY reliable mobile selector here — mirrors a real device where
    // the WebView may also report a desktop width but must still render mobile.
    viewport: { width: 390, height: 844 },
    userAgent:
      'Mozilla/5.0 (Linux; Android 14; Pixel 7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/151.0.7922.34 Mobile Safari/537.36',
    hasTouch: true,
  },
  webServer: {
    command: 'npm run preview',
    url: 'http://localhost:4173',
    reuseExistingServer: !process.env.CI,
    timeout: 15000,
  },
});
