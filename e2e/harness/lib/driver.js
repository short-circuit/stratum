#!/usr/bin/env node
/**
 * Stratum Automated E2E Harness — WebDriver session management.
 *
 * Drives the compiled Tauri app through tauri-driver + WebKitWebDriver using
 * the W3C WebDriver protocol (webdriver v9 + @wdio ecosystem).
 *
 * Requires the app built in PRODUCTION mode (custom-protocol feature) so
 * frontend assets are embedded in the binary — see e2e/harness/README.md.
 */

import WebDriver from 'webdriver';

export const ELEMENT_KEY = 'element-6066-11e4-a52e-4f735466cecf';

export const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

/** Resolve the WebDriver element ID from a protocol response object. */
export function elementId(x) {
  if (!x) return undefined;
  if (typeof x === 'string') return x;
  return x?.ELEMENT || x?.[ELEMENT_KEY];
}

/**
 * Create a WebDriver session against tauri-driver and launch the app.
 *
 * @param {string} appBinary absolute path to the compiled stratum-tauri binary
 * @param {object} [opts]
 * @param {string} [opts.hostname] tauri-driver host (default 127.0.0.1)
 * @param {number} [opts.port]    tauri-driver port (default 4444)
 */
export async function createSession(appBinary, opts = {}) {
  const hostname = opts.hostname || '127.0.0.1';
  const port = opts.port || 4444;

  const driver = await WebDriver.newSession({
    hostname,
    port,
    path: '/',
    protocol: 'http',
    // Fail fast instead of hanging ~2min on a stalled session negotiation;
    // callers surface tauri-driver app stderr for diagnosis (see run.mjs).
    connectionRetryTimeout: opts.connectionRetryTimeout ?? 30000,
    connectionRetryCount: opts.connectionRetryCount ?? 1,
    capabilities: {
      alwaysMatch: {
        browserName: 'wry',
        // Disable WebDriver BiDi websocket — WebKitWebDriver (webkitgtk) does
        // not support it; forces classic W3C HTTP commands.
        'wdio:enforceWebDriverClassic': true,
        'webkitgtk:browserOptions': {
          binary: appBinary,
          args: ['--automation'],
        },
      },
    },
  });
  return driver;
}

/**
 * Poll until the app window has rendered real UI content (React mounted).
 * Returns the URL when the UI is interactive.
 */
export async function waitForApp(driver, { timeoutMs = 90000, url = 'tauri://localhost' } = {}) {
  const deadline = Date.now() + timeoutMs;
  let lastUrl = '';
  let lastDetail = '';
  while (Date.now() < deadline) {
    try {
      lastUrl = await driver.getUrl();
      const probe = await driver.executeScript(
        `return {
           url: location.href,
           title: document.title,
           rootChildren: (document.getElementById('root')?.children?.length) || 0,
           anchors: document.querySelectorAll('a[href], a').length,
           bodyText: (document.body?.innerText || '').length,
         };`,
        [],
      );
      lastDetail = JSON.stringify(probe);
      const loaded =
        probe.url.startsWith(url) &&
        probe.title.length > 0 &&
        probe.rootChildren > 0 &&
        (probe.anchors > 0 || probe.bodyText > 0);
      if (loaded) return probe.url;
    } catch (e) {
      /* session not yet interactive */
    }
    await sleep(1200);
  }
  throw new Error(
    `App UI did not render within ${timeoutMs}ms (last probe: ${lastDetail}). ` +
    `Is the app built in production mode (custom-protocol)? See e2e/harness/README.md.`,
  );
}

/** Best-effort teardown of a session. */
export async function deleteSession(driver) {
  if (!driver) return;
  try {
    await driver.deleteSession();
  } catch (e) {
    /* already closed */
  }
}
