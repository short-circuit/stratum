import WebDriver from 'webdriver';
import fs from 'fs';
const APP = '/home/shrtcrct/git/stratum/target/debug/stratum-tauri';
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const out = '/tmp/e2e-type-result.json';
let log = [];

async function main() {
  const driver = await WebDriver.newSession({
    hostname: '127.0.0.1', port: 4444, path: '/', protocol: 'http',
    connectionRetryTimeout: 30000, connectionRetryCount: 1,
    capabilities: { alwaysMatch: {
      browserName: 'wry', 'wdio:enforceWebDriverClassic': true,
      'webkitgtk:browserOptions': { binary: APP, args: ['--automation'] },
    } },
  });
  let ready=false;
  for (let i=0;i<40;i++){ try{const t=await driver.executeScript('return (document.getElementById("root")&&document.getElementById("root").children.length)||0;',[]); if(t>0){ready=true;break;}}catch(e){} await sleep(2000);}
  if(!ready){console.log('NEVER');process.exit(1);}
  await driver.executeScript('location.href = "tauri://localhost/page/notes%2Fwelcome.md";', []);
  await sleep(6000);

  const before = await driver.executeScript(`return (document.body?.innerText||'').slice(0,500);`, []);
  log.push({step:'before-type', body: before});
  console.log('BEFORE:', before.replace(/\n/g,' | '));

  // Type via execCommand append at end of the last block
  await driver.executeScript(`(function(){
    var bn = document.querySelector('[data-editor-type]') || document.querySelector('[contenteditable="true"]');
    if(!bn){ return 'no-editor'; }
    bn.focus();
    var sel = window.getSelection();
    var r = document.createRange();
    r.selectNodeContents(bn);
    r.collapse(false);
    sel.removeAllRanges(); sel.addRange(r);
    document.execCommand('insertText', false, 'EXTRA');
    return 'typed-into:'+bn.tagName;
  })()`, []);
  await sleep(3000);
  const after = await driver.executeScript(`return (document.body?.innerText||'').slice(0,500);`, []);
  log.push({step:'after-type', body: after});
  console.log('AFTER:', (after||'').replace(/\n/g,' | '));
  const png = await driver.takeScreenshot(); fs.writeFileSync('/tmp/ed-after-type.png', Buffer.from(png,'base64'));

  await driver.deleteSession().catch(()=>{});
  fs.writeFileSync(out, JSON.stringify(log,null,2));
  console.log('log ->', out);
}
main().catch(async e=>{console.error('FATAL', e.message); fs.writeFileSync(out, JSON.stringify({fatal:e.message, log},null,2)); process.exit(1);});
