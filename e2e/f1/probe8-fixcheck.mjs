#!/usr/bin/env node
/** E7.F1 probe8: verify watcher starts on a default vault (no config.toml) after the fix.
 * Launch binary, capture its stderr for 18s, externally modify a file mid-run, and
 * confirm via get_blocks that the external edit synced.
 * Uses tauri-driver (already running) — but here we need app stderr, so we run the
 * app directly under the driver-less approach via WebDriver session and capture
 * process stderr through WebKitWebDriver logs is hard. Instead: rely on get_blocks.
 */
import WebDriver from 'webdriver';
import fs from 'fs';
const APP='/home/shrtcrct/git/stratum/target/debug/stratum-tauri';
const HOME=fs.readFileSync('/tmp/f3dir.txt','utf8').trim();
const VAULT=`${HOME}/StratumVault`;
const F=`${VAULT}/notes/f1-editor.md`;
const sleep=ms=>new Promise(r=>setTimeout(r,ms));
const r=[]; const rec=(l,p,d='')=>{r.push({l,p:!!p,d:String(d||'').slice(0,300)});console.log(`[${p?'PASS':'FAIL'}] ${l}${d?' — '+d:''}`);};
async function main(){
  const driver=await WebDriver.newSession({hostname:'127.0.0.1',port:4444,path:'/',protocol:'http',connectionRetryTimeout:60000,connectionRetryCount:2,capabilities:{alwaysMatch:{browserName:'wry','wdio:enforceWebDriverClassic':true,'webkitgtk:browserOptions':{binary:APP,args:['--automation']}}}});
  const js=s=>{let w=s;if(/^\s*\(function\(\)\{[\s\S]*\}\)\(\)\s*$/.test(w))w=w.replace(/^\s*\(function\(\)\{([\s\S]*)\}\)\(\)\s*$/,'return (function(){ $1 })();');else if(!/^\s*return /.test(w))w=`return (function(){${w}})();`;return driver.executeScript(w,[]);};
  let ready=false; for(let i=0;i<90;i++){try{const t=await js('return document.getElementById("root")?document.getElementById("root").children.length:0;');if(t>0){ready=true;break;}}catch(e){} await sleep(2000);}
  rec('boot',ready);
  if(!ready){await driver.deleteSession().catch(()=>{});process.exit(1);}
  await sleep(6000);
  // external modify
  const ext='F1-FIXED-WATCH-'+Date.now();
  fs.appendFileSync(F,`\n- ${ext}\n  .id: bbbbbbbb-0000-0000-0000-0000000000bb\n`);
  await sleep(7000);
  const g=await js(`return (function(){var i=window.__TAURI_INTERNALS__;return i.invoke('get_blocks',{pagePath:'notes/f1-editor.md'}).then(v=>({ok:true,cnt:v.blocks.length,has:v.blocks.some(b=>(b.content||'').indexOf('F1-FIXED-WATCH-')>=0)}),e=>({ok:false,e:String(e)}));})()`);
  rec('watcher(external modify) synced after fix', g.ok&&g.has, JSON.stringify(g));
  const s=await js(`return (function(){var i=window.__TAURI_INTERNALS__;return i.invoke('search_blocks',{query:'F1-FIXED-WATCH-',limit:3}).then(v=>({ok:true,hits:(v.results||[]).length}),e=>({ok:false,e:String(e)}));})()`);
  rec('watcher(external modify) searchable after fix', s.ok&&s.hits>0, JSON.stringify(s));
  // new page
  const np='np-'+Date.now();
  fs.writeFileSync(`${VAULT}/notes/${np}.md`,`---\ntitle: ${np}\n---\nbody ${ext}\n`);
  await sleep(7000);
  const lp=await js(`return (function(){var i=window.__TAURI_INTERNALS__;return i.invoke('list_pages').then(v=>({ok:true,has:v.pages.some(p=>p.path.indexOf('np-')>=0)}),e=>({ok:false,e:String(e)}));})()`);
  rec('watcher(new page) appears after fix', lp.ok&&lp.has, JSON.stringify(lp));
  await driver.deleteSession().catch(()=>{});
  fs.writeFileSync('/tmp/stratum-f1-probe8/r.json',JSON.stringify(r,null,2));
  const fails=r.filter(x=>!x.p);
  console.log('\n==== TOTAL '+r.length+'/FAIL '+fails.length+' ====');
  process.exit(fails.length?2:0);
}
main().catch(e=>{console.error('FATAL',e);process.exit(1);});
