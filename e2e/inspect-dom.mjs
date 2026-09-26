// inspect the DOM structure to find the main content container
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
  await sleep(12000);
  // List all top-level children of #root with their tag/class and text length
  const dom = await driver.executeScript(`Array.from(document.getElementById('root').children).map((el,i)=>({i,tag:el.tagName,cls:(el.className||'').toString().slice(0,60),txt:(el.innerText||'').slice(0,60),kids:el.children.length}))`, []);
  console.log('ROOT CHILDREN:', JSON.stringify(dom, null, 2));
  // List main-ish elements
  const mains = await driver.executeScript(`Array.from(document.querySelectorAll('main, [class*="content"], [class*="main"], [class*="Editor"], [data-testid]')).slice(0,30).map(el=>({tag:el.tagName,cls:(el.className||'').toString().slice(0,80),txt:(el.innerText||'').slice(0,80),ds:el.getAttribute('data-testid')}))`, []);
  console.log('MAIN-LIKE:', JSON.stringify(mains, null, 2));
  try { await driver.deleteSession(); } catch(e){}
}
main().catch(e => { console.error('FATAL', e.message); process.exit(1); });
