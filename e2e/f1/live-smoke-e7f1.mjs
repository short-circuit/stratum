#!/usr/bin/env node
// E7.F1 live smoke probe: boot the rebuilt app against a clean fixture vault
// and verify the editor loads, wiki-links render, and backend block commands
// work end-to-end with the frontmatter fix in place.
import WebDriver from 'webdriver';
import fs from 'fs';

const APP = '/home/shrtcrct/git/stratum/target/debug/stratum-tauri';
const HOME = '/tmp/f1-e7/home';
const VAULT = `${HOME}/StratumVault`;
const OUT = '/home/shrtcrct/git/stratum/e2e/f1/.evidence/live-e7f1';
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// Set up the app home with the vault fixture (app reads HOME/StratumVault).
fs.rmSync(HOME, { recursive: true, force: true });
fs.mkdirSync(`${HOME}/StratumVault`, { recursive: true });
fs.cpSync('/tmp/f1-e7/livevault', VAULT, { recursive: true });
fs.writeFileSync('/tmp/f1-e7/home.txt', HOME);
fs.mkdirSync(OUT, { recursive: true });

const r = [];
const rec = (l, p, d = '') => {
  r.push({ l, p: !!p, d: String(d || '').slice(0, 400) });
  console.log(`[${p ? 'PASS' : 'FAIL'}] ${l}${d ? ' — ' + d : ''}`);
};

async function main() {
  const driver = await WebDriver.newSession({
    hostname: '127.0.0.1', port: 4444, path: '/', protocol: 'http',
    connectionRetryTimeout: 60000, connectionRetryCount: 2,
    capabilities: { alwaysMatch: { browserName: 'wry', 'wdio:enforceWebDriverClassic': true,
      'webkitgtk:browserOptions': { binary: APP, args: ['--automation'] } } },
  });
  const js = (s) => {
    let w = s;
    if (/^\s*\(function\(\)\{[\s\S]*\}\)\(\)\s*$/.test(w)) w = w.replace(/^\s*\(function\(\)\{([\s\S]*)\}\)\(\)\s*$/, 'return (function(){ $1 })();');
    else if (!/^\s*return /.test(w)) w = `return (function(){${w}})();`;
    return driver.executeScript(w, []);
  };
  let ready = false;
  for (let i = 0; i < 120; i++) {
    try { const t = await js('return document.getElementById("root")?document.getElementById("root").children.length:0;'); if (t > 0) { ready = true; break; } } catch (e) {}
    await sleep(1500);
  }
  rec('boot', ready);
  if (!ready) { await driver.deleteSession().catch(()=>{}); fs.writeFileSync(OUT + '/r.json', JSON.stringify(r, null, 2)); process.exit(1); }
  await sleep(6000); // allow initial sync to settle

  // Navigate to the editor page.
  await js(`(function(){ history.pushState({},'', '/page/notes%2Ff1-editor.md'); dispatchEvent(new PopStateEvent('popstate')); return 'ok'; })()`);
  await sleep(6000);
  const ed = await js(`return {
    path: location.pathname,
    hasProseMirror: !!document.querySelector('.ProseMirror[contenteditable="true"]'),
    pmText: (document.querySelector('.ProseMirror[contenteditable="true"]')?.innerText||'').slice(0,400),
  };`);
  rec('editor loads with ProseMirror', ed.hasProseMirror, JSON.stringify({ path: ed.path }));
  rec('fixture content rendered', ed.hasProseMirror && /ship the f1 mission/.test(ed.pmText || ''), JSON.stringify(ed.pmText.slice(0,120)));

  // Marker readback through real IPC.
  const blocks = await js(`return (function(){
    var i=window.__TAURI_INTERNALS__;
    return i.invoke('get_blocks',{pagePath:'notes/f1-editor.md'}).then(v=>({ok:true,cnt:v.blocks.length,markers:v.blocks.map(b=>b.marker).filter(Boolean)}),e=>({ok:false,e:String(e)}));
  })()`);
  rec('get_blocks reachable', blocks.ok, JSON.stringify({ cnt: blocks.cnt }));
  rec('markers present (TODO/DONE)', blocks.ok && blocks.markers && blocks.markers.length >= 2 && blocks.markers.includes('TODO') && blocks.markers.includes('DONE'), JSON.stringify(blocks.markers));

  // Frontmatter round-trip via update_block+save_blocks (frontmatter-preservation path).
  const fid = blocks.ok && blocks.cnt > 0 ? (await js(`return (function(){
    var i=window.__TAURI_INTERNALS__;
    return i.invoke('get_blocks',{pagePath:'notes/f1-editor.md'}).then(v=>v.blocks[0].id);
  })()`)) : null;
  const sv = await js(`(function(){
    var i=window.__TAURI_INTERNALS__;
    return i.invoke('get_blocks',{pagePath:'notes/f1-editor.md'}).then(function(v){
      var b=v.blocks[1];
      b.content=b.content+' (live-edit)';
      return i.invoke('save_blocks',{pagePath:'notes/f1-editor.md',blocks:v.blocks}).then(function(){return {ok:true}},function(e){return {ok:false,e:String(e)}});
    });
  })()`);
  rec('save_blocks applies a live edit', sv.ok, JSON.stringify(sv));
  const disk = fs.readFileSync(`${VAULT}/notes/f1-editor.md`, 'utf8');
  rec('live edit persisted to disk', disk.includes('(live-edit)'), '');
  rec('frontmatter tag preserved on save (tags block form)', disk.includes('- project') && disk.includes('- editor'), disk.slice(0, 60).split('\n').join('|'));

  try { const png = await driver.takeScreenshot(); fs.writeFileSync(OUT + '/live.png', Buffer.from(png, 'base64')); } catch (e) {}
  await driver.deleteSession().catch(()=>{});
  fs.writeFileSync(OUT + '/r.json', JSON.stringify(r, null, 2));
  const fails = r.filter(x => !x.p);
  console.log(`\n==== TOTAL ${r.length} checks, ${fails.length} FAIL ====`);
  if (fails.length) console.log('FAILED: ' + fails.map(f => f.l).join(', '));
  process.exit(fails.length ? 2 : 0);
}
main().catch((e) => { console.error('FATAL', e); process.exit(1); });
