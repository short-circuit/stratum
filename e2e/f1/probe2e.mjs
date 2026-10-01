#!/usr/bin/env node
/** E7.F1 probe2e: on the CLEAN fixture, inspect the mermaid block type + render state + wiki autocomplete. */
import WebDriver from 'webdriver';
import fs from 'fs';
const APP = '/home/shrtcrct/git/stratum/target/debug/stratum-tauri';
const OUT = '/tmp/stratum-f1-probe2e';
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
  await sleep(7000);

  // 1. mermaid block: find blocks containing 'graph TD' and inspect their outer class for mermaid marker
  const mer = await js(`return (function(){
    const all = Array.from(document.querySelectorAll('.ProseMirror [data-node-type="blockOuter"]'));
    const res = [];
    all.forEach(b => {
      const t = (b.textContent||'');
      if (t.indexOf('graph TD') >= 0 || t.indexOf('mermaid') >= 0) {
        res.push({
          text: t.slice(0,120),
          hasSvg: !!b.querySelector('svg'),
          svgId: (b.querySelector('svg')?.id||'').slice(0,40),
          hasPre: !!b.querySelector('pre'),
          preText: (b.querySelector('pre')?.textContent||'').slice(0,80),
          outerClass: String(b.className||'').slice(0,80),
          bgColors: (()=>{ const r=[]; b.querySelectorAll('*').forEach(e=>{ const s=getComputedStyle(e).backgroundColor; if(s && s!=='rgba(0, 0, 0, 0)' && s.indexOf('0, 0, 0, 0')<0) r.push(s); }); return r.slice(0,5); })(),
        });
      }
    });
    return res;
  })()`);
  console.log('MERMAID_BLOCKS:', JSON.stringify(mer, null, 1));

  // 2. wiki autocomplete: focus editor, type [[, check for suggestion menu
  await js(`(function(){ var pm=document.querySelector('.ProseMirror[contenteditable="true"]'); if(!pm) return 'no'; pm.focus(); var sel=window.getSelection(); var r=document.createRange(); r.selectNodeContents(pm); r.collapse(false); sel.removeAllRanges(); sel.addRange(r); return 'ok'; })()`);
  await sleep(300);
  await driver.performActions([{ type: 'key', id: 'kb', actions: [
    { type: 'keyDown', value: '[' }, { type: 'keyUp', value: '[' },
    { type: 'keyDown', value: '[' }, { type: 'keyUp', value: '[' },
  ]}]);
  await sleep(1200);
  const menus = await js(`return (function(){
    const m = document.querySelectorAll('.bn-suggestion-menu, [data-suggestion-menu], .bn-suggestion-decorator, [role="menuitem"]');
    return Array.from(m).map(e => ({ cls: String(e.className||''), text: (e.textContent||'').slice(0,60) }));
  })()`);
  console.log('MENUS_AFTER_[[', JSON.stringify(menus));

  const png = await driver.takeScreenshot();
  fs.writeFileSync(OUT + '/probe2e.png', Buffer.from(png,'base64'));
  await driver.deleteSession().catch(()=>{});
  process.exit(0);
}
main().catch((e) => { console.error('FATAL', e); process.exit(1); });
