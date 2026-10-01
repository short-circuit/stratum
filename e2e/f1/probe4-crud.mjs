#!/usr/bin/env node
/**
 * E7.F1 probe4: live block CRUD + indent/outdent + collapse + marker editing.
 * Uses the real tauri driver session against the CLEAN fixture.
 * Methods: native key actions (Tab/Shift+Tab, Enter), click on drag handles,
 * DOM checks for collapse state, and disk-state checks for save persistence.
 */
import WebDriver from 'webdriver';
import fs from 'fs';
import crypto from 'crypto';
const APP = '/home/shrtcrct/git/stratum/target/debug/stratum-tauri';
const HOME = fs.readFileSync('/tmp/f2dir.txt', 'utf8').trim();
const VAULT_FILE = `${HOME}/StratumVault/notes/f1-editor.md`;
const OUT = '/tmp/stratum-f1-probe4';
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
fs.mkdirSync(OUT, { recursive: true });
const sha = () => crypto.createHash('sha256').update(fs.readFileSync(VAULT_FILE)).digest('hex').slice(0,16);

async function main() {
  const pre = { size: fs.statSync(VAULT_FILE).size, sha: sha(), mtimeMs: fs.statSync(VAULT_FILE).mtimeMs };
  const driver = await WebDriver.newSession({
    hostname: '127.0.0.1', port: 4444, path: '/', protocol: 'http',
    connectionRetryTimeout: 60000, connectionRetryCount: 2,
    capabilities: { alwaysMatch: { browserName: 'wry', 'wdio:enforceWebDriverClassic': true,
      'webkitgtk:browserOptions': { binary: APP, args: ['--automation'] } } },
  });
  const js = (s) => {
    let w = s;
    w = w.replace(/^\s*\(function\(\)\{([\s\S]*)\}\)\(\)\s*$/, 'return (function(){ $1 })();');
    w = /^\s*return /.test(w) ? w : `return (function(){${w}})();`;
    return driver.executeScript(w, []);
  };
  let ready = false;
  for (let i = 0; i < 90; i++) {
    try { const t = await js('return document.getElementById("root")?document.getElementById("root").children.length:0;'); if (t > 0) { ready = true; break; } } catch (e) {}
    await sleep(2000);
  }
  if (!ready) { console.log('BOOT FAIL'); await driver.deleteSession().catch(()=>{}); process.exit(1); }
  await sleep(4000);
  await js(`(function(){ history.pushState({},'', '/page/notes%2Ff1-editor.md'); dispatchEvent(new PopStateEvent('popstate')); return 'ok'; })()`);
  await sleep(6000);

  const blockState = async () => js(`return (function(){
    var outers = Array.prototype.slice.call(document.querySelectorAll('.ProseMirror [data-node-type="blockOuter"]'));
    function hasHandle(b){ return !!b.querySelector('[data-drag-handle], .bn-drag-handle, [class*="sideMenu"], [class*="SideMenu"]'); }
    return outers.map(function(b){
      return {
        text: (b.textContent||'').slice(0,50),
        hasHandle: hasHandle(b),
        collapseBtns: b.querySelectorAll('[aria-label*="collapse" i], [aria-label*="Collapse" i]').length,
        nestedOuters: b.querySelectorAll('[data-node-type="blockGroup"] [data-node-type="blockOuter"]').length,
      };
    });
  })()`);

  console.log('PRE:', JSON.stringify(pre));
  let blocks = await blockState();
  console.log('BLOCKS BEFORE EDIT:', JSON.stringify(blocks, null, 1));

  // --- 1. Append a new block at the end via Enter + type ---
  await js(`(function(){ var pm=document.querySelector('.ProseMirror[contenteditable="true"]'); if(!pm) return 'no'; pm.focus(); var sel=window.getSelection(); var r=document.createRange(); r.selectNodeContents(pm); r.collapse(false); sel.removeAllRanges(); sel.addRange(r); return 'focused'; })()`);
  await sleep(300);
  // Press Enter to create a new block, type content
  await driver.performActions([{ type: 'key', id: 'kb', actions: [
    { type: 'keyDown', value: '\uE007' }, { type: 'keyUp', value: '\uE007' }, // Enter
  ]}]);
  await sleep(400);
  const text = 'F1-SENTINEL-CRUD-001 ';
  for (const ch of text) {
    await driver.performActions([{ type: 'key', id: 'kb', actions: [
      { type: 'keyDown', value: ch }, { type: 'keyUp', value: ch },
    ]}]);
  }
  await sleep(400);
  blocks = await blockState();
  console.log('AFTER ENTER+TEXT tail blocks:', JSON.stringify(blocks.slice(-3)));

  // --- 2. Indent the new block with Tab ---
  // First move cursor to the new block (last block), then Tab
  await js(`(function(){ var pm=document.querySelector('.ProseMirror[contenteditable="true"]'); pm.focus(); var sel=window.getSelection(); var r=document.createRange(); r.selectNodeContents(pm); r.collapse(false); sel.removeAllRanges(); sel.addRange(r); return 'ok'; })()`);
  await sleep(200);
  await driver.performActions([{ type: 'key', id: 'kb', actions: [
    { type: 'keyDown', value: '\uE004' }, { type: 'keyUp', value: '\uE004' }, // Tab
  ]}]);
  await sleep(400);
  blocks = await blockState();
  console.log('AFTER TAB tail blocks:', JSON.stringify(blocks.slice(-4)));

  // --- 3. Outdent with Shift+Tab ---
  await driver.performActions([{ type: 'key', id: 'kb', actions: [
    { type: 'keyDown', value: '\uE008' }, { type: 'keyDown', value: '\uE004' },
    { type: 'keyUp', value: '\uE004' }, { type: 'keyUp', value: '\uE008' },
  ]}]);
  await sleep(400);
  blocks = await blockState();
  console.log('AFTER SHIFT+TAB tail blocks:', JSON.stringify(blocks.slice(-4)));

  // --- 4. Wait for autosave, then inspect disk ---
  await sleep(2500);
  const post = { size: fs.statSync(VAULT_FILE).size, sha: sha(), mtimeMs: fs.statSync(VAULT_FILE).mtimeMs };
  const content = fs.readFileSync(VAULT_FILE, 'utf8');
  console.log('POST:', JSON.stringify(post));
  console.log('DISK HAS SENTINEL:', content.includes('F1-SENTINEL-CRUD-001'));
  console.log('DISK TAIL:\n' + content.slice(-400));

  const png = await driver.takeScreenshot();
  fs.writeFileSync(OUT + '/probe4.png', Buffer.from(png,'base64'));
  await driver.deleteSession().catch(()=>{});
  fs.writeFileSync(OUT + '/result.json', JSON.stringify({ pre, post, sentinel: content.includes('F1-SENTINEL-CRUD-001') }, null, 2));
  process.exit(0);
}
main().catch((e) => { console.error('FATAL', e); process.exit(1); });
