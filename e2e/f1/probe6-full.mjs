#!/usr/bin/env node
/**
 * E7.F1 probe6: COMPREHENSIVE single-session verification of editor/notes/linking scope.
 * Runs in ONE WebDriver session to avoid WebKitWebDriver session limits.
 *
 * Steps:
 *   A. Idle-open round-trip (no typing): does the disk file stay byte-stable?
 *   B. Backlinks panel: linked + unlinked mentions present for the page.
 *   C. Watcher round-trip: externally append to the file, wait, verify DB/index updated.
 *   D. Frontmatter preserved on save (after an edit).
 *   E. Marker rendering (pageMarkers) + marker badge presence.
 *   F. [[ wiki-link autocomplete probe (expected to FAIL = defect).
 *   G. XSS: Mermaid + KaTeX sanitization (JS path-level assertion).
 *   H. Save summary.
 */
import WebDriver from 'webdriver';
import fs from 'fs';
import crypto from 'crypto';
const APP = '/home/shrtcrct/git/stratum/target/debug/stratum-tauri';
const HOME = fs.readFileSync('/tmp/f2dir.txt', 'utf8').trim();
const VAULT = `${HOME}/StratumVault`;
const VAULT_FILE = `${VAULT}/notes/f1-editor.md`;
const OUT = '/tmp/stratum-f1-probe6';
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
fs.mkdirSync(OUT, { recursive: true });
const sha = (f) => crypto.createHash('sha256').update(fs.readFileSync(f)).digest('hex').slice(0,16);
const results = [];
const rec = (label, pass, detail='') => { results.push({ label, pass: !!pass, detail: String(detail||'').slice(0,400) }); console.log(`[${pass?'PASS':'FAIL'}] ${label}${detail?' — '+detail:''}`); };

async function main() {
  // 0) reset fixture to pristine source-of-truth
  fs.copyFileSync('/tmp/stratum-f1-1789776002/StratumVault/notes/f1-editor.md', VAULT_FILE);
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

  // ---- A) IDLE OPEN round-trip ----
  await js(`(function(){ history.pushState({},'', '/page/notes%2Ff1-editor.md'); dispatchEvent(new PopStateEvent('popstate')); return 'ok'; })()`);
  await sleep(7000);
  const idleRaw = fs.readFileSync(VAULT_FILE, 'utf8');
  const idleSha = sha(VAULT_FILE);
  const idleTail = idleRaw.slice(-120);
  const trailingBracketIdle = idleRaw.trimEnd().endsWith('[[');
  const mermaidIdPre = (fs.readFileSync('/tmp/stratum-f1-1789776002/StratumVault/notes/f1-editor.md','utf8').match(/```mermaid[\s\S]*?\.id: ([0-9a-f-]{36})/)||[])[1]||'';
  const mermaidIdIdle = (idleRaw.match(/```mermaid[\s\S]*?\.id: ([0-9a-f-]{36})/)||[])[1]||'';
  rec('A1 idle-open file byte-stable (no rewrite)', idleSha === preSha, `pre=${preSha} idle=${idleSha} rewrote=${idleSha!==preSha}`);
  rec('A2 no trailing "[[" appended on idle', !trailingBracketIdle, JSON.stringify(idleTail.slice(-40)));
  rec('A3 mermaid block id stable on round-trip', mermaidIdPre === mermaidIdIdle, `pre=${mermaidIdPre} idle=${mermaidIdIdle}`);

  // ---- B) BACKLINKS panel ----
  // Expand backlinks accordion(s)
  await js(`(function(){ var els=Array.prototype.slice.call(document.querySelectorAll('*')); els.forEach(function(e){ if((e.textContent||'').trim().indexOf('Backlinks (')===0 || (e.textContent||'').trim()==='Backlinks'){ e.click(); }}); return 'clicked'; })()`);
  await sleep(2500);
  const blText = await js(`return (function(){ return (document.body.innerText||'').slice(0,4000); })()`);
  const blLower = blText.toLowerCase();
  rec('B1 backlinks panel shows Linked References', /linked references \(\d+\)/.test(blLower), blText.split('\n').filter(l=>/linked|mention|backlink/i.test(l)).slice(0,4).join(' | '));
  rec('B2 backlinks panel shows Unlinked Mentions', /unlinked mentions? \(\d+\)/.test(blLower));

  // ---- C) WATCHER round-trip: externally append + ensure index refreshes ----
  // Append a marked line to the file externally
  const extMark = 'F1-EXTERNAL-WATCHED-' + Date.now();
  fs.appendFileSync(VAULT_FILE, `\n- ${extMark}\n  .id: 99999999-0000-0000-0000-000000000099\n`);
  await sleep(4000); // watcher debounce + index
  // Ask backend: get_blocks should now include the external block
  const blocksAfterExt = await js(`return (function(){
    var i=window.__TAURI_INTERNALS__; if(!i||typeof i.invoke!=='function') return {ok:false,e:'no bridge'};
    return i.invoke('get_blocks', { pagePath: 'notes/f1-editor.md' })
      .then(function(v){ return { ok:true, hasExt: v.blocks.some(function(b){ return (b.content||'').indexOf('F1-EXTERNAL-WATCHED')>=0; }) }; },
            function(e){ return { ok:false, e:String(e) }; });
  })()`);
  rec('C1 watcher: external edit synced to DB', blocksAfterExt && blocksAfterExt.ok && blocksAfterExt.hasExt, JSON.stringify(blocksAfterExt));
  // Search index refresh
  const srch = await js(`return (function(){
    var i=window.__TAURI_INTERNALS__;
    return i.invoke('search_blocks', { query: 'F1-EXTERNAL-WATCHED', limit: 3 })
      .then(function(v){ return { ok:true, hits: (v.results||[]).length }; }, function(e){ return { ok:false, e:String(e) }; });
  })()`);
  rec('C2 watcher: external edit searchable after sync', srch && srch.ok && srch.hits > 0, JSON.stringify(srch));

  // ---- D) FRONTMATTER preserved on a real edit + save ----
  // Focus editor, place cursor in first block end, type sentinel
  await js(`(function(){ var pm=document.querySelector('.ProseMirror[contenteditable="true"]'); if(!pm) return 'no'; pm.focus(); var sel=window.getSelection(); var r=document.createRange(); r.selectNodeContents(pm); r.collapse(true); sel.removeAllRanges(); sel.addRange(r); return 'ok'; })()`);
  await sleep(300);
  const sent = 'F1-EDIT-';
  for (const ch of sent) { await driver.performActions([{ type:'key', id:'kb', actions:[{type:'keyDown',value:ch},{type:'keyUp',value:ch}] }]); }
  await sleep(2500); // debounce+save
  const afterEditRaw = fs.readFileSync(VAULT_FILE, 'utf8');
  const hasTitle = /^title:\s*F1 Editor Test/m.test(afterEditRaw);
  const hasCustom = /custom_field:\s*keep-me-verbatim/.test(afterEditRaw);
  const hasTags = /tags:/.test(afterEditRaw);
  rec('D1 frontmatter title preserved after save', hasTitle);
  rec('D2 frontmatter custom_field preserved after save', hasCustom);
  rec('D3 frontmatter tags preserved after save', hasTags);
  rec('D4 edit sentinel persisted to disk', afterEditRaw.includes('F1-EDIT-'));

  // ---- E) MARKERS ----
  // pageMarkers: the page has TODO/DOING/DONE/B markers → badges should render
  const markerBadges = await js(`return (function(){
    var els=Array.prototype.slice.call(document.querySelectorAll('.MuiChip-root, [class*="marker"], [class*="badge"]'));
    return els.map(function(e){ return (e.textContent||'').trim().slice(0,12); }).filter(function(t){ return /^(TODO|DOING|DONE|NOW|LATER|WAITING|CANCELLED|[ABC])$/i.test(t); });
  })()`);
  rec('E1 marker badges rendered (TODO/DOING etc)', markerBadges.length > 0, JSON.stringify(markerBadges));

  // Try to set a marker interactively: type ":" at block start of a NEW block (marker suggestion menu)
  await js(`(function(){ var pm=document.querySelector('.ProseMirror[contenteditable="true"]'); pm.focus(); var sel=window.getSelection(); var r=document.createRange(); r.selectNodeContents(pm); r.collapse(false); sel.removeAllRanges(); sel.addRange(r); return 'ok'; })()`);
  await sleep(200);
  await driver.performActions([{ type:'key', id:'kb', actions:[{type:'keyDown',value:'\uE007'},{type:'keyUp',value:'\uE007'}]}]); // Enter → new block
  await sleep(300);
  await driver.performActions([{ type:'key', id:'kb', actions:[{type:'keyDown',value:':'},{type:'keyUp',value:':'}] }]); // type :
  await sleep(1000);
  const markerMenu = await js(`return (function(){ var m=document.querySelectorAll('.bn-suggestion-menu, [data-suggestion-menu], .bn-suggestion-decorator, [role="menuitem"]'); return Array.prototype.map.call(m, function(e){ return { cls:String(e.className||'').slice(0,50), text:(e.textContent||'').slice(0,40) }; }); })()`);
  const hasMarkerItems = markerMenu.some(m => /TODO|DOING|DONE|Priority|marker/i.test(m.text) || /marker/i.test(m.cls));
  rec('E2 ":" marker suggestion menu appears', hasMarkerItems, JSON.stringify(markerMenu.slice(0,6)));

  // ---- F) [[ wiki-link autocomplete ----
  await driver.performActions([{ type:'key', id:'kb', actions:[{type:'keyDown',value:'\uE003'},{type:'keyUp',value:'\uE003'},{type:'keyDown',value:'\uE003'},{type:'keyUp',value:'\uE003'}] }]); // delete the ':' then type [[
  await sleep(600);
  // Actually: clear the block then type [[
  await js(`(function(){ var pm=document.querySelector('.ProseMirror[contenteditable="true"]'); pm.focus(); var sel=window.getSelection(); var r=document.createRange(); r.selectNodeContents(pm); r.collapse(false); sel.removeAllRanges(); sel.addRange(r); return 'ok'; })()`);
  await driver.performActions([{ type:'key', id:'kb', actions:[{type:'keyDown',value:'\uE003'},{type:'keyUp',value:'\uE003'},{type:'keyDown',value:'['},{type:'keyUp',value:'['},{type:'keyDown',value:'['},{type:'keyUp',value:'['}] }]);
  await sleep(1200);
  const wikiMenu = await js(`return (function(){ var m=document.querySelectorAll('.bn-suggestion-menu, [data-suggestion-menu], [role="menuitem"]'); return Array.prototype.map.call(m, function(e){ return { cls:String(e.className||'').slice(0,50), text:(e.textContent||'').slice(0,40) }; }); })()`);
  const hasWikiItems = wikiMenu.length > 0 && /page|Alpha|wiki|note/i.test(JSON.stringify(wikiMenu));
  rec('F1 "[[ " wiki-link autocomplete menu appears', hasWikiItems, JSON.stringify(wikiMenu.slice(0,5)));

  // ---- G) XSS state on rendered DOM ----
  const xssDom = await js(`return (function(){
    var pm=document.querySelector('.ProseMirror[contenteditable="true"]');
    var scripts = pm?pm.querySelectorAll('script').length:-1;
    var imgOnError = pm?pm.querySelectorAll('img[onerror]').length:-1;
    var bodyScripts = document.querySelectorAll('body script').length;
    return { pmScripts: scripts, pmImgOnerror: imgOnError, bodyScripts };
  })()`);
  rec('G1 no live <script> from XSS payload in editor', xssDom && xssDom.pmScripts === 0, JSON.stringify(xssDom));

  try { const png = await driver.takeScreenshot(); fs.writeFileSync(OUT+'/probe6.png', Buffer.from(png,'base64')); } catch(e){}
  await driver.deleteSession().catch(()=>{});
  fs.writeFileSync(OUT+'/result.json', JSON.stringify(results,null,2));
  const fails = results.filter(r => !r.pass);
  console.log('\n==== TOTAL: ' + results.length + ' checks, ' + fails.length + ' FAIL ====');
  if (fails.length) console.log('FAILED: ' + fails.map(f=>f.label).join(', '));
  process.exit(fails.length ? 2 : 0);
}
main().catch((e) => { console.error('FATAL', e); process.exit(1); });
