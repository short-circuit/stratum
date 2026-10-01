#!/usr/bin/env node
// v9 - Ask Notes, Journal nav, note with backlinks, tags panel
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
  const rec = (label, result, detail='') => { EVIDENCE.push({label,result,detail:String(detail||'').slice(0,600)}); console.log(`[${result}] ${label}`); };
  const js = (s) => driver.executeScript(s, []);
  const body = async (n=1400) => String((await js(`return (document.body?.innerText||'').slice(0,${n});`))||'');

  let ready=false;
  for (let i=0;i<45;i++){ try{const t=await js('return (document.getElementById("root")&&document.getElementById("root").children.length)||0;'); if(t>0){ready=true;break;}}catch(e){} await sleep(2000);}
  if(!ready){ rec('boot','FAIL','no render'); await driver.deleteSession().catch(()=>{}); fs.writeFileSync('/tmp/stratum-ui-v9.json',JSON.stringify(EVIDENCE,null,2)); return; }
  await sleep(3000);

  const nav = async (label) => {
    await js(`(function(){var els=Array.from(document.querySelectorAll('*'));for(var i=0;i<els.length;i++){var e=els[i];if(e.children.length)continue;if((e.textContent||'').trim()==='${label}'){e.click();return;}}})()`);
    await sleep(2500);
    return { url: await js('return location.href;'), txt: await body() };
  };

  // Ask Notes
  let r = await nav('Ask Notes');
  rec('ASK ask-notes screen', /(ask|error|provider|write|question|response)/i.test(r.txt||'')?'OPENED':'CHECK', (r.url||'')+' :: '+(r.txt||'').slice(0,400));
  console.log('ASK:', (r.txt||'').replace(/\n/g,' | ').slice(0,400));
  let png = await driver.takeScreenshot(); fs.writeFileSync('/tmp/shots/v9-ASK.png', Buffer.from(png,'base64'));

  // open alpha-project (the most-linked note) to check backlinks
  await js('location.href="tauri://localhost/page/notes%2Falpha-project.md";'); await sleep(5000);
  const ab = await body();
  rec('LK-04 backlinks panel on alpha', /backlink/i.test(ab||'')?'PANEL-PRESENT':(/(welcome|beta)/i.test(ab||'')?'CHECK':'NONE'), 'alpha body: '+(ab||'').slice(0,600));
  console.log('ALPHA:', (ab||'').replace(/\n/g,' | ').slice(0,600));
  png = await driver.takeScreenshot(); fs.writeFileSync('/tmp/shots/v9-alpha-backlinks.png', Buffer.from(png,'base64'));

  // Journal nav: prev/next day buttons
  r = await nav('Journal');
  rec('JN-01 journal view', (r.url||'').includes('/journal')?'OPENED':'CHECK', (r.url||'')+' :: '+(r.txt||'').slice(0,300));
  console.log('JN:', (r.txt||'').replace(/\n/g,' | ').slice(0,300));
  png = await driver.takeScreenshot(); fs.writeFileSync('/tmp/shots/v9-JN.png', Buffer.from(png,'base64'));

  await driver.deleteSession().catch(()=>{});
  fs.writeFileSync('/tmp/stratum-ui-v9.json', JSON.stringify(EVIDENCE,null,2));
  console.log('saved', EVIDENCE.length, 'items');
}
main().catch(async e=>{ console.error('FATAL', e.message); try{fs.writeFileSync('/tmp/stratum-ui-v9.json', JSON.stringify(EVIDENCE,null,2));}catch(_){} process.exit(1); });
