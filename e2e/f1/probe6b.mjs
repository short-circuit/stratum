#!/usr/bin/env node
/**
 * E7.F1 probe6b — COMPREHENSIVE single-session verification (corrected).
 * Order: idle-open, backlinks (linked+unlinked), watcher round-trip FIRST,
 * frontmatter-on-edit, markers, [[ autocomplete, XSS.
 * Uses corrected fixtures (.marker: property format + a linker page).
 */
import WebDriver from 'webdriver';
import fs from 'fs';
import crypto from 'crypto';
const APP = '/home/shrtcrct/git/stratum/target/debug/stratum-tauri';
const HOME = fs.readFileSync('/tmp/f4dir.txt', 'utf8').trim();
const VAULT = `${HOME}/StratumVault`;
const VAULT_FILE = `${VAULT}/notes/f1-editor.md`;
const OUT = '/tmp/stratum-f1-probe6b';
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
fs.mkdirSync(OUT, { recursive: true });
const sha = (f) => crypto.createHash('sha256').update(fs.readFileSync(f)).digest('hex').slice(0,16);
const results = [];
const rec = (label, pass, detail='') => { results.push({ label, pass: !!pass, detail: String(detail||'').slice(0,400) }); console.log(`[${pass?'PASS':'FAIL'}] ${label}${detail?' — '+detail:''}`); };

async function main() {
  // reset fixture
  fs.copyFileSync('/home/shrtcrct/git/stratum/e2e/f1/fixture-f1-editor.md', VAULT_FILE);
  fs.copyFileSync('/home/shrtcrct/git/stratum/e2e/f1/fixture-linker.md', `${VAULT}/notes/linker.md`);
  const preSha = sha(VAULT_FILE);

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
  for (let i = 0; i < 90; i++) {
    try { const t = await js('return document.getElementById("root")?document.getElementById("root").children.length:0;'); if (t > 0) { ready = true; break; } } catch (e) {}
    await sleep(2000);
  }
  rec('boot', ready);
  if (!ready) { await driver.deleteSession().catch(()=>{}); fs.writeFileSync(OUT+'/result.json', JSON.stringify(results,null,2)); process.exit(1); }
  await sleep(4000);

  // ---- A) IDLE-OPEN round-trip (byte stability) ----
  await js(`(function(){ history.pushState({},'', '/page/notes%2Ff1-editor.md'); dispatchEvent(new PopStateEvent('popstate')); return 'ok'; })()`);
  await sleep(7000);
  const idle = fs.readFileSync(VAULT_FILE,'utf8');
  rec('A1 idle-open byte-stable', sha(VAULT_FILE) === preSha);
  rec('A2 no trailing "[[ "+ mangling', !/\[$/.test(idle.trimEnd()) && !/\[\[\[/.test(idle), idle.slice(-60).replace(/\n/g,'\\n'));
  const mermaidIdIn = (fs.readFileSync('/home/shrtcrct/git/stratum/e2e/f1/fixture-f1-editor.md','utf8').match(/([0-9a-f]{8}-[0-9a-f-]{27})[\s\n]*$/)||[])[1]||'';
  const mermaidIdOut = (idle.match(/([0-9a-f-]{36})/)||[])[1]||'';
  rec('A3 uuid structure preserved (no wholesale churn)', idle.includes('1f399b72-6af2-446f-a6ac-083fce7abfb4') && idle.includes('eee560e2')||idle.includes('71f1a2b3'), '');

  // ---- B) BACKLINKS (linked + unlinked) ----
  await js(`(function(){ var els=Array.prototype.slice.call(document.querySelectorAll('*')); els.forEach(function(e){ if((e.textContent||'').trim().indexOf('Backlinks (')===0){ e.click(); }}); return 'ok'; })()`);
  await sleep(2500);
  const bl = await js(`return (document.body.innerText||'').slice(0,4000);`);
  const bl0 = bl.toLowerCase();
  rec('B1 backlinks: Linked References shown', /linked references \(\d+\)/.test(bl0), bl.split('\n').filter(l=>/linked|mention/i.test(l)).slice(0,6).join(' | '));
  rec('B2 backlinks: Unlinked Mentions shown', /unlinked mentions? \(\d+\)/.test(bl0));

  // ---- C) WATCHER round-trip (external edit) — BEFORE any internal save ----
  // Wait > 2s since the last internal save (idle-open produced none). Still ensure >3s.
  const extMark = 'F1-EXT-' + Date.now();
  fs.appendFileSync(VAULT_FILE, `\n- ${extMark}\n  .id: 99999999-0000-0000-0000-000000000099\n`);
  await sleep(6000);
  const extBlocks = await js(`return (function(){
    var i=window.__TAURI_INTERNALS__;
    return i.invoke('get_blocks', { pagePath: 'notes/f1-editor.md' })
      .then(function(v){ return { ok:true, cnt:(v.blocks||[]).length, hasExt: v.blocks.some(function(b){ return (b.content||'').indexOf('F1-EXT-')>=0; }) }; }, function(e){ return { ok:false, e:String(e) }; });
  })()`);
  rec('C1 watcher: external edit synced to DB', extBlocks.ok && extBlocks.hasExt, JSON.stringify(extBlocks));
  const extSearch = await js(`return (function(){
    var i=window.__TAURI_INTERNALS__;
    return i.invoke('search_blocks', { query: 'F1-EXT-', limit: 3 })
      .then(function(v){ return { ok:true, hits:(v.results||[]).length }; }, function(e){ return { ok:false, e:String(e) }; });
  })()`);
  rec('C2 watcher: external edit indexed/searchable', extSearch.ok && extSearch.hits > 0, JSON.stringify(extSearch));

  // ---- D) FRONTMATTER preserved on an actual edit ----
  // remove the externally-appended block first via editor or just type sentinel at start
  await js(`(function(){ var pm=document.querySelector('.ProseMirror[contenteditable="true"]'); pm.focus(); var sel=window.getSelection(); var r=document.createRange(); r.selectNodeContents(pm); r.collapse(true); sel.removeAllRanges(); sel.addRange(r); return 'ok'; })()`);
  await sleep(300);
  const sent = 'F1-EDIT-';
  for (const ch of sent) { await driver.performActions([{ type:'key', id:'kb', actions:[{type:'keyDown',value:ch},{type:'keyUp',value:ch}] }]); }
  await sleep(2500);
  const after = fs.readFileSync(VAULT_FILE,'utf8');
  rec('D1 frontmatter title preserved', /^title:\s*F1 Editor Test/m.test(after));
  rec('D2 frontmatter custom_field preserved', /custom_field:\s*keep-me-verbatim/.test(after));
  rec('D3 frontmatter tags preserved', /tags:[^\n]*\n- project/.test(after));
  rec('D4 edit sentinel persisted', after.includes('F1-EDIT-'));

  // ---- E) MARKERS ----
  // Markers from fixture (.marker: property) should surface as marker badges in header
  const badges = await js(`return (function(){
    var els=Array.prototype.slice.call(document.querySelectorAll('.MuiChip-root, [class*="badge"], [class*="marker"]'));
    return els.map(function(e){ return (e.textContent||'').trim().slice(0,12); }).filter(function(t){ return /^(TODO|DOING|DONE|NOW|LATER|WAITING|[ABC])$/i.test(t); });
  })()`);
  rec('E1 marker badges rendered from .marker: fixtures', badges.length > 0, JSON.stringify(badges));
  // Check DB now carries markers (get_blocks)
  const mk = await js(`return (function(){
    var i=window.__TAURI_INTERNALS__;
    return i.invoke('get_blocks', { pagePath: 'notes/f1-editor.md' })
      .then(function(v){ return { ok:true, markers: v.blocks.map(function(b){ return b.marker; }).filter(Boolean) }; }, function(e){ return { ok:false, e:String(e) }; });
  })()`);
  rec('E2 DB carries block markers', mk.ok && mk.markers.length > 0, JSON.stringify(mk));
  // Interactive :: marker menu
  await js(`(function(){ var pm=document.querySelector('.ProseMirror[contenteditable="true"]'); pm.focus(); var sel=window.getSelection(); var r=document.createRange(); r.selectNodeContents(pm); r.collapse(false); sel.removeAllRanges(); sel.addRange(r); return 'ok'; })()`);
  await sleep(200);
  await driver.performActions([{ type:'key', id:'kb', actions:[{type:'keyDown',value:'\uE007'},{type:'keyUp',value:'\uE007'}] }]);
  await sleep(300);
  await driver.performActions([{ type:'key', id:'kb', actions:[{type:'keyDown',value:':'},{type:'keyUp',value:':'}] }]);
  await sleep(1200);
  const mm = await js(`return (function(){ var m=document.querySelectorAll('.bn-suggestion-menu, [data-suggestion-menu], [role="menuitem"], .bn-suggestion-decorator'); return Array.prototype.map.call(m, function(e){ return { cls:String(e.className||'').slice(0,60), text:(e.textContent||'').slice(0,50) }; }); })()`);
  const mmHas = mm.some(function(x){ return /TODO|DOING|DONE|Priority|marker/i.test(x.text); });
  rec('E3 ":" marker suggestion menu appears', mmHas, JSON.stringify(mm.slice(0,5)));

  // ---- F) [[ wiki-link autocomplete ----
  await driver.performActions([{ type:'key', id:'kb', actions:[{type:'keyDown',value:'\uE003'},{type:'keyUp',value:'\uE003'},{type:'keyDown',value:'['},{type:'keyUp',value:'['},{type:'keyDown',value:'['},{type:'keyUp',value:'['}] }]);
  await sleep(1500);
  const wm = await js(`return (function(){ var m=document.querySelectorAll('.bn-suggestion-menu, [data-suggestion-menu], [role="menuitem"]'); return Array.prototype.map.call(m, function(e){ return { cls:String(e.className||'').slice(0,60), text:(e.textContent||'').slice(0,50) }; }); })()`);
  const wmHas = wm.length > 0 && /page|Alpha|wiki|note/i.test(JSON.stringify(wm));
  rec('F1 "[[ " wiki-link autocomplete menu appears', wmHas, JSON.stringify(wm.slice(0,5)));

  // ---- G) XSS live state ----
  const x = await js(`return (function(){ var pm=document.querySelector('.ProseMirror[contenteditable="true"]'); return { pmScripts: pm?pm.querySelectorAll('script').length:-1, pmImgOnError: pm?pm.querySelectorAll('img[onerror]').length:-1, bodyScripts: document.querySelectorAll('body script').length }; })()`);
  rec('G1 no live <script> from XSS payload', x.pmScripts === 0, JSON.stringify(x));

  try { const png = await driver.takeScreenshot(); fs.writeFileSync(OUT+'/probe6b.png', Buffer.from(png,'base64')); } catch(e){}
  await driver.deleteSession().catch(()=>{});
  fs.writeFileSync(OUT+'/result.json', JSON.stringify(results,null,2));
  const fails = results.filter(r=>!r.pass);
  console.log('\n==== TOTAL '+results.length+' / FAIL '+fails.length+' ====');
  if (fails.length) console.log('FAILS: '+fails.map(f=>f.label).join(', '));
  process.exit(fails.length ? 2 : 0);
}
main().catch(e=>{ console.error('FATAL', e); process.exit(1); });
