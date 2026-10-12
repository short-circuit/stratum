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
for (let i = 0; i < 60; i++) {
  try { const p = await exec(() => ({ c: document.getElementById('root')?.children?.length||0, b:(document.body?.innerText||'').length })); if (p.c>0&&p.b>100) break; } catch(e){}
  await sleep(1000);
}
await exec(() => { [...document.querySelectorAll('[role="button"],button,[role="menuitem"],li')].find(e=>(e.textContent||'').trim()==='Journal')?.click(); return true; });
await sleep(4000);
await exec(() => { const b=[...document.querySelectorAll('button,[role="button"]')].find(e=>(e.getAttribute('aria-label')||'').toLowerCase().includes('sprachnotiz aufnehmen')); if(b){b.click();return true;}return false; });
await sleep(3000);
const png = await driver.takeScreenshot();
const fs = await import('fs');
fs.writeFileSync('/tmp/voice-capture-after-click.png', Buffer.from(png,'base64'));
console.log('screenshot saved');
// Try clicking record via label text (Button contains text 'Sprachnotiz aufnehmen')
const byText = await exec(() => { const t=[...document.querySelectorAll('button')].find(e=>(e.textContent||'').trim()==='Sprachnotiz aufnehmen'); if(t){t.click();return true;} return false; });
console.log('clicked by text:', byText);
await sleep(2500);
console.log('stop now:', await exec(() => !!([...document.querySelectorAll('button,[role="button"]')].find(e=>(e.getAttribute('aria-label')||'').toLowerCase().includes('stoppen & speichern')))));
await driver.deleteSession();
