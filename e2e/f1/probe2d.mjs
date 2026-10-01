#!/usr/bin/env node
/** Dump node type of each leaf blockContainer (data-node-type attr + node type class) */
import WebDriver from 'webdriver';
import fs from 'fs';
const APP = '/home/shrtcrct/git/stratum/target/debug/stratum-tauri';
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
async function main() {
  const driver = await WebDriver.newSession({
    hostname: '127.0.0.1', port: 4444, path: '/', protocol: 'http',
    connectionRetryTimeout: 60000, connectionRetryCount: 2,
    capabilities: { alwaysMatch: { browserName: 'wry', 'wdio:enforceWebDriverClassic': true,
      'webkitgtk:browserOptions': { binary: APP, args: ['--automation'] } } },
  });
  const js = (s) => {
    let wrapped = s;
    wrapped = wrapped.replace(/^\s*\(function\(\)\{([\s\S]*)\}\)\(\)\s*$/, 'return (function(){ $1 })();');
    wrapped = /^\s*return /.test(wrapped) ? wrapped : `return (function(){${wrapped}})();`;
    return driver.executeScript(wrapped, []);
  };
  let ready = false;
  for (let i = 0; i < 90; i++) {
    try { const t = await js('return document.getElementById("root")?document.getElementById("root").children.length:0;'); if (t > 0) { ready = true; break; } } catch (e) {}
    await sleep(2000);
  }
  if (!ready) { await driver.deleteSession().catch(()=>{}); process.exit(1); }
  await sleep(4000);
  await js(`(function(){ history.pushState({},'', '/page/notes%2Ff1-editor.md'); dispatchEvent(new PopStateEvent('popstate')); return 'ok'; })()`);
  await sleep(6000);
  const dump = await js(`return (function(){
    const all = Array.from(document.querySelectorAll('.ProseMirror [data-node-type]'));
    const seen = [];
    all.forEach(e => {
      const t = e.getAttribute('data-node-type');
      const id = (e.getAttribute('data-id')||'').slice(0,10);
      const cls = (e.className && e.className.baseVal!==undefined ? e.className.baseVal : e.className) || '';
      seen.push({ t, id, cls: String(cls).slice(0,60), text: (e.textContent||'').slice(0,40) });
    });
    return seen;
  })()`);
  console.log('SIZE', dump.length);
  console.log(JSON.stringify(dump, null, 1));
  await driver.deleteSession().catch(()=>{});
  process.exit(0);
}
main().catch((e) => { console.error('FATAL', e); process.exit(1); });
