#!/usr/bin/env node
/**
 * E7.F1 phase-1 probe: boot the app against the controlled fixture and
 * inspect editor state — block structure, markers, wiki-links, math/mermaid,
 * backlinks presence. Pure observation (no edits) first.
 */
import WebDriver from 'webdriver';
import fs from 'fs';

const APP = '/home/shrtcrct/git/stratum/target/debug/stratum-tauri';
const OUT = '/tmp/stratum-f1-probe1';
const HOST = '127.0.0.1', PORT = 4444;
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

fs.mkdirSync(OUT, { recursive: true });

async function main() {
  const driver = await WebDriver.newSession({
    hostname: HOST, port: PORT, path: '/', protocol: 'http',
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

  // Root landing — what does it show?
  const boot = await js(`return {
    url: location.href,
    title: document.title,
    body: (document.body.innerText||'').slice(0,600),
  };`);
  console.log('URL:', boot.url);
  console.log('TITLE:', boot.title);
  console.log('BODY:', JSON.stringify(boot.body.slice(0,400)));

  // Navigate to the editor page f1-editor
  await js(`(function(){ history.pushState({},'', '/page/notes%2Ff1-editor.md'); dispatchEvent(new PopStateEvent('popstate')); return 'ok'; })()`);
  await sleep(5000);
  const editor = await js(`return {
    path: location.pathname,
    hasProseMirror: !!document.querySelector('.ProseMirror[contenteditable="true"]'),
    pmText: (document.querySelector('.ProseMirror[contenteditable="true"]')?.innerText||'').slice(0,2000),
    mathRendered: !!document.querySelector('.math-inline-rendered'),
    mathCount: document.querySelectorAll('.math-inline-rendered').length,
    mermaid: !!document.querySelector('.mermaid, [class*="mermaid"]'),
    mermaidCount: document.querySelectorAll('.mermaid, svg').length,
    wikiLinks: Array.from(document.querySelectorAll('a')).filter(a=>a.textContent.includes('[[ Alpha Project')).length,
    scriptTagsInBody: document.querySelectorAll('script').length,
  };`);
  console.log('PATH:', editor.path);
  console.log('HAS_EDITOR:', editor.hasProseMirror);
  console.log('MATH_RENDERED:', editor.mathRendered, 'count', editor.mathCount);
  console.log('MERMAID:', editor.mermaid, 'svg count', editor.mermaidCount);
  console.log('WIKILINKS:', editor.wikiLinks);
  console.log('PM_TEXT (first 1800):', JSON.stringify((editor.pmText||'').slice(0,1800)));

  const png = await driver.takeScreenshot();
  fs.writeFileSync(OUT + '/probe1.png', Buffer.from(png, 'base64'));

  // Check console messages
  try {
    const logs = await driver.getLogs('browser');
    console.log('CONSOLE_LOGS:', JSON.stringify((logs||[]).slice(-10)));
  } catch (e) { console.log('CONSOLE_LOGS: n/a', e.message); }

  await driver.deleteSession().catch(()=>{});
  console.log('DONE probe1');
  process.exit(0);
}
main().catch((e) => { console.error('FATAL', e); process.exit(1); });
