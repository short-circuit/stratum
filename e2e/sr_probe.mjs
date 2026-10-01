#!/usr/bin/env node
// Focused SR-01 probe: type 'monad', click Search, wait longer, dump the FULL result region HTML
import WebDriver from 'webdriver';
import fs from 'fs';
const APP = '/home/shrtcrct/git/stratum/target/debug/stratum-tauri';
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function main() {
  const driver = await WebDriver.newSession({
    hostname: '127.0.0.1', port: 4444, path: '/', protocol: 'http',
    connectionRetryTimeout: 30000, connectionRetryCount: 1,
    capabilities: { alwaysMatch: { browserName: 'wry', 'wdio:enforceWebDriverClassic': true,
      'webkitgtk:browserOptions': { binary: APP, args: ['--automation'] } } },
  });
  const js = (s) => driver.executeScript(s, []);

  let ready = false;
  for (let i = 0; i < 50; i++) { try { const t = await js('return (document.getElementById("root")?document.getElementById("root").children.length:0);'); if (t > 0) { ready = true; break; } } catch (e) {} await sleep(2000); }
  if (!ready) { console.log('BOOT FAIL'); await driver.deleteSession().catch(()=>{}); return; }
  await sleep(4000);

  // go to Search
  await js(`(function(){var els=Array.from(document.querySelectorAll('*'));for(var i=0;i<els.length;i++){var e=els[i];if(e.children.length)continue;if((e.textContent||'').trim()==='Search'){e.click();return;}}})()`);
  await sleep(2500);

  const typed = await js(`(function(){var inp=document.querySelector('input');if(!inp)return 'no-input';var setter=Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype,'value').set;setter.call(inp,'monad');inp.dispatchEvent(new Event('input',{bubbles:true}));inp.dispatchEvent(new Event('change',{bubbles:true}));return 'typed';})()`);
  await sleep(1500);
  await js(`(function(){var b=Array.from(document.querySelectorAll('button')).filter(function(x){return (x.textContent||'').trim()==='Search';});if(b.length){b[0].click();return b.length;}return 0;})()`);

  // wait, then dump innerText AND innerHTML of the main content area
  for (let w = 0; w < 5; w++) {
    await sleep(3000);
    const full = await js(`return (document.body?document.body.innerText:'');`);
    fs.writeFileSync(`/tmp/sr-probe-${w}.txt`, String(full||''));
    const html = await js(`return (document.body?document.body.innerHTML:'');`);
    fs.writeFileSync(`/tmp/sr-probe-${w}.html`, String(html||''));
    console.log(`[probe${w}] hasMonad=${(full||'').includes('monad')} len=${(full||'').length}`);
  }
  let png = await driver.takeScreenshot(); fs.writeFileSync('/tmp/sr-probe.png', Buffer.from(png,'base64'));
  await driver.deleteSession().catch(()=>{});
}
main().catch((e)=>{ console.error('FATAL', e.message); process.exit(1); });
