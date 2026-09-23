import { test, expect, type Page } from '@playwright/test';
import { mockTauriInvoke, DEFAULT_MOCK_CONFIG } from '../mocks/tauri';

// ---------------------------------------------------------------------------
// QA verification pass for the mobile 3D graph toggle (feature t_7a473772).
//
// Drives the REAL app (Vite preview) against the mocked Tauri backend.
// The app's mobile variant is selected by viewport width < 768 (see
// useResponsive) OR by Tauri mobile platform detection; WebGL capability comes
// from the real chromium renderer (headless chromium exposes WebGL2).
//
// Path matrix:
//   A) Mobile, capable device (webgl + plenty cores)  → toggle present,
//      toggling on renders the 3D ForceGraph canvas and persists.
//   B) Mobile, constrained (forced deviceMemory=2)     → 3D preference on but
//      unsupported: renderer stays 2D, a dismissible notice is shown.
//   C) Mobile, no WebGL (forced)                       → same graceful 2D
//      fallback + dismissible notice.
//   D) Desktop (>=768px)                               → no 3D toggle in
//      settings; ForceGraph3D (desktop 3D) still renders.
// ---------------------------------------------------------------------------

const MOCK_CFG = { ...DEFAULT_MOCK_CONFIG };

// The mobile settings sheet shows a capability line that differs by support:
//   supported   → 'Render the graph in 3D (experimental on mobile)'
//   unsupported → '3D not supported on this device — falls back to 2D'
// The fallback notice (an Alert with an onClose) reads:
//   "3D view isn't supported on this device (WebGL unavailable or too
//    constrained). Showing the 2D graph instead."
const NOTICE_TEXT = /3D view isn't supported on this device/;
const SETTINGS_UNSUPPORTED = /3D not supported on this device/;
const SETTINGS_SUPPORTED = /Render the graph in 3D/;

// Force a low-end device by defining navigator.deviceMemory=2 GiB. Must be
// installed as an init script BEFORE the app code reads deviceMemory (which it
// does during GraphPanel mount via detectGraph3dSupport).
async function forceLowEnd(page: Page) {
  await page.addInitScript(() => {
    try {
      Object.defineProperty(navigator, 'deviceMemory', { get: () => 2, configurable: true });
    } catch {
      // deviceMemory is read-only in some engines; the fallback path is no-op.
    }
  });
}

// Simulate a device WITHOUT WebGL. Overriding the prototype's native getContext
// trips "Illegal invocation" (the native is special-builtin and rejects .call
// with an alien receiver). Instead we patch document.createElement to install a
// per-instance shadowing getContext on each canvas: webgl requests → null, all
// else → real method bound to that exact canvas.
async function forceNoWebGL(page: Page) {
  await page.addInitScript(() => {
    const origCreate = document.createElement.bind(document);
    document.createElement = function (tag: string, options?: ElementCreationOptions) {
      const el = origCreate(tag, options) as HTMLCanvasElement;
      if (tag.toLowerCase() === 'canvas') {
        const realGetContext = el.getContext.bind(el);
        el.getContext = function (type: string, ...args: unknown[]): RenderingContext | null {
          // Treat every WebGL context name (webgl2, webgl, experimental-webgl)
          // as unavailable so webglAvailable() in detectGraph3dSupport fails.
          if (/^(webgl2|webgl|experimental-webgl)$/.test(type)) return null;
          return realGetContext(type as '2d', ...(args as [CanvasRenderingContext2DSettings?]));
        } as HTMLCanvasElement['getContext'];
        (el as HTMLCanvasElement & { __shadowInstalled?: boolean }).__shadowInstalled = true;
      }
      return el;
    } as typeof document.createElement;
  });
}

// (primeUse3dOn removed: superseded by mockTauriInvoke's graphSettings config,
// which seeds the preference deterministically inside the mock's own init script.)

// True when any <canvas> on the page carries a live WebGL context (i.e. the
// ForceGraph3D renderer is in use). ForceGraph2D draws on a 2d context canvas.
async function hasWebGLCanvas(page: Page): Promise<boolean> {
  return page.evaluate(() => {
    type GL = WebGLRenderingContext | WebGL2RenderingContext;
    const isGL = (c: RenderingContext | null): c is GL =>
      c !== null && typeof (c as GL).getParameter === 'function';
    const canvases = Array.from(document.querySelectorAll('canvas'));
    for (const c of canvases) {
      try {
        const gl =
          c.getContext('webgl2') ||
          c.getContext('webgl') ||
          c.getContext('experimental-webgl');
        if (isGL(gl)) return true;
      } catch {
        // A canvas may be mid-destruction; ignore and continue scanning.
      }
    }
    return false;
  });
}

test.describe('Mobile 3D graph toggle (feature t_7a473772)', () => {
  test.beforeEach(async ({ page }) => {
    await mockTauriInvoke(page, MOCK_CFG);
  });

  test('A: capable mobile device — 3D toggle present, enabling renders 3D and persists', async ({ page }) => {
    await page.goto('/graph');
    await expect(page.getByText('Building graph...')).toBeHidden({ timeout: 15000 });

    // Open mobile settings bottom sheet.
    await page.getByRole('button', { name: 'Open settings' }).click();
    const enable3d = page.getByRole('checkbox', { name: 'Enable 3D view' });
    await expect(enable3d).toBeVisible();
    // Default is 2D (opt-in feature).
    await expect(enable3d).not.toBeChecked();
    // Capability line says supported.
    await expect(page.getByText(SETTINGS_SUPPORTED)).toBeVisible();

    // Default renderer is 2D (no WebGL canvas).
    await expect
      .poll(async () => hasWebGLCanvas(page), { timeout: 5000 })
      .toBe(false);

    // Enable 3D.
    await enable3d.check();
    await expect(enable3d).toBeChecked();

    // Renderer switches to WebGL (3D) canvas.
    await expect
      .poll(async () => hasWebGLCanvas(page), { timeout: 10000 })
      .toBe(true);

    // No fallback notice on a capable device.
    await expect(page.getByText(NOTICE_TEXT)).toBeHidden();

    // Wait for the debounced save (600ms) to flush to the mock backend.
    await page.waitForTimeout(900);

    // Preference persists: reload and confirm the toggle stays on.
    await page.reload();
    await expect(page.getByText('Building graph...')).toBeHidden({ timeout: 15000 });
    await page.getByRole('button', { name: 'Open settings' }).click();
    await expect(page.getByRole('checkbox', { name: 'Enable 3D view' })).toBeChecked();
    // And it still renders 3D after reload (capable device + pref on).
    await expect
      .poll(async () => hasWebGLCanvas(page), { timeout: 10000 })
      .toBe(true);
  });

  test('B: constrained mobile device — 3D pref on but unsupported → graceful 2D fallback + dismissible notice', async ({ page }) => {
    // Seed the persisted preference use_3d:true via the mock config (deterministic;
    // the localStorage init-script ordering made the old primeUse3dOn unreliable).
    await mockTauriInvoke(page, { ...MOCK_CFG, graphSettings: { use_3d: true } });
    await forceLowEnd(page);
    await page.goto('/graph');
    await expect(page.getByText('Building graph...')).toBeHidden({ timeout: 15000 });

    // Preference was persisted ON but device is constrained → renderer is still
    // 2D (no WebGL canvas).
    await expect
      .poll(async () => hasWebGLCanvas(page), { timeout: 5000 })
      .toBe(false);

    // Dismissible fallback notice is shown.
    const notice = page.getByText(NOTICE_TEXT);
    await expect(notice).toBeVisible();

    // Dismiss via the MUI Alert onClose icon-button (the only button inside the
    // alert). The notice lives above 2D canvas; do this before opening settings
    // so the drawer scrim doesn't cover the close button.
    await page.getByRole('alert').getByRole('button').click();
    await expect(notice).toBeHidden();

    // Open settings: capability line says unsupported; checkbox reflects the
    // preserved preference (on) but it is DISABLED (device can't render 3D).
    await page.getByRole('button', { name: 'Open settings' }).click();
    await expect(page.getByText(SETTINGS_UNSUPPORTED)).toBeVisible();
    const enable3d = page.getByRole('checkbox', { name: 'Enable 3D view' });
    await expect(enable3d).toBeChecked();
    await expect(enable3d).toBeDisabled();

    // Closing the drawer doesn't bring the notice back, and renderer stays 2D.
    await page.keyboard.press('Escape');
    await expect(notice).toBeHidden();
    await expect
      .poll(async () => hasWebGLCanvas(page), { timeout: 5000 })
      .toBe(false);
  });

  test('C: mobile device without WebGL — graceful 2D fallback + dismissible notice', async ({ page }) => {
    await mockTauriInvoke(page, { ...MOCK_CFG, graphSettings: { use_3d: true } });
    await forceNoWebGL(page);
    await page.goto('/graph');
    await expect(page.getByText('Building graph...')).toBeHidden({ timeout: 15000 });

    // Guard: confirm the browser really lacks WebGL so the fallback is
    // genuinely under test (self-check of the harness override).
    const probe = await page.evaluate(() => {
      const c = document.createElement('canvas');
      let webglOk = false;
      for (const n of ['webgl2', 'webgl', 'experimental-webgl']) {
        try {
          if (c.getContext(n)) webglOk = true;
        } catch {
          // A context creation that throws is treated as unavailable.
        }
      }
      return webglOk;
    });
    expect(probe).toBe(false);

    await expect(page.getByText(NOTICE_TEXT)).toBeVisible();

    // Renderer stays 2D (no WebGL available at all).
    await expect
      .poll(async () => hasWebGLCanvas(page), { timeout: 5000 })
      .toBe(false);

    // Dismiss the notice first (so the drawer scrim doesn't cover it later).
    const notice = page.getByText(NOTICE_TEXT);
    await page.getByRole('alert').getByRole('button').click();
    await expect(notice).toBeHidden();

    // Settings: capability unsupported; checkbox on + disabled.
    await page.getByRole('button', { name: 'Open settings' }).click();
    await expect(page.getByText(SETTINGS_UNSUPPORTED)).toBeVisible();
    const enable3d = page.getByRole('checkbox', { name: 'Enable 3D view' });
    await expect(enable3d).toBeChecked();
    await expect(enable3d).toBeDisabled();
  });

  test('D: desktop unchanged — no 3D toggle in settings; 3D canvas still renders', async ({ page }) => {
    // This config runs with a mobile UA/viewport; force a full desktop UA +
    // viewport for this one so the responsive gate selects the desktop variant.
    await page.setViewportSize({ width: 1280, height: 720 });
    await page.context().addInitScript(() => {
      // Override the UA for the remainder of this context's lifetime. The
      // init script runs on navigation, before useResponsive/getPlatform read
      // navigator.userAgent.
      Object.defineProperty(navigator, 'userAgent', {
        get: () =>
          'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) HeadlessChrome/151.0.7922.34 Safari/537.36',
        configurable: true,
      });
    });
    // Re-navigate so the responsive gate re-evaluates at desktop width/UA.
    await page.goto('/graph');
    await expect(page.getByText('Building graph...')).toBeHidden({ timeout: 15000 });
    await page.getByTitle('Graph settings').click();
    // Desktop settings panel: no "3D view" control anywhere.
    await expect(page.getByText('Connected notes')).toBeVisible();
    await expect(page.getByRole('checkbox', { name: 'Enable 3D view' })).toHaveCount(0);
    // Desktop always renders 3D (GraphCanvas → ForceGraph3D).
    await expect
      .poll(async () => hasWebGLCanvas(page), { timeout: 10000 })
      .toBe(true);
  });
});
