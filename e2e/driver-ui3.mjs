#!/usr/bin/env node
// Stratum acceptance UI driver — v3 (positional executeScript)
import WebDriver from 'webdriver';
import fs from 'fs';

const APP = '/home/shrtcrct/git/stratum/target/debug/stratum-tauri';
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function createSession() {
  return WebDriver.newSession({
    hostname: '127.0.0.1', port: 4444, path: '/', protocol: 'http',
    connectionRetryTimeout: 30000, connectionRetryCount: 1,
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
      const probe = await driver.executeScript(
        `return { url: location.href, title: document.title,
                rootChildren: (document.getElementById('root')?.children?.length)||0,
                text: (document.body?.innerText||'').length };`, []);
      last = JSON.stringify(probe);
      if (probe && probe.rootChildren > 0 && probe.text > 0) return probe;
    } catch (e) { last = 'ERR ' + e.message; }
    await sleep(1500);
  }
  throw new Error('App did not render (last: ' + last + ')');
}

let log = [];
async function probe(driver, label) {
  const info = await driver.executeScript(`(() => {
    const txt = (document.body?.innerText || '').slice(0, 5000);
    const interactive = [...document.querySelectorAll('button, a, input, [role="button"], [role="tab"], [role="menuitem"], textarea, [role="checkbox"], [contenteditable="true"]')]
      .map((el, i) => ({ i, tag: el.tagName, role: el.getAttribute('role'),
        text: (el.innerText||el.value||el.getAttribute('aria-label')||el.getAttribute('placeholder')||'').replace(/\\s+/g,' ').trim().slice(0,50) }))
      .slice(0, 200);
    return { url: location.href, txt, interactive };
  })()`, []);
  log.push({ label, at: new Date().toISOString(), ...info });
  console.log('\n===== ' + label + ' =====');
  console.log('URL:', info?.url);
  console.log('TEXT:', (info?.txt || '(none)').slice(0, 1400));
  console.log('INTERACTIVE (' + (info?.interactive?.length || 0) + '):');
  for (const el of (info?.interactive || []).slice(0, 80)) {
    console.log('  [' + el.i + '] <' + el.tag + '> role=' + el.role + ' ' + JSON.stringify(el.text));
  }
  return info;
}

async function click(driver, index) {
  return driver.executeScript(`(() => {
    const els = [...document.querySelectorAll('button, a, input, [role="button"], [role="tab"], [role="menuitem"], textarea, [role="checkbox"], [contenteditable="true"]')];
    const el = els[${index}]; if (!el) return 'not-found';
    el.click();
    return 'clicked[' + ${index} + '] ' + (el.innerText || el.tagName || el.value || '').slice(0,40).replace(/\\s+/g,' ') + ' @ ' + el.tagName;
  })()`, []);
}

async function type(driver, selector, text) {
  return driver.executeScript(`(() => {
    const el = document.querySelector(${JSON.stringify(selector)});
    if (!el) return 'no-el ' + ${JSON.stringify(selector)};
    el.focus();
    const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value')?.set
      || Object.getOwnPropertyDescriptor(window.HTMLTextAreaElement.prototype, 'value')?.set;
    if (setter) setter.call(el, ${JSON.stringify(text)});
    el.dispatchEvent(new Event('input', { bubbles: true }));
    el.dispatchEvent(new Event('change', { bubbles: true }));
    return 'typed';
  })()`, []);
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
