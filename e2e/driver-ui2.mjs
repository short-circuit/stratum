#!/usr/bin/env node
// Stratum acceptance UI driver — iterative probe harness (v2)
import WebDriver from 'webdriver';
import fs from 'fs';

const APP = '/home/shrtcrct/git/stratum/target/debug/stratum-tauri';
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

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
        'webkitgtk:browserOptions': { binary: APP, args: ['--automation'] },
      },
    },
  });
}

async function waitForApp(driver, { timeoutMs = 120000 } = {}) {
  const deadline = Date.now() + timeoutMs;
  let last = '';
  while (Date.now() < deadline) {
    try {
      const probe = await driver.executeScript({
        script: `return { url: location.href, title: document.title,
                rootChildren: (document.getElementById('root')?.children?.length)||0,
                text: (document.body?.innerText||'').length };`,
        args: [],
      });
      last = JSON.stringify(probe);
      if (probe && probe.rootChildren > 0 && probe.text > 0) return probe;
    } catch (e) { last = 'ERR ' + e.message; }
    await sleep(1500);
  }
  throw new Error('App did not render (last: ' + last + ')');
}

let log = [];
async function probe(driver, label) {
  const info = await driver.executeScript({
    script: `(() => {
      const txt = (document.body?.innerText || '').slice(0, 4000);
      const interactive = [...document.querySelectorAll('button, a, input, [role="button"], [role="tab"], [role="menuitem"], textarea, [role="checkbox"], [contenteditable="true"]')]
        .map((el, i) => ({ i, tag: el.tagName, role: el.getAttribute('role'),
          text: (el.innerText||el.value||el.getAttribute('aria-label')||el.getAttribute('placeholder')||'').slice(0,50) }))
        .slice(0, 150);
      return { url: location.href, txt, interactive };
    })()`,
    args: [],
  });
  log.push({ label, at: new Date().toISOString(), ...info });
  console.log('\n===== ' + label + ' =====');
  console.log('URL:', info?.url);
  console.log('TEXT:', (info?.txt || '(none)').slice(0, 1200));
  console.log('INTERACTIVE (' + (info?.interactive?.length || 0) + '):');
  for (const el of (info?.interactive || []).slice(0, 60)) {
    console.log('  [' + el.i + '] <' + el.tag + '> role=' + el.role + ' ' + JSON.stringify(el.text));
  }
  return info;
}

async function main() {
  const driver = await createSession();
  console.log('session:', driver.sessionId);
  const ui = await waitForApp(driver);
  console.log('APP RENDERED:', JSON.stringify(ui));

  await probe(driver, 'boot');

  const png = await driver.takeScreenshot();
  fs.writeFileSync('/tmp/stratum-acceptance-boot.png', Buffer.from(png, 'base64'));
  console.log('screenshot -> /tmp/stratum-acceptance-boot.png');

  try { await driver.deleteSession(); } catch (e) {}
  fs.writeFileSync('/tmp/stratum-ui-log.json', JSON.stringify(log, null, 2));
  console.log('log saved');
}

main().catch(async (e) => {
  console.error('FATAL:', e.message);
  fs.writeFileSync('/tmp/stratum-ui-log.json', JSON.stringify({ fatal: e.message, log }, null, 2));
  process.exit(1);
});
