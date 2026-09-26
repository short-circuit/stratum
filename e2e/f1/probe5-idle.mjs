#!/usr/bin/env node
/**
 * E7.F1 probe5: IDLE-OPEN ROUND-TRIP.
 * Open the page, do NOT type. Wait >debounce. Does the disk file change?
 * Checks: block ids preserved? trailing [[ appended? content byte-stable?
 */
import WebDriver from 'webdriver';
import fs from 'fs';
import crypto from 'crypto';
const APP = '/home/shrtcrct/git/stratum/target/debug/stratum-tauri';
const HOME = fs.readFileSync('/tmp/f2dir.txt', 'utf8').trim();
const VAULT_FILE = `${HOME}/StratumVault/notes/f1-editor.md`;
const OUT = '/tmp/stratum-f1-probe5';
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
fs.mkdirSync(OUT, { recursive: true });
const sha = () => crypto.createHash('sha256').update(fs.readFileSync(VAULT_FILE)).digest('hex').slice(0,16);

async function main() {
  // Reset fixture to pristine from the source-of-truth
  fs.copyFileSync('/tmp/stratum-f1-1789776002/StratumVault/notes/f1-editor.md', VAULT_FILE);
  const preRaw = fs.readFileSync(VAULT_FILE, 'utf8');
  const pre = { size: preRaw.length, sha: sha(), mtimeMs: fs.statSync(VAULT_FILE).mtimeMs, hasTrailingBracket: preRaw.trimEnd().endsWith('[[') };

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

  // Navigate to the page WITHOUT typing
  await js(`(function(){ history.pushState({},'', '/page/notes%2Ff1-editor.md'); dispatchEvent(new PopStateEvent('popstate')); return 'ok'; })()`);
  await sleep(7000); // let it open + load, no edits

  // Sample the editor text
  const edText = await js(`return (function(){ var pm=document.querySelector('.ProseMirror[contenteditable="true"]'); return pm?pm.innerText.slice(-200):'no-editor'; })()`);
  console.log('EDITOR TAIL (no edit):\n' + edText);

  // Wait past the autosave debounce + save
  await sleep(3000);
  const idleRaw = fs.readFileSync(VAULT_FILE, 'utf8');
  const idle = { size: idleRaw.length, sha: sha(), hasTrailingBracket: idleRaw.trimEnd().endsWith('[['), tail: idleRaw.slice(-120) };
  console.log('PRE: ', JSON.stringify({ size: pre.size, sha: pre.sha, hasTrailingBracket: pre.hasTrailingBracket }));
  console.log('IDLE:', JSON.stringify({ size: idle.size, sha: idle.sha, hasTrailingBracket: idle.hasTrailingBracket, tail: idle.tail }));

  // mermaid UUID stability
  const mermaidIdPre = (preRaw.match(/```mermaid[\s\S]*?\.id: ([0-9a-f-]{36})/) || [])[1] || 'none';
  const mermaidIdIdle = (idleRaw.match(/```mermaid[\s\S]*?\.id: ([0-9a-f-]{36})/) || [])[1] || 'none';
  console.log('MERMAID ID pre:', mermaidIdPre, 'idle:', mermaidIdIdle, 'stable:', mermaidIdPre === mermaidIdIdle);

  await driver.deleteSession().catch(()=>{});
  fs.writeFileSync(OUT + '/result.json', JSON.stringify({
    idleRewrite: idle.sha !== pre.sha,
    trailingBracketAdded: !pre.hasTrailingBracket && idle.hasTrailingBracket,
    mermaidIdStable: mermaidIdPre === mermaidIdIdle,
    pre, idle, editorTail: edText,
  }, null, 2));
  console.log('RESULT JSON written:', OUT + '/result.json');
  process.exit(0);
}
main().catch((e) => { console.error('FATAL', e); process.exit(1); });
