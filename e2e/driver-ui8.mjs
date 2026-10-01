#!/usr/bin/env node
// Stratum acceptance UI walkthrough — v8 (robust nav, full body)
import WebDriver from 'webdriver';
import fs from 'fs';
const APP = '/home/shrtcrct/git/stratum/target/debug/stratum-tauri';
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const EVIDENCE = [];

async function main() {
  const driver = await WebDriver.newSession({
    hostname: '127.0.0.1', port: 4444, path: '/', protocol: 'http',
    connectionRetryTimeout: 30000, connectionRetryCount: 1,
    capabilities: { alwaysMatch: { browserName: 'wry', 'wdio:enforceWebDriverClassic': true,
      'webkitgtk:browserOptions': { binary: APP, args: ['--automation'] } } },
  });
  const rec = (label, result, detail='') => { EVIDENCE.push({label,result,detail:String(detail||'').slice(0,700)}); console.log(`[${result}] ${label}`); };
  const js = (s) => driver.executeScript(s, []);
  const body = async (n=1600) => { const t = await js(`return (document.body?.innerText||'').slice(0,${n});`); return (t||''); };

  let ready=false;
  for (let i=0;i<45;i++){ try{const t=await js('return (document.getElementById("root")&&document.getElementById("root").children.length)||0;'); if(t>0){ready=true;break;}}catch(e){} await sleep(2000);}
  if(!ready){ rec('boot','FAIL','no render'); await driver.deleteSession().catch(()=>{}); fs.writeFileSync('/tmp/stratum-ui-v8.json',JSON.stringify(EVIDENCE,null,2)); return; }
  await sleep(3000);

  const nav = async (label) => {
    await js(`(function(){var els=Array.from(document.querySelectorAll('*'));var best=null;var bestN=1e9;for(var i=0;i<els.length;i++){var e=els[i];if(e.children.length)continue;var t=(e.textContent||'').trim();if(t===null)continue;if(t==='${label}'&&e.children.length===0){best=e;break;}if(t.indexOf('${label}')>=0&&t.length<bestN&&e.children.length===0){best=e;bestN=t.length;}}if(best){best.click();return 'clicked:'+best.tagName;}return 'none';})()`);
    await sleep(2500);
    const url = await js('return location.href;');
    return { url, txt: await body() };
  };

  // Kanban
  let r = await nav('Kanban');
  rec('KN-01 kanban opens', /(to do|in progress|done|todo|kanban)/i.test(r.txt||'')?'OPENED':'CHECK', (r.url||'')+' :: '+(r.txt||'').slice(0,350));
  console.log('KN:', (r.txt||'').replace(/\n/g,' | ').slice(0,350));
  let png = await driver.takeScreenshot(); fs.writeFileSync('/tmp/shots/v8-KN-01.png', Buffer.from(png,'base64'));

  // Graph
  r = await nav('Graph');
  rec('GR-01 graph opens', /(graph|node|edge|orphan)/i.test(r.txt||'')?'OPENED':'CHECK', (r.url||'')+' :: '+(r.txt||'').slice(0,350));
  console.log('GR:', (r.txt||'').replace(/\n/g,' | ').slice(0,350));
  png = await driver.takeScreenshot(); fs.writeFileSync('/tmp/shots/v8-GR-01.png', Buffer.from(png,'base64'));

  // Search
  r = await nav('Search');
  rec('SR-01 search screen', /search/i.test(r.txt||'')?'OPENED':'CHECK', (r.url||'')+' :: '+(r.txt||'').slice(0,350));
  console.log('SR:', (r.txt||'').replace(/\n/g,' | ').slice(0,350));
  png = await driver.takeScreenshot(); fs.writeFileSync('/tmp/shots/v8-SR-01.png', Buffer.from(png,'base64'));

  // Query
  r = await nav('Query');
  rec('QY-01 query screen', /query/i.test(r.txt||'')?'OPENED':'CHECK', (r.url||'')+' :: '+(r.txt||'').slice(0,350));
  // reset
  const rc = await js(`(function(){var b=Array.from(document.querySelectorAll('button')).filter(function(x){return /reset/i.test(x.textContent||'');});b.forEach(function(x){x.click();});return b.length;})()`);
  await sleep(1500);
  const r2 = await nav('Query');
  rec('TK-03 reset example queries', rc>0 ? (r2.txt&&r2.txt.length>50?'LOADED':'CHECK') : 'NO-RESET', 'reset buttons: '+rc+' :: '+(r2.txt||'').slice(0,350));
  console.log('QY:', (r.txt||'').replace(/\n/g,' | ').slice(0,350));
  png = await driver.takeScreenshot(); fs.writeFileSync('/tmp/shots/v8-QY-01.png', Buffer.from(png,'base64'));

  // Templates
  r = await nav('Templates');
  rec('TM-04 templates screen', /(template|meeting)/i.test(r.txt||'')?'OPENED':'CHECK', (r.url||'')+' :: '+(r.txt||'').slice(0,350));
  console.log('TM:', (r.txt||'').replace(/\n/g,' | ').slice(0,350));
  png = await driver.takeScreenshot(); fs.writeFileSync('/tmp/shots/v8-TM.png', Buffer.from(png,'base64'));

  // Flashcards
  r = await nav('Flashcards');
  rec('FC screen', /(flash|card|review)/i.test(r.txt||'')?'OPENED':'CHECK', (r.url||'')+' :: '+(r.txt||'').slice(0,350));
  console.log('FC:', (r.txt||'').replace(/\n/g,' | ').slice(0,350));
  png = await driver.takeScreenshot(); fs.writeFileSync('/tmp/shots/v8-FC.png', Buffer.from(png,'base64'));

  // Settings
  r = await nav('Settings');
  rec('S settings screen', /(settings|vault|ai|sync|theme|plugin)/i.test(r.txt||'')?'OPENED':'CHECK', (r.url||'')+' :: '+(r.txt||'').slice(0,500));
  console.log('S:', (r.txt||'').replace(/\n/g,' | ').slice(0,500));
  png = await driver.takeScreenshot(); fs.writeFileSync('/tmp/shots/v8-S.png', Buffer.from(png,'base64'));

  await driver.deleteSession().catch(()=>{});
  fs.writeFileSync('/tmp/stratum-ui-v8.json', JSON.stringify(EVIDENCE,null,2));
  console.log('saved', EVIDENCE.length, 'items');
}
main().catch(async e=>{ console.error('FATAL', e.message); try{fs.writeFileSync('/tmp/stratum-ui-v8.json', JSON.stringify(EVIDENCE,null,2));}catch(_){} process.exit(1); });
