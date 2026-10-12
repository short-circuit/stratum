import WebDriver from 'webdriver';
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const appBinary = './.worktrees/t_e1982a1b/target/debug/stratum-tauri';
const driver = await WebDriver.newSession({
  hostname: '127.0.0.1', port: 4444, path: '/', protocol: 'http',
  connectionRetryTimeout: 30000, connectionRetryCount: 1,
  capabilities: { alwaysMatch: { browserName: 'wry',
    'wdio:enforceWebDriverClassic': true,
    'webkitgtk:browserOptions': { binary: appBinary, args: ['--automation'] } } },
});
const exec = (fn) => driver.executeScript(`return (${fn.toString()})()`, []);
console.log('Session:', driver.sessionId);
for (let i = 0; i < 60; i++) {
  try { const p = await exec(() => ({ c: document.getElementById('root')?.children?.length||0, b:(document.body?.innerText||'').length })); if (p.c>0&&p.b>100) break; } catch(e){}
  await sleep(1000);
}
await exec(() => { [...document.querySelectorAll('[role="button"],button,[role="menuitem"],li')].find(e=>(e.textContent||'').trim()==='Journal')?.click(); return true; });
await sleep(4000);
const clicked = await exec(() => { const b=[...document.querySelectorAll('button,[role="button"]')].find(e=>(e.getAttribute('aria-label')||'').toLowerCase().includes('sprachnotiz aufnehmen')); if(b){b.click();return true;}return false; });
console.log('clicked record:', clicked);
for (let k=0;k<10;k++){
  await sleep(1000);
  const info = await exec(() => {
    const t = (document.body.innerText||'');
    const hasStop = !!([...document.querySelectorAll('button,[role="button"]')].find(e=>(e.getAttribute('aria-label')||'').toLowerCase().includes('stoppen & speichern')));
    const hasCancel = !!([...document.querySelectorAll('button,[role="button"]')].find(e=>(e.getAttribute('aria-label')||'').toLowerCase().includes('abbrechen')));
    const errText = [...document.querySelectorAll('.MuiAlert-message, [role="alert"]')].map(e=>e.textContent||'').slice(0,5);
    return { hasStop, hasCancel, errText };
  });
  console.log(`t+${k+1}s stop=${info.hasStop} cancel=${info.hasCancel} errText=${JSON.stringify(info.errText)}`);
  if (info.hasStop || info.hasCancel || (info.errText&&info.errText.length)) break;
}
console.log('all buttons:', await exec(() => [...document.querySelectorAll('button,[role="button"]')].map(e=>e.getAttribute('aria-label')||e.textContent||'').filter(Boolean).slice(0,20)));
await driver.deleteSession();
