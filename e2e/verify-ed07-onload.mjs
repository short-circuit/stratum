#!/usr/bin/env node
/**
 * Verify ED-07 / CRITICAL fix: opening a note and waiting past the autosave
 * debounce must NOT rewrite the on-disk .md file (previous behaviour wrote
 * lossy `serialize_blocks` output on zero user edits).
 *
 * Method:
 *   1. Snapshot pristine alpha-project.md + welcome.md hashes/sizes.
 *   2. Boot the real binary against an isolated HOME+vault (auto-resolved
 *      ~/StratumVault).
 *   3. Open alpha-project.md; wait 5s (>= debounce 500ms + flush slack).
 *   4. Sample window.__saveDebug at intervals to confirm no save fired
 *      (skipped=true is the vector we WANT surfaced, i.e. the guard worked).
 *   5. Compare on-disk files: must be byte-identical to snapshot.
 *
 * The vault dir is passed in argv[2] (it is copied to $HOME/StratumVault and
 * the HOME env is remounted accordingly; the app resolves the vault from HOME).
 */
import WebDriver from 'webdriver';
import fs from 'fs';
import path from 'path';
import crypto from 'crypto';

const APP = process.env.HARNESS_APP || '/home/shrtcrct/git/stratum/target/debug/stratum-tauri';
// The vault source (a pristine fixture) to copy into HOME/StratumVault.
const VAULT_SRC = process.env.VAULT_SRC || '/tmp/stratum-acceptance-vault';
const HOME_DIR = process.env.PROBE_HOME || '/tmp/stratum-vfy-ed07-home';
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const sha = (p) => crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex').slice(0, 16);

function copyDir(src, dst) {
  fs.rmSync(dst, { recursive: true, force: true });
  fs.mkdirSync(dst, { recursive: true });
  const walk = (cur, dir) => {
    for (const e of fs.readdirSync(cur, { withFileTypes: true })) {
      const s = path.join(cur, e.name);
      const d = path.join(dir, e.name);
      if (e.isDirectory()) { fs.mkdirSync(d, { recursive: true }); walk(s, d); }
      else fs.copyFileSync(s, d);
    }
  };
  walk(src, dst);
}

async function main() {
  const vaultFile = path.join(HOME_DIR, 'StratumVault', 'notes', 'alpha-project.md');
  const notes = path.join(HOME_DIR, 'StratumVault', 'notes');
  copyDir(VAULT_SRC, path.join(HOME_DIR, 'StratumVault'));

  // Snapshot pristine files BEFORE the app touches them.
  const snap = {};
  for (const f of fs.readdirSync(notes)) {
    const p = path.join(notes, f);
    if (f.endsWith('.md')) snap[f] = { sha: sha(p), size: fs.statSync(p).size };
  }
  console.log('PRISTINE SNAPSHOT:', JSON.stringify(snap, null, 2));

  const driver = await WebDriver.newSession({
    hostname: '127.0.0.1', port: 4444, path: '/', protocol: 'http',
    connectionRetryTimeout: 60000, connectionRetryCount: 2,
    capabilities: { alwaysMatch: { browserName: 'wry', 'wdio:enforceWebDriverClassic': true,
      'webkitgtk:browserOptions': { binary: APP, args: ['--automation'], env: { HOME: HOME_DIR } } } },
  });
  const js = (s) => driver.executeScript(s, []);

  let ready = false;
  for (let i = 0; i < 60; i++) {
    try { const t = await js('return document.getElementById("root")?document.getElementById("root").children.length:0;'); if (t > 0) { ready = true; break; } } catch (e) {}
    await sleep(2000);
  }
  if (!ready) { console.log('BOOT FAIL'); await driver.deleteSession().catch(()=>{}); process.exit(1); }
  await sleep(4000);

  // Open alpha-project via the sidebar link.
  const clicked = await js(`(function(){var els=Array.from(document.querySelectorAll('*'));for(var i=0;i<els.length;i++){var e=els[i];if(e.children.length)continue;var t=(e.textContent||'').trim();if(t==='alpha-project'){e.click();return 'clicked';}}return 'none';})()`);
  console.log('open alpha-project:', clicked);
  await sleep(4000);
  console.log('URL now:', await js('return location.href;'));

  // Sample debug flags + editor presence over 6s.
  const samples = [];
  for (const [label, wait] of [['t+0.5', 500], ['t+1.5', 1000], ['t+3.0', 1500], ['t+4.5', 1500]]) {
    await sleep(wait);
    try {
      const v = await js(`return (function(){var d=window.__saveDebug||{};var pm=document.querySelector('.ProseMirror[contenteditable="true"]');return 'called='+(d.called?1:0)+' saved='+(d.saved?1:0)+' skipped='+(d.skipped?1:0)+' editor='+(pm?1:0);})();`);
      samples.push(`${label}: ${v}`);
    } catch (e) { samples.push(`${label}: ERR ${e.message}`); }
  }
  console.log('SAVE-DEBUG SAMPLES:\n' + samples.join('\n'));

  // Wait a bit more, then compare on-disk.
  await sleep(2500);
  const after = {};
  for (const f of fs.readdirSync(notes)) {
    if (!f.endsWith('.md')) continue;
    const p = path.join(notes, f);
    after[f] = { sha: sha(p), size: fs.statSync(p).size };
  }
  console.log('AFTER  SNAPSHOT:', JSON.stringify(after, null, 2));

  let unchanged = true;
  for (const f of Object.keys(snap)) {
    const a = snap[f], b = after[f];
    const same = b && a.sha === b.sha && a.size === b.size;
    console.log(`${same ? 'PASS' : 'FAIL'}  on-disk ${f} unchanged  (${a.sha}/${a.size} vs ${b ? b.sha + '/' + b.size : 'MISSING'})`);
    if (!same) unchanged = false;
  }

  const png = await driver.takeScreenshot();
  fs.mkdirSync('/tmp/stratum-vfy-ed07', { recursive: true });
  fs.writeFileSync('/tmp/stratum-vfy-ed07/final.png', Buffer.from(png, 'base64'));
  console.log('screenshot /tmp/stratum-vfy-ed07/final.png');

  await driver.deleteSession().catch(()=>{});
  console.log(unchanged ? 'RESULT: PASS (pristine vault unmodified on open)' : 'RESULT: FAIL (vault file(s) changed on open)');
  process.exit(unchanged ? 0 : 1);
}
main().catch((e) => { console.error('FATAL', e.message); process.exit(1); });
