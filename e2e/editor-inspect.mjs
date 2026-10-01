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
  await driver.executeScript('location.href = "tauri://localhost/page/notes%2Fwelcome.md";', []);
  await sleep(6000);
  console.log('URL:', await driver.executeScript('return location.href;', []));

  // Inspect ALL contenteditable elements and their innerText
  const eds = await driver.executeScript(`JSON.stringify(Array.from(document.querySelectorAll('[contenteditable]')).map(function(e,i){return {i,tag:e.tagName,cls:String(e.className||'').slice(0,50),inner:String(e.innerText||'').slice(0,200),ph:e.getAttribute('data-placeholder')||''};}))`, []);
  console.log('EDITABLES:\n', eds);

  // Check whether BlockNote content area shows the real blocks
  const body = await driver.executeScript(`return (document.body?.innerText||'').slice(0,600);`, []);
  console.log('BODY:\n', body);

  // list blocknote blocks
  const blocks = await driver.executeScript(`JSON.stringify(Array.from(document.querySelectorAll('.bn-block')).map(function(e,i){return {i,txt:String(e.innerText||'').slice(0,80),cls:String(e.className||'').slice(0,40)};}))`, []);
  console.log('BN-BLOCKS:\n', blocks);

  const png = await driver.takeScreenshot();
  fs.writeFileSync('/tmp/ed-welcome-inspect.png', Buffer.from(png,'base64'));
  console.log('shot'); 
  try{await driver.deleteSession();}catch(e){}
}
main().catch(e=>{console.error('FATAL',e.message);process.exit(1);});
