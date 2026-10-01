// Determinism test: open a note, wait 8s without typing, check disk
import WebDriver from 'webdriver';
import fs from 'fs';
const APP = '/home/shrtcrct/git/stratum/target/debug/stratum-tauri';
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

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
  await driver.executeScript('location.href = "tauri://localhost/page/notes%2Falpha-project.md";', []);
  await sleep(9000);   // no typing, just opening
  const body = await driver.executeScript(`return (document.body?.innerText||'').slice(0,700);`, []);
  console.log('ALPHA OPENED BODY:', (body||'').replace(/\n/g,' | '));
  const png = await driver.takeScreenshot(); fs.writeFileSync('/tmp/alpha-opened.png', Buffer.from(png,'base64'));
  console.log('shot');
  await driver.deleteSession().catch(()=>{});
  console.log('DONE — now check disk');
}
main().catch(e=>{console.error('FATAL', e.message); process.exit(1);});
