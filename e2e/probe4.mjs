// v4 probe - test simple vs complex executeScript after full render
import WebDriver from 'webdriver';

const APP = '/home/shrtcrct/git/stratum/target/debug/stratum-tauri';
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function main() {
  const driver = await WebDriver.newSession({
    hostname: '127.0.0.1', port: 4444, path: '/', protocol: 'http',
    connectionRetryTimeout: 30000, connectionRetryCount: 1,
    capabilities: {
      alwaysMatch: {
        browserName: 'wry',
        'wdio:enforceWebDriverClassic': true,
        'webkitgtk:browserOptions': { binary: APP, args: ['--automation'] },
      },
    },
  });
  await sleep(15000);
  console.log('--- after 15s ---');
  try {
    const t = await driver.executeScript('return document.title;', []);
    console.log('TITLE:', t);
  } catch (e) { console.log('title ERR', e.message.slice(0,150)); }

  try {
    const u = await driver.executeScript('return location.href;', []);
    console.log('URL:', u);
  } catch (e) { console.log('url ERR', e.message.slice(0,150)); }

  try {
    const n = await driver.executeScript('return (document.getElementById("root")?.children?.length)||0;', []);
    console.log('rootChildren:', n);
  } catch (e) { console.log('root ERR', e.message.slice(0,150)); }

  try {
    const bi = await driver.executeScript('return document.body ? document.body.innerText.slice(0,300) : "nobody";', []);
    console.log('BODYTEXT:', JSON.stringify(bi));
  } catch (e) { console.log('body ERR', e.message.slice(0,200)); }

  try {
    const int = await driver.executeScript(`return Array.from(document.querySelectorAll('button, a, [role="menuitem"], [role="tab"]')).map(e => e.tagName + ':' + (e.innerText||'').slice(0,30));`, []);
    console.log('INTERACTIVE:', JSON.stringify(int));
  } catch (e) { console.log('int ERR', e.message.slice(0,200)); }

  try { await driver.deleteSession(); } catch(e) {}
}
main().catch(e => { console.error('FATAL', e.message); process.exit(1); });
