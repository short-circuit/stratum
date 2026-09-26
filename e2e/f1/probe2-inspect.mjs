#!/usr/bin/env node
/**
 * E7.F1 phase-2 probe: targeted inspection of
 *  a) mermaid render path (does an SVG actually render?)
 *  b) wiki-link [[ autocomplete (type [[ and see if a menu appears)
 *  c) marker : autocomplete
 *  d) how wiki-links appear in DOM (href scheme)
 */
import WebDriver from 'webdriver';
import fs from 'fs';

const APP = '/home/shrtcrct/git/stratum/target/debug/stratum-tauri';
const OUT = '/tmp/stratum-f1-probe2';
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

  await js(`(function(){ history.pushState({},'', '/page/notes%2Ff1-editor.md'); dispatchEvent(new PopStateEvent('popstate')); return 'ok'; })()`);
  await sleep(5000);

  // (a) DOM inspection for mermaid render + links
  const dom = await js(`return (function(){
    const pm = document.querySelector('.ProseMirror[contenteditable="true"]');
    const out = { hasPM: !!pm, links: [], mermaidEls: [], mathEls: [], markerEls: [] };
    if (!pm) return out;
    // links inside editor
    pm.querySelectorAll('a').forEach(a => {
      out.links.push({ text: a.textContent, href: a.getAttribute('href'), cls: a.className || '' });
    });
    // any element with mermaid hint
    pm.querySelectorAll('*').forEach(e => {
      const cn = (e.className && e.className.baseVal !== undefined ? e.className.baseVal : e.className) || '';
      if (typeof cn === 'string' && /mermaid/i.test(cn)) out.mermaidEls.push(cn.slice(0,80));
      if (typeof cn === 'string' && /math-inline/i.test(cn)) out.mathEls.push(cn.slice(0,80));
    });
    // svg count inside editor
    out.svgInPM = pm.querySelectorAll('svg').length;
    // marker-ish decorations: search for badge spans
    pm.querySelectorAll('[class*="marker"], [class*="badge"]').forEach(e => {
      out.markerEls.push(((e.className&&e.className.baseVal!==undefined?e.className.baseVal:e.className)||'')+'::'+(e.textContent||'').slice(0,20));
    });
    return out;
  })()`);
  console.log('DOM links:', JSON.stringify(dom.links, null, 1));
  console.log('mermaid els:', JSON.stringify(dom.mermaidEls));
  console.log('math els:', JSON.stringify(dom.mathEls));
  console.log('marker els:', JSON.stringify(dom.markerEls));
  console.log('svg in PM:', dom.svgInPM);

  // (b) Test wiki-link autocomplete: focus editor, clear to a fresh block, type [[
  const focusRes = await js(`(function(){
    const pm = document.querySelector('.ProseMirror[contenteditable="true"]');
    if (!pm) return 'no-pm';
    pm.focus();
    const sel = window.getSelection(); const range = document.createRange();
    range.selectNodeContents(pm); range.collapse(true);
    sel.removeAllRanges(); sel.addRange(range);
    return 'focused';
  })()`);
  console.log('focus:', focusRes);
  await sleep(300);

  // Type [[ via native keys
  const keys = [['[',1],['[',1]];
  const keyActions = [];
  for (const [ch] of keys) keyActions.push({type:'keyDown', value:ch},{type:'keyUp', value:ch});
  await driver.performActions([{ type:'key', id:'kb', actions:keyActions }]);
  await sleep(1200);

  const afterBrackets = await js(`return (function(){
    const pm = document.querySelector('.ProseMirror[contenteditable="true"]');
    const menus = Array.from(document.querySelectorAll('[role="menu"], .bn-suggestion-menu, [class*="suggestion"]')).map(e => ({
      cls: (e.className||'').slice(0,80), text: (e.textContent||'').slice(0,300),
    }));
    return { pmText: (pm?pm.innerText:'').slice(0,200), menus };
  })()`);
  console.log('AFTER [[ pmText:', JSON.stringify(afterBrackets.pmText));
  console.log('AFTER [[ menus:', JSON.stringify(afterBrackets.menus, null, 1));

  // (c) Test marker autocomplete: type ':' + TAB/new block, then ':'
  // Move to end of doc and add a new block, then type ':'
  const markerSetup = await js(`(function(){
    const pm = document.querySelector('.ProseMirror[contenteditable="true"]');
    if (!pm) return 'no-pm';
    pm.focus();
    const sel = window.getSelection(); const range = document.createRange();
    range.selectNodeContents(pm); range.collapse(false);
    sel.removeAllRanges(); sel.addRange(range);
    return 'end-focused';
  })()`);
  console.log('set end focus:', markerSetup);
  await sleep(200);
  // Enter to create new block
  await driver.performActions([{type:'key',id:'kb',actions:[{type:'keyDown',value:'\uE007'},{type:'keyUp',value:'\uE007'}]}]);
  await sleep(500);
  // type ':'
  await driver.performActions([{type:'key',id:'kb',actions:[{type:'keyDown',value:':'},{type:'keyUp',value:':'}]}]);
  await sleep(1200);
  const afterColon = await js(`return (function(){
    const menus = Array.from(document.querySelectorAll('[id*="suggestion"], [class*="suggestion"], [role="option"], [class*="menu"]')).map(e => ({
      cls: (e.className||'').slice(0,100), text: (e.textContent||'').slice(0,200),
    })).filter(m => m.text && !m.text.includes('Suggested'));
    const pm = document.querySelector('.ProseMirror[contenteditable="true"]');
    return { menus: menus.slice(0,12), pmText: (pm?pm.innerText:'').slice(-160) };
  })()`);
  console.log('AFTER : menus:', JSON.stringify(afterColon.menus, null, 1));
  console.log('AFTER : pmText:', JSON.stringify(afterColon.pmText));

  // Screenshot of whatever state we're in
  const png = await driver.takeScreenshot();
  fs.writeFileSync(OUT + '/probe2.png', Buffer.from(png, 'base64'));
  await driver.deleteSession().catch(()=>{});
  console.log('DONE probe2');
  process.exit(0);
}
main().catch((e) => { console.error('FATAL', e); process.exit(1); });
