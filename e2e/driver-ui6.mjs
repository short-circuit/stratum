#!/usr/bin/env node
// Stratum acceptance UI driver — v6 (comprehensive, JSON-string returns, retry-on-null)
import WebDriver from 'webdriver';
import fs from 'fs';

const APP = '/home/shrtcrct/git/stratum/target/debug/stratum-tauri';
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const OUT = '/tmp/stratum-ui-evidence-v6.json';
let EVIDENCE = [];

function ev(label, result, detail) {
  const row = { label, result, detail };
  EVIDENCE.push(row);
  console.log(`  [${result}] ${label}${detail ? ' :: ' + String(detail).slice(0,260) : ''}`);
}
function shot(driver, name) {
  return driver.takeScreenshot().then(b => { fs.writeFileSync('/tmp/' + name, Buffer.from(b, 'base64')); return '/tmp/' + name; }).catch(e => 'shot-fail:' + e.message);
}

async function createSession() {
  return WebDriver.newSession({
    hostname: '127.0.0.1', port: 4444, path: '/', protocol: 'http',
    connectionRetryTimeout: 30000, connectionRetryCount: 1,
    capabilities: { alwaysMatch: {
      browserName: 'wry', 'wdio:enforceWebDriverClassic': true,
      'webkitgtk:browserOptions': { binary: APP, args: ['--automation'] },
    } },
  });
}

// reliable executeScript: returns parsed value; retries transient nulls
async function jsv(driver, expr) {
  for (let i = 0; i < 4; i++) {
    try {
      const r = await driver.executeScript(expr, []);
      if (r !== null && r !== undefined) return r;
    } catch (e) { /* transient */ }
    await sleep(1000);
  }
  return null;
}
async function json(driver, expr) { return jsv(driver, expr); }

async function waitAppMount(driver, secs) {
  await sleep(secs * 1000);
  const url = await json(driver, 'return location.href;');
  console.log('URL:', url);
  return url;
}

async function nav(driver, name) {
  const r = await json(driver, `(function(){var els=Array.from(document.querySelectorAll('button,[role="button"],[role="menuitem"],a,span,div'));var el=els.find(function(e){return (e.innerText||'').trim()===${JSON.stringify(name)};});if(!el)return 'notfound';el.click();return 'clicked '+${JSON.stringify(name)};})()`);
  await sleep(2500);
  const url = await json(driver, 'return location.href;');
  return { r, url };
}

async function main() {
  const driver = await createSession();
  console.log('session:', driver.sessionId);

  await waitAppMount(driver, 14);

  // ── VT-01 first-launch behavior ─────────────
  // NOTE: HOME was pre-seeded so this is *not* a true first-launch. Recorded separately.

  // ── VT-04 sidebar stats ─────────────────────
  const sidebar = await json(driver, `return (document.body?.innerText||'').slice(0,700);`);
  ev('VT-04 sidebar stats (13b/4p + recent)', (sidebar||'').includes('13b') && (sidebar||'').includes('4p') ? 'PASS' : 'FAIL', (sidebar||'').slice(0,160));

  // ── JN-01 Journal opens to today ────────────
  const jurl = await json(driver, 'return location.href;');
  ev('JN-01 journal default route', (jurl||'').includes('journal') ? 'PASS' : 'FAIL', jurl);
  const jbody = await json(driver, `return (document.querySelector('[contenteditable="true"],.editor-content,main,article')?.innerText||'').slice(0,150);`);
  // placeholder check from boot shot; keep minimal
  const jtxt = await json(driver, `return (document.body?.innerText||'').includes('Enter text or type') ? 'placeholder-present' : 'no-placeholder';`);
  ev('JN-01 journal placeholder', (jtxt||'')==='placeholder-present' ? 'PASS' : 'CHECK', jtxt);
  await shot(driver, 'v6-journal.png');

  // ── Open a real note via RECENT ─────────────
  const openNote = await nav(driver, 'welcome');
  ev('open-welcome note', (openNote.r||'').includes('clicked') ? 'OPENED' : 'NOT-FOUND', JSON.stringify(openNote));
  await sleep(2500);
  const noteBody = await json(driver, `return (document.body?.innerText||'').slice(0,900);`);
  ev('welcome-note content (wiki-link to Alpha)', (noteBody||'').includes('Alpha') ? 'PASS' : 'CHECK', (noteBody||'').slice(0,200));
  await shot(driver, 'v6-welcome-note.png');

  // ── ED-07 type into editor → auto-save status ──
  const typed = await json(driver, `(function(){var el=document.querySelector('[contenteditable="true"]');if(!el)return 'no-editor';el.focus();var s=window.getSelection();s.selectAllChildren(el);document.execCommand('delete',false);document.execCommand('insertText',false,'Acceptance check line\\n');return 'typed:'+el.innerText.slice(-40);})()`);
  ev('ED-07 type in editor', (typed||'').startsWith('typed') ? 'TYPED' : 'CHECK', typed);
  await sleep(2500);
  const status = await json(driver, `return (document.body?.innerText||'').includes('Saved') ? 'saved-shown' : (document.body?.innerText||'').includes('Saving') ? 'saving-shown' : 'none';`);
  ev('ED-07 auto-save indicator', (status||'')!=='none' ? 'PASS' : 'FAIL', status);

  try { await driver.deleteSession(); } catch(e){}
  fs.writeFileSync(OUT, JSON.stringify(EVIDENCE, null, 2));
  console.log('evidence ->', OUT);
}
main().catch(async e => {
  console.error('FATAL', e.message);
  fs.writeFileSync(OUT, JSON.stringify({ fatal: e.message, ev: EVIDENCE }, null, 2));
  process.exit(1);
});
