#!/usr/bin/env node
/**
 * Verify the edit-then-save corruption fix (ED-07 / CRITICAL) on the real
 * binary against the real probe vault (/tmp/stratum-home/StratumVault).
 *
 * v2: navigates directly to the note via the router (pushState), then applies
 * a real edit through ProseMirror's input pipeline (execCommand insertText),
 * because the app is a BrowserRouter SPA and the sidebar synthetic click was
 * unreliable under WebDriver.
 */
import WebDriver from 'webdriver';
import fs from 'fs';
import crypto from 'crypto';

const APP = '/home/shrtcrct/git/stratum/target/debug/stratum-tauri';
const VAULT_FILE = '/tmp/stratum-home/StratumVault/notes/alpha-project.md';
const OUT = '/tmp/stratum-vfy-edit';
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const sha = () => crypto.createHash('sha256').update(fs.readFileSync(VAULT_FILE)).digest('hex').slice(0, 16);

async function main() {
  fs.mkdirSync(OUT, { recursive: true });
  const pre = { size: fs.statSync(VAULT_FILE).size, sha: sha(), mtimeMs: fs.statSync(VAULT_FILE).mtimeMs };
  console.log('PRE:', JSON.stringify(pre));

  const driver = await WebDriver.newSession({
    hostname: '127.0.0.1', port: 4444, path: '/', protocol: 'http',
    connectionRetryTimeout: 60000, connectionRetryCount: 2,
    capabilities: { alwaysMatch: { browserName: 'wry', 'wdio:enforceWebDriverClassic': true,
      'webkitgtk:browserOptions': { binary: APP, args: ['--automation'] } } },
  });
  const js = (s) => {
    // WebKitWebDriver only serializes a script's completion value when the
    // script has an explicit top-level `return`. For our IIFE-bodied scripts,
    // convert `(function(){ BODY })()` → `return (function(){ BODY })();` so
    // the inner return value actually reaches us (the driver otherwise returns null).
    let wrapped = s;
    wrapped = wrapped.replace(/^\s*\(function\(\)\{([\s\S]*)\}\)\(\)\s*$/, 'return (function(){ $1 })();');
    wrapped = /^\s*return /.test(wrapped) ? wrapped : `return (function(){${wrapped}})();`;
    return driver.executeScript(wrapped, []);
  };

  let ready = false;
  for (let i = 0; i < 60; i++) {
    try { const t = await js('return document.getElementById("root")?document.getElementById("root").children.length:0;'); if (t > 0) { ready = true; break; } } catch (e) {}
    await sleep(2000);
  }
  if (!ready) { console.log('BOOT FAIL'); await driver.deleteSession().catch(()=>{}); process.exit(1); }
  await sleep(4000);

  // Navigate directly to the note (SPA route). alpha-project is at notes/alpha-project.md.
  const nav = await js(`(function(){
    try {
      history.pushState({}, '', '/page/notes%2Falpha-project.md');
      dispatchEvent(new PopStateEvent('popstate'));
      return 'pushed';
    } catch (e) { return 'err:' + e.message; }
  })()`);
  console.log('nav:', nav);
  await sleep(4000);
  console.log('URL now:', await js('return location.pathname;'));
  const hasEditor = await js(`return !!document.querySelector('.ProseMirror[contenteditable="true"]');`);
  console.log('editor present:', hasEditor);

  // Apply a REAL edit: focus the editor, place the cursor at the end, then type
  // via WebDriver native key actions (ProseMirror consumes real beforeinput/input).
  const ins = await js(`(function(){
    var pm = document.querySelector('.ProseMirror[contenteditable="true"]');
    if(!pm) { window.__editProbe='no-editor'; return 'no-editor'; }
    pm.focus();
    var sel = window.getSelection(); var range = document.createRange();
    range.selectNodeContents(pm); range.collapse(false);
    sel.removeAllRanges(); sel.addRange(range);
    window.__editProbe='focused';
    return 'focused';
  })()`);
  console.log('focus:', ins);
  await sleep(200);
  // Type "ZZ-SENTINEL-9087 " via native key events.
  const text = 'ZZ-SENTINEL-9087 ';
  const keyActions = [];
  for (const ch of text) {
    keyActions.push(
      { type: 'keyDown', value: ch },
      { type: 'keyUp', value: ch },
    );
  }
  await driver.performActions([{ type: 'key', id: 'kb', actions: keyActions }]);
  console.log('typed sentinel');
  await sleep(400);
  const editorTail = await js(`return (document.querySelector('.ProseMirror')||{}).innerText ? document.querySelector('.ProseMirror').innerText.slice(-140) : 'no-editor';`);
  console.log('EDITOR TAIL:', JSON.stringify(editorTail));

  // Sample save-debug flags.
  for (const [label, wait] of [['t+0.3', 300], ['t+0.8', 500], ['t+1.5', 700], ['t+3', 1500]]) {
    await sleep(wait);
    try {
      const v = await js(`return (function(){var d=window.__saveDebug||{};return 'called='+(d.called?1:0)+' saved='+(d.saved?1:0)+' skipped='+(d.skipped?1:0);})();`);
      console.log(`[${label}] ${v}`);
    } catch (e) { console.log(`[${label}] ERR ${e.message}`); }
  }

  const post = { size: fs.statSync(VAULT_FILE).size, sha: sha(), mtimeMs: fs.statSync(VAULT_FILE).mtimeMs };
  const postContent = fs.readFileSync(VAULT_FILE, 'utf8');
  console.log('POST:', JSON.stringify(post));
  console.log('POST FILE TAIL:\n' + postContent.slice(-260));

  const png = await driver.takeScreenshot();
  fs.writeFileSync(OUT + '/edit.png', Buffer.from(png, 'base64'));
  await driver.deleteSession().catch(()=>{});

  const okSentinel = postContent.includes('ZZ-SENTINEL-9087');
  const okEmbed = postContent.includes('{{embed [[Alpha Project]]}}');
  const noDouble = !postContent.includes('[[[[Alpha Project]]]]');
  const noMangle = !postContent.includes('.question: :') && !postContent.includes(']: true');
  const saved = post.mtimeMs !== pre.mtimeMs;

  const checks = {
    'file WAS rewritten (edit persisted)': saved,
    'sentinel text persisted to disk': okSentinel,
    'embed construct intact after save': okEmbed,
    'no doubled-bracket corruption': noDouble,
    'flashcard :: not mangled': noMangle,
  };
  let all = true;
  for (const [k, v] of Object.entries(checks)) { console.log(`${v ? 'PASS' : 'FAIL'}  ${k}`); if (!v) all = false; }
  fs.writeFileSync(OUT + '/result.json', JSON.stringify({ pre, post, checks, editorTail, tail: postContent.slice(-260) }, null, 2));
  console.log(all ? 'RESULT: PASS' : 'RESULT: FAIL');
  process.exit(all ? 0 : 1);
}
main().catch((e) => { console.error('FATAL', e.message); process.exit(1); });
