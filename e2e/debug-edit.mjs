// Debug: inspect editor DOM + attempt native key-based edit
import WebDriver from 'webdriver';
import fs from 'fs';

const APP = '/home/shrtcrct/git/stratum/target/debug/stratum-tauri';
const VAULT_FILE = '/tmp/stratum-vfy2/home/StratumVault/notes/alpha-project.md';
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

function fileState() {
  try {
    const s = fs.statSync(VAULT_FILE);
    return { mtimeMs: s.mtimeMs, size: s.size, content: fs.readFileSync(VAULT_FILE, 'utf8') };
  } catch (e) { return { error: String(e) }; }
}

async function main() {
  const driver = await WebDriver.newSession({
    hostname: '127.0.0.1', port: 4444, path: '/', protocol: 'http',
    connectionRetryTimeout: 30000, connectionRetryCount: 1,
    capabilities: { alwaysMatch: { browserName: 'wry', 'wdio:enforceWebDriverClassic': true,
      'webkitgtk:browserOptions': { binary: APP, args: ['--automation'] } } },
  });
  const js = (s) => driver.executeScript(s, []);
  const body = async (n = 900) => { try { const t = await js(`return (document.body?.innerText||'').slice(0,${n});`); return (t || ''); } catch (e) { return ''; } };

  let ready = false;
  for (let i = 0; i < 48; i++) {
    try { const t = await js('return (document.getElementById("root")&&document.getElementById("root").children.length)||0;'); if (t > 0) { ready = true; break; } } catch (e) {}
    await sleep(2000);
  }
  if (!ready) { console.log('BOOT FAIL'); await driver.deleteSession().catch(()=>{}); return; }
  await sleep(3500);

  // click sidebar alpha-project
  await js(`(function(){var els=Array.from(document.querySelectorAll('*'));var best=null;var bestN=1e9;for(var i=0;i<els.length;i++){var e=els[i];if(e.children.length)continue;var t=(e.textContent||'').trim();if(t==='alpha-project'){best=e;break;}if(t.indexOf('alpha-project')>=0&&t.length<bestN){best=e;bestN=t.length;}}if(best){best.click();return 'clicked';}return 'none';})()`);
  await sleep(3500);

  // Inspect the editor DOM structure
  const dom = await js(`
    (function(){
      var out = {};
      var ed = document.querySelector('.blocknote-editor-container');
      out.containerClass = ed ? ed.className : 'NONE';
      var ce = document.querySelector('[contenteditable="true"]');
      out.contentEditable = ce ? (ce.className||'')+' :: '+ce.tagName : 'NONE';
      var pm = document.querySelector('.ProseMirror');
      out.proseMirror = pm ? (pm.className||'')+' :: '+(pm.getAttribute('contenteditable')) : 'NONE';
      var bn = document.querySelector('.bn-editor');
      out.bnEditor = bn ? bn.className : 'NONE';
      out.editors = Array.from(document.querySelectorAll('[contenteditable="true"]')).map(function(e){return e.className;});
      return out;
    })()
  `);
  console.log('DOM:', JSON.stringify(dom));

  // Try focusing the contenteditable and sending real key presses via WebDriver actions
  let elemId = null;
  try {
    elemId = await driver.findElement('css selector', '.ProseMirror[contenteditable="true"], [contenteditable="true"]');
    console.log('ELEM ID:', elemId);
    await sleep(500);
    // Click to focus
    const clickedScope = await js(`(function(){var e=document.querySelector('.ProseMirror[contenteditable="true"], [contenteditable="true"]'); if(!e) return 'none'; e.focus(); var sel=window.getSelection(); var r=document.createRange(); r.selectNodeContents(e); r.collapse(false); sel.removeAllRanges(); sel.addRange(r); return 'focused';})()`);
    console.log('FOCUS:', clickedScope);
    await sleep(500);
    // Type via WebDriver actions protocol
    await driver.performActions([{
      type: 'key', id: 'kb',
      actions: [
        { type: 'keyDown', value: 'Z' },
        { type: 'keyUp', value: 'Z' },
      ],
    }]);
    await sleep(800);
    const txt = await body(1400);
    console.log('AFTER TYPE contains Z?', txt.includes('Z'));
    console.log('AFTER TYPE sample:', txt.slice(0, 200).replace(/\n/g, ' | '));
  } catch (e) {
    console.log('TYPE ERR:', e.message);
  }

  // Now wait for autosave and check the file
  await sleep(4500);
  const st = fileState();
  console.log('FILE after edit:', JSON.stringify({ mtimeMs: st.mtimeMs, size: st.size, hasZ: (st.content||'').includes('Z'), db: ((st.content||'').match(/\[\[\[\[/g)||[]).length }));

  await driver.deleteSession().catch(()=>{});
}
main().catch(async (e) => { console.error('FATAL', e.message); process.exit(1); });
