#!/usr/bin/env node
/**
 * Precise real voice-capture E2E (E9.F4) — German-locale vault.
 * Journal header has aria-label "Sprachnotiz aufnehmen" (record).
 * After stop: "Stoppen & speichern" (stop), "Abbrechen" (cancel).
 * Memo marker: "Sprachnotiz" (in voice memo block).
 *
 * Audio: cpal records via ALSA. On headless envs use a null-plugin config so
 * the stream yields real samples: export ALSA_CONFIG_PATH=e2e/alsa-null.conf
 * (see e2e/alsa-null-setup.sh). Requires tauri-driver on :4444 and the app
 * binary built with custom-protocol (see e2e/harness).
 */
import WebDriver from 'webdriver';
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const appBinary = '/home/shrtcrct/git/stratum/.worktrees/t_e1982a1b/target/debug/stratum-tauri';
const driver = await WebDriver.newSession({
  hostname: '127.0.0.1', port: 4444, path: '/', protocol: 'http',
  connectionRetryTimeout: 30000, connectionRetryCount: 1,
  capabilities: { alwaysMatch: { browserName: 'wry',
    'wdio:enforceWebDriverClassic': true,
    'webkitgtk:browserOptions': { binary: appBinary, args: ['--automation'] } } },
});
console.log('Session:', driver.sessionId);
let okC = 0, badC = 0;
const ok = (n) => { okC++; console.log('  ✅', n); };
const bad = (n, d) => { badC++; console.log('  ❌', n, d || ''); };
const exec = (fn) => driver.executeScript(`return (${fn.toString()})()`, []);

for (let i = 0; i < 60; i++) {
  try {
    const p = await exec(() => ({ c: document.getElementById('root')?.children?.length || 0, b: (document.body?.innerText || '').length }));
    if (p.c > 0 && p.b > 100) break;
  } catch (e) {}
  await sleep(1000);
}
// nav to journal
await exec(() => { [...document.querySelectorAll('[role="button"],button,[role="menuitem"],li')].find(e=>(e.textContent||'').trim()==='Journal')?.click(); return true; });
await sleep(4000);

// click record
const rec = await exec(() => {
  const b = [...document.querySelectorAll('button,[role="button"]')].find(e=>(e.getAttribute('aria-label')||'').toLowerCase().includes('sprachnotiz aufnehmen'));
  if (b) { b.click(); return true; }
  return false;
});
if (rec) ok('voice-capture button clicked (Sprachnotiz aufnehmen)'); else bad('voice-capture button clicked');

// wait for stop button
let stopSeen = false;
for (let i = 0; i < 20; i++) {
  await sleep(500);
  stopSeen = await exec(() => !!([...document.querySelectorAll('button,[role="button"]')].find(e=>(((e.getAttribute('aria-label')||'')+' '+(e.textContent||'')).toLowerCase().includes('stoppen & speichern')))));
  if (stopSeen) break;
}
if (stopSeen) ok('recording state (Stop & save / Stoppen & speichern)'); else bad('recording state');

await sleep(4000); // record 4s silence

// stop
const stopped = await exec(() => {
  const b = [...document.querySelectorAll('button,[role="button"]')].find(e=>(((e.getAttribute('aria-label')||'')+' '+(e.textContent||'')).toLowerCase().includes('stoppen & speichern')));
  if (b) { b.click(); return true; }
  return false;
});
if (stopped) ok('stop tapped'); else bad('stop tapped');

// wait for memo: a NEW "Voice Memo" heading must appear (count increases).
// Do NOT match the FAB label "Sprachnotiz aufnehmen" (always contains
// 'sprachnotiz'), and require a count increase because earlier runs may have
// already inserted memos into today's journal.
let memoSeen = false; let bodyNow = '';
const countMemos = (b) => String(b).toLowerCase().split('voice memo').length - 1;
const baseline = await exec(() => document.body.innerText || '');
let baselineCount = countMemos(baseline);
for (let i = 0; i < 150; i++) {
  await sleep(2000);
  bodyNow = await exec(() => document.body.innerText || '');
  if (countMemos(bodyNow) > baselineCount) { memoSeen = true; break; }
}
if (memoSeen) ok('memo block landed in journal after real transcription', `body ${String(bodyNow).length} chars`);
else bad('memo block landed', String(bodyNow).slice(0, 300));

// wait for the FAB to return to idle so the second recording can start.
let fabBack = false;
for (let i = 0; i < 30; i++) {
  await sleep(500);
  fabBack = await exec(() => !!([...document.querySelectorAll('button,[role="button"]')].find(e=>(e.getAttribute('aria-label')||'').toLowerCase().includes('sprachnotiz aufnehmen'))));
  if (fabBack) break;
}
if (fabBack) ok('capture button returned to idle (repeatable)'); else bad('capture button returned to idle');

// cancel path
const rec2 = await exec(() => {
  const b = [...document.querySelectorAll('button,[role="button"]')].find(e=>(e.getAttribute('aria-label')||'').toLowerCase().includes('sprachnotiz aufnehmen'));
  if (b) { b.click(); return true; } return false;
});
await sleep(2500);
const rec2Seen = await exec(() => !!([...document.querySelectorAll('button,[role="button"]')].find(e=>(((e.getAttribute('aria-label')||'')+' '+(e.textContent||'')).toLowerCase().includes('stoppen & speichern')))));
if (rec2Seen) {
  ok('second recording started');
  const cancelled = await exec(() => {
    const b = [...document.querySelectorAll('button,[role="button"]')].find(e=>(e.getAttribute('aria-label')||'').toLowerCase()==='abbrechen' || (e.getAttribute('aria-label')||'').toLowerCase().includes('abbrechen'));
    if (b) { b.click(); return true; } return false;
  });
  await sleep(1500);
  if (cancelled) ok('cancel tapped'); else bad('cancel tapped');
} else bad('second recording started');

console.log(`\n📊 ${okC} passed, ${badC} failed`);
await driver.deleteSession();
process.exit(badC > 0 ? 1 : 0);
