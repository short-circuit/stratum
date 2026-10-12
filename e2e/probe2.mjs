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
for (let i = 0; i < 60; i++) {
  try {
    const p = await driver.executeScript(`return {r:!!document.getElementById('root'), c:document.getElementById('root')?.children?.length||0, b:(document.body?.innerText||'').length};`, []);
    if (p && p.r && p.c > 0 && p.b > 100) break;
  } catch (e) {}
  await sleep(1000);
}
// Navigate to journal
await driver.executeScript(`[...document.querySelectorAll('[role="button"],button,[role="menuitem"],li')].find(e=>(e.textContent||'').trim()==='Journal')?.click(); return true;`, []);
await sleep(4000);
const dump = await driver.executeScript(`return {
  url: location.href,
  body: (document.body?.innerText||'').slice(0,700),
  allAria: [...document.querySelectorAll('button,[role="button"]')].map(e=>e.getAttribute('aria-label')).filter(Boolean),
  allTitles: [...document.querySelectorAll('[title]')].map(e=>e.getAttribute('title')).filter(Boolean),
  micIcons: document.querySelectorAll('svg, .MuiSvgIcon-root').length,
};`, []);
console.log(JSON.stringify(dump, null, 1));
await driver.deleteSession();
