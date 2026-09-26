#!/usr/bin/env node
// Stratum acceptance UI driver — Phase D.
// Drives the compiled production binary via tauri-driver (already running on :4444).
// HOME is controlled so lib.rs auto-resolves ~/StratumVault = /tmp/stratum-home/StratumVault.

import WebDriver from 'webdriver';

const APP = '/home/shrtcrct/git/stratum/target/debug/stratum-tauri';
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

const ELEMENT_KEY = 'element-6066-11e4-a52e-4f735466cecf';
function id(x) {
  if (!x) return undefined;
  if (typeof x === 'string') return x;
  return x?.ELEMENT || x?.[ELEMENT_KEY];
}

async function createSession() {
  return WebDriver.newSession({
    hostname: '127.0.0.1',
    port: 4444,
    path: '/',
    protocol: 'http',
    connectionRetryTimeout: 30000,
    connectionRetryCount: 1,
    capabilities: {
      alwaysMatch: {
        browserName: 'wry',
        'wdio:enforceWebDriverClassic': true,
        'webkitgtk:browserOptions': {
          binary: APP,
          args: ['--automation'],
        },
      },
    },
  });
}

async function waitForApp(driver, { timeoutMs = 120000 } = {}) {
  const deadline = Date.now() + timeoutMs;
  let last = '';
  while (Date.now() < deadline) {
    try {
      const probe = await driver.executeScript(
        `return { url: location.href, title: document.title,
                  rootChildren: (document.getElementById('root')?.children?.length)||0,
                  text: (document.body?.innerText||'').length };`, []);
      last = JSON.stringify(probe);
      if (probe && probe.rootChildren > 0 && probe.text > 0) return probe;
    } catch (e) { /* not ready */ }
    await sleep(1500);
  }
  throw new Error('App did not render within timeout (last: ' + last + ')');
}

let result = {};

async function main() {
  const driver = await createSession();
  result.session = driver.sessionId;
  console.log('session:', driver.sessionId);

  const ui = await waitForApp(driver);
  console.log('APP RENDERED:', JSON.stringify(ui));

  // Dump page text + all interactive controls to understand the shell.
  const info = await driver.executeScript(`(() => {
    const txt = (document.body?.innerText || '').slice(0, 3000);
    const interactive = [...document.querySelectorAll('button, a, input, [role="button"], [role="tab"], [role="menuitem"], textarea, select')]
      .map((el, i) => ({ i, tag: el.tagName, text: (el.innerText||el.value||el.getAttribute('aria-label')||'').slice(0,40), role: el.getAttribute('role') }))
      .slice(0, 120);
    return { txt, interactive };
  })()`);
  result.info = info;
  console.log('PAGE TEXT (head):\n' + (info?.txt || '(none)').slice(0, 1500));

  const png = await driver.takeScreenshot();
  require('fs').writeFileSync('/tmp/stratum-acceptance-boot.png', Buffer.from(png, 'base64'));
  console.log('screenshot saved');

  try { await driver.deleteSession(); } catch (e) {}
  require('fs').writeFileSync('/tmp/stratum-ui-info.json', JSON.stringify(result, null, 2));
}

main().catch(async (e) => {
  console.error('FATAL:', e.message);
  require('fs').writeFileSync('/tmp/stratum-ui-info.json', JSON.stringify({ fatal: e.message }, null, 2));
  process.exit(1);
});
