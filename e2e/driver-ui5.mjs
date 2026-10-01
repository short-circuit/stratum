#!/usr/bin/env node
// Stratum acceptance UI driver — v5 (full feature walk, single-statement scripts)
import WebDriver from 'webdriver';
import fs from 'fs';

const APP = '/home/shrtcrct/git/stratum/target/debug/stratum-tauri';
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const OUT = '/tmp/stratum-ui-evidence-v5.json';
let EVIDENCE = [];

function ev(label, result, detail) {
  const row = { label, result, detail };
  EVIDENCE.push(row);
  console.log(`  [${result}] ${label}${detail ? ' :: ' + String(detail).slice(0,300) : ''}`);
}

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

// single-statement JS helper
async function js(driver, expr) {
  return driver.executeScript(expr, []);
}
async function jsArr(driver, expr) {
  return driver.executeScript(expr, []);
}

async function nav(driver, name) {
  // Click sidebar nav by text
  return js(driver, `(() => {
    const els = Array.from(document.querySelectorAll('button, [role="button"], [role="menuitem"], a, span'));
    const el = els.find(e => (e.innerText||'').trim() === ${JSON.stringify(name)});
    if (!el) return 'nav-not-found:'+${JSON.stringify(name)};
    el.click(); return 'nav-clicked:'+${JSON.stringify(name)};
  })()`);
}

async function main() {
  const driver = await createSession();
  console.log('session:', driver.sessionId);
  await sleep(12000);   // let app fully mount
  console.log('URL:', await js(driver, 'return location.href;'));
  console.log('TITLE:', await js(driver, 'return document.title;'));

  // ── VT-01/VT-04 boot state ─────────────
  const sidebar = await js(driver, `return (document.body?.innerText||'').slice(0,600);`);
  ev('VT-04 sidebar-stats', sidebar.includes('13b') ? 'PASS' : 'CHECK', sidebar.slice(0,120));
  ev('VT-04 nav-items', /Journal\nGraph\nKanban\nSearch\nQuery\nAsk Notes\nTemplates\nFlashcards\nWhiteboards\nPlugins\nSettings/.test(sidebar) ? 'PASS' : 'FAIL', sidebar.slice(0,200));

  // ── Click through nav (Graph, Kanban, Search, Query...) ──
  for (const item of ['Graph', 'Kanban', 'Search', 'Query']) {
    const r = await nav(driver, item);
    console.log('nav', item, '=>', r);
    await sleep(1500);
    const body = await js(driver, `return (document.body?.innerText||'').slice(0,800);`);
    ev('NAV-'+item, body.length > 0 ? 'OPENED' : 'BLANK', body.slice(0,150));
  }

  await driver.takeScreenshot().then(b => fs.writeFileSync('/tmp/shot-graph.png', Buffer.from(b,'base64')));

  try { await driver.deleteSession(); } catch(e){}
  fs.writeFileSync(OUT, JSON.stringify(EVIDENCE, null, 2));
  console.log('evidence ->', OUT);
}
main().catch(async e => {
  console.error('FATAL', e.message);
  fs.writeFileSync(OUT, JSON.stringify({fatal:e.message, ev:EVIDENCE}, null, 2));
  process.exit(1);
});
