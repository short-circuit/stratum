// inspect DOM - with robust render wait
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
  for (let i = 0; i < 30; i++) {
    try {
      const t = await driver.executeScript('return (document.getElementById("root") && document.getElementById("root").children.length) || 0;', []);
      if (t > 0) { ready = true; console.log('root ready at', (i+1)*2, 's children=', t); break; }
    } catch(e) { /* not up */ }
    await sleep(2000);
  }
  if (!ready) { console.log('ROOT NEVER READY'); try{await driver.deleteSession();}catch(e){} process.exit(1); }

  const dom = await driver.executeScript(`Array.from(document.getElementById('root').children).map((el,i)=>({i,tag:el.tagName,cls:(el.className||'').toString().slice(0,60),txt:(el.innerText||'').slice(0,60),kids:el.children.length}))`, []);
  console.log('ROOT CHILDREN:', JSON.stringify(dom, null, 2));
  const mains = await driver.executeScript(`Array.from(document.querySelectorAll('main, [class*="content"], [class*="main"], [class*="Editor"], [data-testid]')).slice(0,40).map(el=>({tag:el.tagName,cls:(el.className||'').toString().slice(0,80),txt:(el.innerText||'').slice(0,100),ds:el.getAttribute('data-testid')}))`, []);
  console.log('MAIN-LIKE:', JSON.stringify(mains, null, 2));
  const png = await driver.takeScreenshot();
  require('fs').writeFileSync('/tmp/inspect-shot.png', Buffer.from(png,'base64'));
  console.log('shot saved');
  try { await driver.deleteSession(); } catch(e){}
}
main().catch(e => { console.error('FATAL', e.message); process.exit(1); });
