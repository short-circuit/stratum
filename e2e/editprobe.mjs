import WebDriver from 'webdriver';
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
  let ready = false;
  for (let i = 0; i < 40; i++) {
    try { const t = await driver.executeScript('return (document.getElementById("root") && document.getElementById("root").children.length) || 0;', []); if (t>0){ready=true;break;} } catch(e){}
    await sleep(2000);
  }
  if (!ready) { console.log('NEVER'); process.exit(1); }

  // open welcome note by navigating URL directly
  await driver.executeScript('location.href = "tauri://localhost/page/notes%2Fwelcome.md";', []);
  await sleep(4000);
  console.log('URL:', await driver.executeScript('return location.href;', []));

  // list editable elements
  const ed = await driver.executeScript(`JSON.stringify(Array.from(document.querySelectorAll('[contenteditable]')).map(function(e,i){return {i,tag:e.tagName,role:e.getAttribute('role'),cls:String(e.className||'').slice(0,60),ph:e.getAttribute('aria-label')||e.getAttribute('placeholder')||'',txt:String(e.innerText||'').slice(0,60)};}))`, []);
  console.log('EDITABLES:', ed);

  // list textarea/input
  const in2 = await driver.executeScript(`JSON.stringify(Array.from(document.querySelectorAll('input,textarea')).map(function(e,i){return {i,tag:e.tagName,ph:e.getAttribute('placeholder')||'',val:String(e.value||'').slice(0,40)};}))`, []);
  console.log('INPUTS:', in2);

  // Try typing into first contenteditable
  const typeRes = await driver.executeScript(`(function(){var el=document.querySelector('[contenteditable="true"]');if(!el)return 'no-ed';el.focus();var s=window.getSelection();var r=document.createRange();r.selectNodeContents(el);r.collapse(false);s.removeAllRanges();s.addRange(r);document.execCommand('insertText',false,'Acceptance line');return 'ok:'+el.innerText.slice(-30);})()`, []);
  console.log('TYPE:', typeRes);
  await sleep(3000);
  const body1 = await driver.executeScript(`return (document.body?.innerText||'').slice(0,400);`, []);
  console.log('BODY1:', JSON.stringify(body1.slice(-300)));

  const png = await driver.takeScreenshot();
  require('fs').writeFileSync('/tmp/editor-type.png', Buffer.from(png,'base64'));
  console.log('shot saved');
  try { await driver.deleteSession(); } catch(e){}
}
main().catch(e => { console.error('FATAL', e.message); process.exit(1); });
