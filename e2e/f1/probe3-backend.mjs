#!/usr/bin/env node
/** E7.F1 probe3: ground truth — invoke backend get_blocks + open_page through Tauri internals. */
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

  // Detect the Tauri invoke bridge variant
  const bridge = await js(`return (function(){
    const i = window.__TAURI_INTERNALS__;
    const t = window.__TAURI__;
    return {
      hasInternalsInvoke: !!(i && typeof i.invoke === 'function'),
      hasTauriCoreInvoke: !!(t && t.core && typeof t.core.invoke === 'function'),
    };
  })()`);
  console.log('BRIDGE', JSON.stringify(bridge));

  const inv = (cmd, args) => js(`return (function(){
    const a = ${JSON.stringify(args || {})};
    const internals = window.__TAURI_INTERNALS__;
    const tauri = window.__TAURI__;
    const p = internals && typeof internals.invoke === 'function'
      ? internals.invoke(${JSON.stringify(cmd)}, a)
      : (tauri && tauri.core ? tauri.core.invoke(${JSON.stringify(cmd)}, a) : Promise.reject('no bridge'));
    return p.then(v => ({ ok:true, v }), e => ({ ok:false, e:String(e) }));
  })()`);

  // Ground truth: get_blocks for the f1 page
  const blocks = await inv('get_blocks', { pagePath: 'notes/f1-editor.md' });
  console.log('GET_BLOCKS:', JSON.stringify(blocks, null, 1).slice(0, 4000));

  // open_page
  const page = await inv('open_page', { path: 'notes/f1-editor.md' });
  console.log('OPEN_PAGE keys:', JSON.stringify(page).slice(0, 1500));
  if (page.ok && page.v && page.v.content) console.log('PAGE CONTENT:\n' + String(page.v.content).slice(0, 2000));

  const png = await driver.takeScreenshot();
  fs.writeFileSync('/tmp/stratum-f1-probe3.png', Buffer.from(png, 'base64'));
  await driver.deleteSession().catch(()=>{});
  process.exit(0);
}
main().catch((e) => { console.error('FATAL', e); process.exit(1); });
