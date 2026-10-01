#!/usr/bin/env node
// Stratum acceptance UI walkthrough — v7 (all feature screens)
// Operates on a FRESH home vault each run (caller re-copies fixture).
import WebDriver from 'webdriver';
import fs from 'fs';
const APP = '/home/shrtcrct/git/stratum/target/debug/stratum-tauri';
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const EVIDENCE = [];
const SHOTS = [];

async function main() {
  const driver = await WebDriver.newSession({
    hostname: '127.0.0.1', port: 4444, path: '/', protocol: 'http',
    connectionRetryTimeout: 30000, connectionRetryCount: 1,
    capabilities: { alwaysMatch: {
      browserName: 'wry', 'wdio:enforceWebDriverClassic': true,
      'webkitgtk:browserOptions': { binary: APP, args: ['--automation'] },
    } },
  });
  const rec = (label, result, detail='') => { EVIDENCE.push({label,result,detail:String(detail||'').slice(0,400)}); console.log(`[${result}] ${label}`); };

  let ready=false;
  for (let i=0;i<45;i++){ try{const t=await driver.executeScript('return (document.getElementById("root")&&document.getElementById("root").children.length)||0;',[]); if(t>0){ready=true;break;}}catch(e){} await sleep(2000);}
  if(!ready){ rec('boot','FAIL','no render'); await driver.deleteSession().catch(()=>{}); save(); return; }
  await sleep(3000);

  const nav = async (label) => {
    // click nav item by text
    await driver.executeScript(`(function(){var els=Array.from(document.querySelectorAll('div,span,a,button'));var t=els.filter(function(e){return e.children.length===0&&(e.textContent||'').trim()==='${label}';});if(t.length)t[0].click();return t.length;})()`, []);
    await sleep(2500);
    const url = await driver.executeScript('return location.href;', []);
    const txt = await driver.executeScript(`return (document.body?.innerText||'').slice(0,1200);`, []);
    return { url, txt };
  };

  // ============ 1. Kanban ============
  try {
    let r = await nav('Kanban');
    rec('KN-01 kanban opens', (r.txt||'').includes('To Do')||(r.txt||'').includes('TODO') ? 'OPENED':'CHECK', r.url +' :: '+ (r.txt||'').slice(0,300));
    const shot = await driver.takeScreenshot(); SHOTS.push({label:'KN-01-kanban', png:shot});
    console.log('  KN body:', (r.txt||'').replace(/\n/g,' | ').slice(0,300));
  } catch(e){ rec('KN-01 kanban opens','ERROR',e.message); }

  // ============ 2. Graph ============
  try {
    let r = await nav('Graph');
    rec('GR-01 graph opens', (r.txt||'').includes('Graph') ? 'OPENED':'CHECK', r.url +' :: '+ (r.txt||'').slice(0,300));
    const shot = await driver.takeScreenshot(); SHOTS.push({label:'GR-01-graph', png:shot});
    console.log('  GR body:', (r.txt||'').replace(/\n/g,' | ').slice(0,300));
  } catch(e){ rec('GR-01 graph opens','ERROR',e.message); }

  // ============ 3. Search ============
  try {
    let r = await nav('Search');
    rec('SR-01 search screen', (r.txt||'').includes('Search') ? 'OPENED':'CHECK', r.url +' :: '+ (r.txt||'').slice(0,300));
    const shot = await driver.takeScreenshot(); SHOTS.push({label:'SR-01-search', png:shot});
    console.log('  SR body:', (r.txt||'').replace(/\n/g,' | ').slice(0,260));
  } catch(e){ rec('SR-01 search screen','ERROR',e.message); }

  // ============ 4. Query ============
  try {
    let r = await nav('Query');
    rec('QY-01 query screen', (r.txt||'').includes('Query')||(r.txt||'').toLowerCase().includes('query') ? 'OPENED':'CHECK', r.url +' :: '+ (r.txt||'').slice(0,300));
    // try reset button
    await driver.executeScript(`(function(){var els=Array.from(document.querySelectorAll('button'));var t=els.filter(function(e){return /reset/i.test(e.textContent||'');});t.forEach(function(b){b.click();});return t.length;})()`, []);
    await sleep(1500);
    const r2 = await nav('Query');
    rec('TK-03 reset example queries', (r2.txt||'').length>0 ? 'CHECK' : 'CHECK', 'after reset: '+(r2.txt||'').slice(0,300));
    const shot = await driver.takeScreenshot(); SHOTS.push({label:'QY-01-query', png:shot});
  } catch(e){ rec('QY-01 query screen','ERROR',e.message); }

  // ============ 5. Templates ============
  try {
    let r = await nav('Templates');
    rec('TM-04 templates screen', (r.txt||'').toLowerCase().includes('template')||(r.txt||'').includes('meeting') ? 'OPENED':'CHECK', r.url +' :: '+ (r.txt||'').slice(0,300));
    const shot = await driver.takeScreenshot(); SHOTS.push({label:'TM-templates', png:shot});
    console.log('  TM body:', (r.txt||'').replace(/\n/g,' | ').slice(0,260));
  } catch(e){ rec('TM-04 templates screen','ERROR',e.message); }

  // ============ 6. Flashcards ============
  try {
    let r = await nav('Flashcards');
    rec('FC-01 flashcards screen', (r.txt||'').toLowerCase().includes('flash')||(r.txt||'').toLowerCase().includes('card')||(r.txt||'').toLowerCase().includes('review') ? 'OPENED':'CHECK', r.url +' :: '+ (r.txt||'').slice(0,300));
    const shot = await driver.takeScreenshot(); SHOTS.push({label:'FC-flashcards', png:shot});
    console.log('  FC body:', (r.txt||'').replace(/\n/g,' | ').slice(0,260));
  } catch(e){ rec('FC-01 flashcards screen','ERROR',e.message); }

  // ============ 7. Settings ============
  try {
    let r = await nav('Settings');
    rec('S-01 settings screen', /(settings|vault|ai|sync|theme|appearance)/i.test(r.txt||'') ? 'OPENED':'CHECK', r.url +' :: '+ (r.txt||'').slice(0,400));
    const shot = await driver.takeScreenshot(); SHOTS.push({label:'S-settings', png:shot});
    console.log('  S body:', (r.txt||'').replace(/\n/g,' | ').slice(0,400));
  } catch(e){ rec('S-01 settings screen','ERROR',e.message); }

  // ============ Save evidence ============
  await driver.deleteSession().catch(()=>{});
  save();
}
function save(){
  fs.writeFileSync('/tmp/stratum-ui-v7.json', JSON.stringify(EVIDENCE,null,2));
  for(const s of SHOTS){ fs.writeFileSync('/tmp/shots/'+s.label+'.png', Buffer.from(s.png,'base64')); }
  console.log('saved evidence', EVIDENCE.length, 'items,', SHOTS.length, 'shots');
}
main().catch(async e=>{ console.error('FATAL', e.message); try{fs.writeFileSync('/tmp/stratum-ui-v7.json', JSON.stringify(EVIDENCE,null,2));}catch(_){} process.exit(1); });
