// Final live verification (t_b43e9453) — captures save indicator with a REAL edit.
import WebDriver from 'webdriver';
import fs from 'fs';

const APP = '/home/shrtcrct/git/stratum/target/debug/stratum-tauri';
const VAULT_FILE = '/tmp/stratum-vfy3b/home/StratumVault/notes/alpha-project.md';
const PROOF = {};
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

function fileState() {
  try {
    const s = fs.statSync(VAULT_FILE);
    const content = fs.readFileSync(VAULT_FILE, 'utf8');
    return { mtimeMs: s.mtimeMs, size: s.size, content };
  } catch (e) { return { error: String(e) }; }
}
const dbCnt = (c) => ({ doubleBrackets: (c.match(/\[\[\[\[/g) || []).length, embedDbl: (c.match(/\{\{embed \[\[\[\[/g) || []).length });

async function main() {
  const pre = fileState();
  PROOF.pre = { mtimeMs: pre.mtimeMs, size: pre.size, db: dbCnt(pre.content) };

  const driver = await WebDriver.newSession({
    hostname: '127.0.0.1', port: 4444, path: '/', protocol: 'http',
    connectionRetryTimeout: 30000, connectionRetryCount: 1,
    capabilities: { alwaysMatch: { browserName: 'wry', 'wdio:enforceWebDriverClassic': true,
      'webkitgtk:browserOptions': { binary: APP, args: ['--automation'] } } },
  });
  const js = (s) => driver.executeScript(s, []);
  const clickByText = async (label) => {
    const r = await js(`(function(){var els=Array.from(document.querySelectorAll('*'));var best=null;var bestN=1e9;for(var i=0;i<els.length;i++){var e=els[i];if(e.children.length)continue;var t=(e.textContent||'').trim();if(t==='${label}'){best=e;break;}if(t.indexOf('${label}')>=0&&t.length<bestN){best=e;bestN=t.length;}}if(best){best.click();return 'clicked';}return 'none:${label}';})()`);
    return r;
  };

  let ready = false;
  for (let i = 0; i < 48; i++) {
    try { const t = await js('return document.getElementById("root")?document.getElementById("root").children.length:0;'); if (t > 0) { ready = true; break; } } catch (e) {}
    await sleep(2000);
  }
  PROOF.boot = ready ? 'OK' : 'FAIL';
  if (!ready) { await driver.deleteSession().catch(()=>{}); finish(); return; }
  await sleep(3500);

  await clickByText('alpha-project');
  await sleep(3500);

  // A) idle wait — no edits
  await sleep(6000);
  const afterIdle = fileState();
  PROOF.afterIdle = { mtimeMs: afterIdle.mtimeMs, size: afterIdle.size, db: dbCnt(afterIdle.content) };
  PROOF.idleRewrite = afterIdle.mtimeMs !== pre.mtimeMs || afterIdle.size !== pre.size;

  // B) real edit — focus via JS (proven to work) then native key action
  let edited = 'none';
  try {
    const focusRes = await js(`(function(){var e=document.querySelector('.ProseMirror[contenteditable="true"]');if(!e)return 'no-el';e.focus();var sel=window.getSelection();var r=document.createRange();r.selectNodeContents(e);r.collapse(false);sel.removeAllRanges();sel.addRange(r);return 'focused';})()`);
    await sleep(300);
    await driver.performActions([{ type: 'key', id: 'kb', actions: [
      { type: 'keyDown', value: 'Y' }, { type: 'keyUp', value: 'Y' },
    ]}]);
    edited = focusRes;
  } catch (e) { edited = 'ERR:' + e.message; }
  PROOF.edited = edited;
  await sleep(2000);
  // Grab the save indicator DURING the save window (while saving=true)
  const uiDuring = await js(`(function(){var t=document.body?document.body.innerText:'';var m=t.match(/Saving…/);return m?m[0]:null;})()`).catch(()=> null);
  PROOF.saveIndicatorDuring = uiDuring;
  await sleep(4000); // let debounce+save complete
  const afterEdit = fileState();
  PROOF.afterEdit = { mtimeMs: afterEdit.mtimeMs, size: afterEdit.size, db: dbCnt(afterEdit.content) };
  PROOF.editSaved = afterEdit.mtimeMs !== afterIdle.mtimeMs || afterEdit.size !== afterIdle.size;
  PROOF.editContentHasY = (afterEdit.content || '').includes('Y');
  PROOF.editNoDoubleBracket = dbCnt(afterEdit.content).doubleBrackets === 0 && dbCnt(afterEdit.content).embedDbl === 0;
  // After the save completes, the indicator shows "Saved …time"
  await sleep(500);
  const uiAfter = await js(`(function(){var t=document.body?document.body.innerText:'';var m=t.match(/Saved [0-9:]+/);return m?m[0]:null;})()`).catch(()=> null);
  PROOF.saveIndicatorAfter = uiAfter;
  let png = await driver.takeScreenshot();
  fs.writeFileSync('/tmp/stratum-vfy3b/saved-indicator.png', Buffer.from(png, 'base64'));

  // C) backlinks expand
  await clickByText('Backlinks');
  await sleep(2500);
  const bl = await js(`(function(){var t=document.body?document.body.innerText:'';return t.slice(0,3000);})()`).catch(()=> '');
  PROOF.backlinksHasLinked = /linked references \(\d+\)/i.test(bl || '');
  PROOF.backlinksSample = (bl || '').split('\n').filter(l => /backlink|linked|mention|alpha/i.test(l)).slice(0, 12);
  png = await driver.takeScreenshot();
  fs.writeFileSync('/tmp/stratum-vfy3b/backlinks-expanded.png', Buffer.from(png, 'base64'));

  await driver.deleteSession().catch(()=>{});
  finish();
}

function finish() {
  console.log('=== LIVE VERIFICATION (t_b43e9453) FINAL ===');
  console.log('boot: ' + PROOF.boot);
  console.log('idleRewrite: ' + PROOF.idleRewrite + '  (must be false)');
  console.log('  pre db=' + JSON.stringify(PROOF.pre?.db) + '  idle db=' + JSON.stringify(PROOF.afterIdle?.db));
  console.log('edited: ' + PROOF.edited);
  console.log('editSaved: ' + PROOF.editSaved + '  contentHasY: ' + PROOF.editContentHasY + '  noDoubleBracket: ' + PROOF.editNoDoubleBracket);
  console.log('  post db=' + JSON.stringify(PROOF.afterEdit?.db) + '  size=' + PROOF.afterEdit?.size);
  console.log('saveIndicatorDuring: ' + JSON.stringify(PROOF.saveIndicatorDuring));
  console.log('saveIndicatorAfter: ' + JSON.stringify(PROOF.saveIndicatorAfter));
  console.log('backlinksHasLinked: ' + PROOF.backlinksHasLinked);
  console.log('  sample: ' + JSON.stringify(PROOF.backlinksSample));
  fs.writeFileSync('/tmp/stratum-vfy3b/proof.json', JSON.stringify(PROOF, null, 2));
  const pass = PROOF.boot === 'OK' && PROOF.idleRewrite === false && PROOF.editSaved === true && PROOF.editNoDoubleBracket === true && PROOF.backlinksHasLinked === true && (PROOF.saveIndicatorAfter !== null || PROOF.saveIndicatorDuring !== null);
  console.log('\nRESULT: ' + (pass ? 'PASS' : 'REVIEW'));
  console.log('proof: /tmp/stratum-vfy3b/proof.json');
  process.exit(pass ? 0 : 2);
}

main().catch(async (e) => { console.error('FATAL', e.message); try { fs.writeFileSync('/tmp/stratum-vfy3b/proof.json', JSON.stringify(PROOF, null, 2)); } catch (_) {} process.exit(1); });
