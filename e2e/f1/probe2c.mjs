#!/usr/bin/env node
/** Find the mermaid block's actual node type in the ProseMirror doc. */
import WebDriver from 'webdriver';
import fs from 'fs';

const APP = '/home/shrtcrct/git/stratum/target/debug/stratum-tauri';
const OUT = '/tmp/stratum-f1-probe2c';
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
fs.mkdirSync(OUT, { recursive: true });

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

  // Dump all leaf block node types + first words so we can spot mermaid
  const dump = await js(`return (function(){
    const pm = document.querySelector('.ProseMirror[contenteditable="true"]');
    if (!pm) return { hasPM:false };
    const blocks = pm.querySelectorAll('[data-node-type="blockOuter"]');
    const arr = [];
    blocks.forEach(b => {
      const container = b.querySelector('[data-node-type="blockContainer"]');
      const t = (b.textContent||'');
      arr.push({
        id: (container?.getAttribute('data-id')||'').slice(0,8),
        content: t.slice(0,60),
        dataType: container?.getAttribute('data-content-type')||'',
        containsPre: !!b.querySelector('pre'),
        containsCode: !!b.querySelector('code'),
        codeText: (b.querySelector('code')?.textContent||'').slice(0,50),
      });
    });
    return arr;
  })()`);
  console.log(JSON.stringify(dump, null, 1));

  const png = await driver.takeScreenshot();
  fs.writeFileSync(OUT + '/probe2c.png', Buffer.from(png, 'base64'));
  await driver.deleteSession().catch(()=>{});
  process.exit(0);
}
main().catch((e) => { console.error('FATAL', e); process.exit(1); });
