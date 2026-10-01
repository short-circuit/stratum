#!/usr/bin/env node
// v10 - Search E2E: type query, click Search, inspect results
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
  const body = async (n=1500) => String((await js(`return (document.body?.innerText||'').slice(0,${n});`))||'');

  let ready=false;
  for (let i=0;i<45;i++){ try{const t=await js('return (document.getElementById("root")&&document.getElementById("root").children.length)||0;'); if(t>0){ready=true;break;}}catch(e){} await sleep(2000);}
  if(!ready){ rec('boot','FAIL','no render'); await driver.deleteSession().catch(()=>{}); fs.writeFileSync('/tmp/stratum-ui-v10.json',JSON.stringify(EVIDENCE,null,2)); return; }
  await sleep(3000);

  // go to search
  await js(`(function(){var els=Array.from(document.querySelectorAll('*'));for(var i=0;i<els.length;i++){var e=els[i];if(e.children.length)continue;if((e.textContent||'').trim()==='Search'){e.click();return;}}})()`);
  await sleep(2500);
  // find input and set value + dispatch
  const setres = await js(`(function(){
    var inp = document.querySelector('input');
    if(!inp) return 'no-input';
    var setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype,'value').set;
    setter.call(inp,'monad');
    inp.dispatchEvent(new Event('input',{bubbles:true}));
    return 'typed:'+inp.value;
  })()`);
  await sleep(1200);
  // click Search button
  const clickres = await js(`(function(){var b=Array.from(document.querySelectorAll('button')).filter(function(x){return (x.textContent||'').trim()==='Search';});if(b.length){b[0].click();return 'clicked:'+b.length;}return 'no-btn';})()`);
  await sleep(3500);
  const res = await body();
  rec('SR-01 search monad', res.toLowerCase().includes('monad')?'RESULTS':'NO-RESULTS', 'set:'+setres+' click:'+clickres+' :: '+res.slice(0,500));
  console.log('SR res:', (res||'').replace(/\n/g,' | ').slice(0,500));
  let png = await driver.takeScreenshot(); fs.writeFileSync('/tmp/shots/v10-search.png', Buffer.from(png,'base64'));

  // try tag search
  await js(`(function(){
    var inp = document.querySelector('input');
    if(!inp) return;
    var setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype,'value').set;
    setter.call(inp,'#rust');
    inp.dispatchEvent(new Event('input',{bubbles:true}));
  })()`);
  await sleep(800);
  await js(`(function(){var b=Array.from(document.querySelectorAll('button')).filter(function(x){return (x.textContent||'').trim()==='Search';});if(b.length)b[0].click();})()`);
  await sleep(3000);
  const res2 = await body();
  rec('SR-02 tag search', res2.toLowerCase().includes('rust')?'RESULTS':'NO-RESULTS', 'body: '+res2.slice(0,500));
  console.log('SR2:', (res2||'').replace(/\n/g,' | ').slice(0,500));

  await driver.deleteSession().catch(()=>{});
  fs.writeFileSync('/tmp/stratum-ui-v10.json', JSON.stringify(EVIDENCE,null,2));
  console.log('saved');
}
main().catch(async e=>{ console.error('FATAL', e.message); try{fs.writeFileSync('/tmp/stratum-ui-v10.json', JSON.stringify(EVIDENCE,null,2));}catch(_){} process.exit(1); });
