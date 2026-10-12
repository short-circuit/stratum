#!/usr/bin/env node
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
const exec = (fn) => driver.executeScript(`return (${fn.toString()})()`, []);

for (let i = 0; i < 60; i++) {
  try {
    const p = await exec(() => ({ c: document.getElementById('root')?.children?.length || 0, b: (document.body?.innerText || '').length }));
    if (p.c > 0 && p.b > 100) break;
  } catch (e) {}
  await sleep(1000);
}
await exec(() => { [...document.querySelectorAll('[role="button"],button,[role="menuitem"],li')].find(e=>(e.textContent||'').trim()==='Journal')?.click(); return true; });
await sleep(4000);

console.log('After nav: url =', await exec(() => location.href));
console.log('Aria labels now:', await exec(() => [...document.querySelectorAll('button,[role="button"]')].map(e => e.getAttribute('aria-label')).filter(Boolean)));
console.log('record present:', await exec(() => !!([...document.querySelectorAll('button,[role="button"]')].find(e=>(e.getAttribute('aria-label')||'').toLowerCase().includes('sprachnotiz aufnehmen')))));
console.log('stop present:', await exec(() => !!([...document.querySelectorAll('button,[role="button"]')].find(e=>(e.getAttribute('aria-label')||'').toLowerCase().includes('stoppen & speichern')))));

// click record + immediate checks
const clicked = await exec(() => { const b = [...document.querySelectorAll('button,[role="button"]')].find(e=>(e.getAttribute('aria-label')||'').toLowerCase().includes('sprachnotiz aufnehmen')); if (b) { b.click(); return true; } return false; });
console.log('clicked record:', clicked);
await sleep(3000);
console.log('after click, stop present:', await exec(() => !!([...document.querySelectorAll('button,[role="button"]')].find(e=>(e.getAttribute('aria-label')||'').toLowerCase().includes('stoppen & speichern')))));
console.log('after click, cancel present:', await exec(() => !!([...document.querySelectorAll('button,[role="button"]')].find(e=>(e.getAttribute('aria-label')||'').toLowerCase().includes('abbrechen')))));
console.log('body after click:', (await exec(() => (document.body.innerText||'').slice(0,400))).replace(/\s+/g,' '));

await sleep(4000);
// stop
const stopped = await exec(() => { const b = [...document.querySelectorAll('button,[role="button"]')].find(e=>(e.getAttribute('aria-label')||'').toLowerCase().includes('stoppen & speichern')); if (b) { b.click(); return true; } return false; });
console.log('stopped:', stopped);
// dump recording path presence: check body for 'Transkription'/'working' over time
for (let k = 0; k < 8; k++) {
  await sleep(3000);
  const b = await exec(() => (document.body.innerText||'').slice(-600));
  console.log(`t+${3*(k+1)}s tail:`, JSON.stringify(String(b).replace(/\s+/g,' ').slice(-250)));
}
await driver.deleteSession();
