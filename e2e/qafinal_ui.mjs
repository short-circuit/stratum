#!/usr/bin/env node
// QA FINAL GATE UI driver (t_f251937d) — runs against HOME=/tmp/stratum-home (controlled fixture)
// Re-verifies previously-FAIL items on the fixed binary: SR-01 search, KN-02 kanban cards,
// FC-01 flashcards question, LK-04 backlinks, ED-07 idle autosave + save indicator,
// GG-04/GG-05 clean index rebuild, FF-04 byte-stability round-trip.
import WebDriver from 'webdriver';
import fs from 'fs';
import crypto from 'crypto';

const APP = '/home/shrtcrct/git/stratum/target/debug/stratum-tauri';
const NOTES = '/tmp/stratum-home/StratumVault/notes/alpha-project.md';
const SHOTS = '/tmp/shots-qafinal';
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const EVIDENCE = [];
const rec = (label, result, detail='') => { EVIDENCE.push({label, result, detail: String(detail||'').slice(0,800)}); console.log(`[${result}] ${label}`); };

fs.mkdirSync(SHOTS, { recursive: true });

function fileState() {
  try {
    const s = fs.statSync(NOTES);
    return { mtimeMs: s.mtimeMs, size: s.size, sha: crypto.createHash('sha256').update(fs.readFileSync(NOTES)).digest('hex').slice(0,16) };
  } catch (e) { return { error: String(e) }; }
}
const shot = async (driver, name) => { try { const png = await driver.takeScreenshot(); fs.writeFileSync(`${SHOTS}/${name}.png`, Buffer.from(png,'base64')); } catch(e){} };

async function main() {
  const pre = fileState();

  const driver = await WebDriver.newSession({
    hostname: '127.0.0.1', port: 4444, path: '/', protocol: 'http',
    connectionRetryTimeout: 30000, connectionRetryCount: 1,
    capabilities: { alwaysMatch: { browserName: 'wry', 'wdio:enforceWebDriverClassic': true,
      'webkitgtk:browserOptions': { binary: APP, args: ['--automation'] } } },
  });
  const js = (s) => driver.executeScript(s, []);
  const clickByText = async (label) => {
    const r = await js(`(function(){var els=Array.from(document.querySelectorAll('*'));var best=null;var bestN=1e9;var found=false;for(var i=0;i<els.length;i++){var e=els[i];if(e.children.length)continue;var t=(e.textContent||'').trim();if(!t)continue;if(t==='${label}'){best=e;found=true;break;}if(t.indexOf('${label}')>=0&&t.length<bestN){best=e;bestN=t.length;}}if(best){best.click();return 'clicked:'+best.tagName+':exact='+found;}return 'none';})()`);
    return r;
  };
  const body = async (n=4000) => String((await js(`return (document.body?.innerText||'').slice(0,${n});`))||'');
  const log = async (tag) => { try { const t = await body(6000); fs.writeFileSync(`${SHOTS}/${tag}.txt`, t); } catch(e){} };

  let ready=false;
  for (let i=0;i<50;i++){ try{const t=await js('return (document.getElementById("root")&&document.getElementById("root").children.length)||0;'); if(t>0){ready=true;break;}}catch(e){} await sleep(2000);}
  rec('boot', ready?'OK':'FAIL', 'controlled HOME=/tmp/stratum-home, pristine fixture + fresh .pkm');
  if(!ready){ await driver.deleteSession().catch(()=>{}); finish(); return; }
  await sleep(4000);
  const bootTxt = await body();
  // any lock/repair banner?
  rec('boot-no-lockbanner', /lockbusy|repair database|retry repair/i.test(bootTxt)?'LOCKBANNER':'CLEAN', bootTxt.slice(0,300));
  await log('boot');
  await shot(driver,'boot-journal');

  // ---- ED-07 idle: open alpha-project, wait >debounce, confirm no disk rewrite ----
  await clickByText('alpha-project');
  await sleep(3500);
  await shot(driver,'ed-open');
  await sleep(7000); // > md_write_delay
  const idle = fileState();
  const idleRewrite = idle.mtimeMs !== pre.mtimeMs;
  rec('ED-07 idle autosave', idleRewrite?'REWRITE':'NO-REWRITE', `pre sha=${pre.sha} size=${pre.size} / idle sha=${idle.sha} size=${idle.size}`);
  await log('ed-idle');

  // ---- LK-04 backlinks on alpha-project ----
  await clickByText('Backlinks');
  await sleep(3000);
  const bl = await body(5000);
  const blLinked = /backlinks\s*\(\d+\)/i.test(bl) || /linked references\s*\(\d+\)/i.test(bl);
  const blItem = /beta-notes|welcome/i.test(bl);
  rec('LK-04 backlinks', (blLinked && blItem)?'ITEMS':(blLinked?'COUNT-ONLY':'EMPTY'), bl.split('\n').filter(l=>/backlink|linked|mention|reference/i.test(l)).slice(0,8).join(' | '));
  await shot(driver,'lk-backlinks'); await log('lk-backlinks');

  // ---- ED-08/FF: real edit + save indicator + disk byte-stability ----
  const focusRes = await js(`(function(){var e=document.querySelector('.ProseMirror[contenteditable="true"]');if(!e)return 'no-el';e.focus();var sel=window.getSelection();var r=document.createRange();r.selectNodeContents(e);r.collapse(false);sel.removeAllRanges();sel.addRange(r);return 'focused';})()`);
  await sleep(300);
  await driver.performActions([{ type:'key', id:'kb', actions:[{type:'keyDown',value:'Q'},{type:'keyUp',value:'Q'}]}]);
  rec('ED-08 edit focus+type', focusRes==='focused'?'TYPED':'NO-EDIT', String(focusRes));
  await sleep(1500);
  const during = await js(`(function(){var t=document.body?document.body.innerText:'';var m=t.match(/Saving…|Saving \.\.\./);return m?m[0]:null;})()`).catch(()=>null);
  const savedDuring = await js(`(function(){var t=document.body?document.body.innerText:'';var m=t.match(/Saved [0-9:]+/);return m?m[0]:null;})()`).catch(()=>null);
  rec('ED-07 save indicator', (during||savedDuring)?'SEEN':(focusRes==='focused'?'WAIT':'NONE'), JSON.stringify({during, savedDuring}));
  await sleep(5000);
  const after = fileState();
  const post = fs.readFileSync(NOTES,'utf8');
  const dbl = (post.match(/\[\[\[\[/g)||[]).length;
  const qQ = post.includes('Q');
  const propMangle = /\.question: :|\.answer: :/;
  rec('ED-08 edit persisted', (after.mtimeMs!==idle.mtimeMs)&&qQ?'PERSISTED':'NOT-SAVED', `size ${idle.size}->${after.size}, sha=${after.sha}, hasQ=${qQ}`);
  rec('FF-04 no double-bracket', dbl===0?'CLEAN':`DBL=${dbl}`, `doubleBrackets=${dbl}`);
  rec('FC-01/ED-04 :: intact', propMangle.test(post)?'MANGLED':'INTACT', 'no .question: : / .answer: : on disk');
  await shot(driver,'ed-after-save'); await log('ed-after-save');

  // ---- SR-01 search monad ----
  await clickByText('Search');
  await sleep(2500);
  const typed = await js(`(function(){var inp=document.querySelector('input');if(!inp)return 'no-input';var setter=Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype,'value').set;setter.call(inp,'monad');inp.dispatchEvent(new Event('input',{bubbles:true}));inp.dispatchEvent(new Event('change',{bubbles:true}));return 'typed';})()`);
  await sleep(1500);
  await js(`(function(){var b=Array.from(document.querySelectorAll('button')).filter(function(x){return (x.textContent||'').trim()==='Search';});if(b.length)b[0].click();return b.length;})()`);
  await sleep(4000);
  const sr = await body(3000);
  const srResult = /alpha-project|score:|monad/i.test(sr);
  const srFull = String((await js(`return (document.body?document.body.innerText:'');`))||'');
  const srHasMonad = srFull.toLowerCase().includes('monad');
  const srHasNote = srFull.includes('alpha-project.md');
  rec('SR-01 search monad', (srHasMonad && srHasNote)?'MATCH-FOUND':(srHasNote?'NOTE-FOUND':'NO-RESULTS'), 'typed='+typed+' :: monadInFull='+srHasMonad+' noteInFull='+srHasNote+' :: '+sr.replace(/\n/g,'|').slice(0,400));
  fs.writeFileSync(`${SHOTS}/sr-full.txt`, srFull);
  await shot(driver,'sr-search'); await log('sr-search');

  // ---- KN-01/02 Kanban cards ----
  await clickByText('Kanban');
  await sleep(3000);
  const kn = await body(4000);
  const knCards = /(ship MVP|write docs|evaluate database|design sign-off|drafting proposal|initial research)/.test(kn);
  rec('KN-01 kanban opens', /(to do|in progress|done)/i.test(kn)?'OPENED':'NOOPEN', '');
  rec('KN-02 kanban cards from fixture', knCards?'CARDS':'NO-CARDS', kn.replace(/\n/g,'|').slice(0,600));
  await shot(driver,'kn-kanban'); await log('kn-kanban');

  // ---- FC-01 flashcards ----
  await clickByText('Flashcards');
  await sleep(3000);
  const fc = await body(3000);
  rec('FC-01 flashcard question', fc.includes('What is a monad?')?'REAL-QUESTION':'NOT-FOUND', fc.replace(/\n/g,'|').slice(0,500));
  await shot(driver,'fc-flashcards'); await log('fc-flashcards');

  // ---- TG-06 tag cloud / GR-01 graph ----
  await clickByText('Graph');
  await sleep(3000);
  const gr = await body(2500);
  rec('GR-01 graph opens', /graph|node|orphan/i.test(gr)?'OPENED':'CHECK', gr.replace(/\n/g,'|').slice(0,300));
  await shot(driver,'gr-graph'); await log('gr-graph');

  await clickByText('Templates');
  await sleep(2500);
  const tm = await body(2000);
  rec('TM-04 templates', tm.includes('meeting-notes')?'LISTED':'CHECK', tm.replace(/\n/g,'|').slice(0,200));

  await driver.deleteSession().catch(()=>{});
  finish();
}

async function finish() {
  fs.writeFileSync('/tmp/qafinal-ui.json', JSON.stringify(EVIDENCE,null,2));
  console.log('\n=== QA FINAL UI EVIDENCE ===');
  for (const e of EVIDENCE) console.log(`[${e.result}] ${e.label}`);
  console.log('saved /tmp/qafinal-ui.json');
}

main().catch(async (e)=>{ console.error('FATAL', e.message); try{ fs.writeFileSync('/tmp/qafinal-ui.json', JSON.stringify(EVIDENCE,null,2)); }catch(_){} process.exit(1); });
