#!/usr/bin/env node
/**
 * E7.F1 probe9 — WATCHER ROUND-TRIP VERIFICATION (post-fix, clean env).
 * Boots the app on a FRESH vault (f5dir) with the fixture, waits for initial
 * sync to settle, then externally MODIFIES an existing .md and creates a NEW
 * .md, verifying each reaches the SQLite DB + search index via the watcher.
 * Uses the fixed binary (config fallback so the watcher starts without a
 * config.toml). No app-internal save happens before the external writes, and
 * waits exceed the 2s own-save skip window.
 */
import WebDriver from 'webdriver';
import fs from 'fs';
const APP = '/home/shrtcrct/git/stratum/target/debug/stratum-tauri';
const HOME = fs.readFileSync('/tmp/f5dir.txt', 'utf8').trim();
const VAULT = `${HOME}/StratumVault`;
const F = `${VAULT}/notes/f1-editor.md`;
const OUT = '/home/shrtcrct/git/stratum/e2e/f1/.evidence/probe9';
const sleep = ms => new Promise(r => setTimeout(r, ms));
fs.mkdirSync(OUT, { recursive: true });
const r = [];
const rec = (l, p, d = '') => { r.push({ l, p: !!p, d: String(d || '').slice(0, 400) }); console.log(`[${p ? 'PASS' : 'FAIL'}] ${l}${d ? ' — ' + d : ''}`); };

async function main() {
  const driver = await WebDriver.newSession({
    hostname: '127.0.0.1', port: 4444, path: '/', protocol: 'http',
    connectionRetryTimeout: 60000, connectionRetryCount: 2,
    capabilities: { alwaysMatch: { browserName: 'wry', 'wdio:enforceWebDriverClassic': true,
      'webkitgtk:browserOptions': { binary: APP, args: ['--automation'] } } },
  });
  const js = s => {
    let w = s;
    if (/^\s*\(function\(\)\{[\s\S]*\}\)\(\)\s*$/.test(w)) w = w.replace(/^\s*\(function\(\)\{([\s\S]*)\}\)\(\)\s*$/, 'return (function(){ $1 })();');
    else if (!/^\s*return /.test(w)) w = `return (function(){${w}})();`;
    return driver.executeScript(w, []);
  };
  let ready = false;
  for (let i = 0; i < 120; i++) { try { const t = await js('return document.getElementById("root")?document.getElementById("root").children.length:0;'); if (t > 0) { ready = true; break; } } catch (e) {} await sleep(1500); }
  rec('boot', ready);
  if (!ready) { await driver.deleteSession().catch(() => {}); fs.writeFileSync(OUT + '/r.json', JSON.stringify(r, null, 2)); process.exit(1); }
  await sleep(8000); // let initial filesystem sync + watcher fully settle (>2s own-save window)

  // baseline: blocks reachable + markers present (fresh sync of proper fixture)
  const base = await js(`return (function(){
    var i=window.__TAURI_INTERNALS__; if(!i||typeof i.invoke!=='function') return {ok:false,e:'no bridge'};
    return i.invoke('get_blocks',{pagePath:'notes/f1-editor.md'}).then(v=>({ok:true,cnt:v.blocks.length}),e=>({ok:false,e:String(e)}));
  })()`);
  rec('baseline: get_blocks reachable', base.ok, JSON.stringify(base));
  const mk = await js(`return (function(){
    var i=window.__TAURI_INTERNALS__;
    return i.invoke('get_blocks',{pagePath:'notes/f1-editor.md'}).then(v=>({ok:true, markers:v.blocks.map(b=>b.marker).filter(Boolean)}),e=>({ok:false,e:String(e)}));
  })()`);
  rec('baseline: markers present in fresh sync', mk.ok && mk.markers.length > 0, JSON.stringify(mk));

  // EXTERNAL MODIFY of an existing page
  const ext1 = 'F1-EXT-VFY-' + Date.now();
  fs.appendFileSync(F, `\n- ${ext1}\n  .id: aaaaaaaa-0000-0000-0000-0000000000aa\n`);
  await sleep(8000);
  const afterMod = await js(`return (function(){
    var i=window.__TAURI_INTERNALS__;
    return i.invoke('get_blocks',{pagePath:'notes/f1-editor.md'}).then(v=>({ok:true,cnt:v.blocks.length,has:v.blocks.some(b=>(b.content||'').indexOf('F1-EXT-VFY-')>=0)}),e=>({ok:false,e:String(e)}));
  })()`);
  rec('watcher: external modify synced to DB (existing page)', afterMod.ok && afterMod.has, JSON.stringify(afterMod));
  const sz = await js(`return (function(){
    var i=window.__TAURI_INTERNALS__;
    return i.invoke('search_blocks',{query:'F1-EXT-VFY-',limit:3}).then(v=>({ok:true,hits:(v.results||[]).length}),e=>({ok:false,e:String(e)}));
  })()`);
  rec('watcher: external modify searchable', sz.ok && sz.hits > 0, JSON.stringify(sz));

  // CREATE a NEW page externally
  const newPage = `newvfy-${Date.now()}`;
  fs.writeFileSync(`${VAULT}/notes/${newPage}.md`, `---\ntitle: ${newPage}\n---\nExternal ${ext1} content.\n`);
  await sleep(8000);
  const np = await js(`return (function(){
    var i=window.__TAURI_INTERNALS__;
    return i.invoke('list_pages').then(v=>({ok:true, has:v.pages.some(p=>p.path.indexOf('newvfy-')>=0)}),e=>({ok:false,e:String(e)}));
  })()`);
  rec('watcher: new external page appears in list_pages', np.ok && np.has, JSON.stringify(np));
  const np2 = await js(`return (function(){
    var i=window.__TAURI_INTERNALS__;
    return i.invoke('get_blocks',{pagePath:'notes/'+${JSON.stringify(newPage)}+'.md'}).then(v=>({ok:true,cnt:v.blocks.length}),e=>({ok:false,e:String(e)}));
  })()`);
  rec('watcher: new page blocks reachable', np2.ok && np2.cnt > 0, JSON.stringify(np2));

  // backlinks reflect the linker page (synced at boot via watcher/filesystem sync)
  const bl = await js(`return (function(){
    var i=window.__TAURI_INTERNALS__;
    return i.invoke('get_page_backlinks',{pagePath:'notes/f1-editor.md'}).then(v=>({ok:true,txt:JSON.stringify(v).slice(0,200)}),e=>({ok:false,e:String(e)}));
  })()`);
  rec('backlinks API reachable', bl.ok, bl.txt || bl.e);

  try { const png = await driver.takeScreenshot(); fs.writeFileSync(OUT + '/probe9.png', Buffer.from(png, 'base64')); } catch (e) {}
  await driver.deleteSession().catch(() => {});
  fs.writeFileSync(OUT + '/r.json', JSON.stringify(r, null, 2));
  const fails = r.filter(x => !x.p);
  console.log(`\n==== TOTAL ${r.length} / FAIL ${fails.length} ====`);
  if (fails.length) console.log('FAILS: ' + fails.map(x => x.l).join(', '));
  process.exit(fails.length ? 2 : 0);
}
main().catch(e => { console.error('FATAL', e); process.exit(1); });
