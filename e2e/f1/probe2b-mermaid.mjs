#!/usr/bin/env node
/**
 * E7.F1 probe 2b: precise mermaid block rendering state + console error capture.
 */
import WebDriver from 'webdriver';
import fs from 'fs';

const APP = '/home/shrtcrct/git/stratum/target/debug/stratum-tauri';
const OUT = '/tmp/stratum-f1-probe2b';
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
  console.log('BOOT', ready ? 'OK' : 'FAIL');
  if (!ready) { await driver.deleteSession().catch(()=>{}); process.exit(1); }
  await sleep(4000);

  // Install console error capture BEFORE navigation
  await js(`(function(){
    window.__errs = [];
    const orig = console.error.bind(console);
    console.error = (...a) => { window.__errs.push(a.map(x => String(x && x.stack || x)).join(' ').slice(0,500)); orig(...a); };
    window.addEventListener('error', e => { window.__errs.push('global:' + e.message); });
    window.addEventListener('unhandledrejection', e => { window.__errs.push('rejection:' + (e.reason && e.reason.message || e.reason)); });
    return 'installed';
  })()`);

  await js(`(function(){ history.pushState({},'', '/page/notes%2Ff1-editor.md'); dispatchEvent(new PopStateEvent('popstate')); return 'ok'; })()`);
  await sleep(6000);

  // Locate mermaid-ish blocks: blocknote nodes that contain a monospace contentDOM
  const mermaid = await js(`return (function(){
    const pm = document.querySelector('.ProseMirror[contenteditable="true"]');
    if (!pm) return { hasPM:false };
    // BlockNote block DOM elements are [data-node-type][data-content-type] with data-id
    const blocks = pm.querySelectorAll('[data-node-type]');
    const out = { blockCount: blocks.length, blocks: [] };
    blocks.forEach(b => {
      const type = b.getAttribute('data-node-type');
      const id = b.getAttribute('data-id') || b.getAttribute('id') || '';
      const style = b.getAttribute('style') || '';
      const inner = (b.textContent||'').slice(0,120);
      const hasSvg = b.querySelectorAll('svg').length;
      const pre = b.querySelector('pre');
      const code = b.querySelector('code');
      out.blocks.push({ type, id: id.slice(0,16), inner, hasSvg, hasPre: !!pre, hasCode: !!code, style: style.slice(0,80) });
    });
    return out;
  })()`);
  console.log('BLOCKS:', JSON.stringify(mermaid, null, 1));

  // Check for any mermaid-rendered content anywhere (svg with id mermaid-)
  const anyMermaid = await js(`return (function(){
    const svgs = Array.from(document.querySelectorAll('svg[id^="mermaid"]'));
    return { count: svgs.length, ids: svgs.map(s=>s.id).slice(0,5), outer: (document.querySelector('svg[id^="mermaid"]')?.outerHTML||'').slice(0,300) };
  })()`);
  console.log('ANY MERMAID SVG:', JSON.stringify(anyMermaid));

  const errs = await js(`return (window.__errs||[]).slice(0,20)`);
  console.log('CONSOLE ERRORS:', JSON.stringify(errs, null, 1));

  const png = await driver.takeScreenshot();
  fs.writeFileSync(OUT + '/probe2b.png', Buffer.from(png, 'base64'));
  await driver.deleteSession().catch(()=>{});
  console.log('DONE');
  process.exit(0);
}
main().catch((e) => { console.error('FATAL', e); process.exit(1); });
