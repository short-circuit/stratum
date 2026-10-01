// Probe executeScript API shape against this webdriver build
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
  console.log('session', driver.sessionId);

  // Poll a few times for the app to come up
  for (let i = 0; i < 40; i++) {
    try {
      // shape A: positional
      const a = await driver.executeScript(
        'return 1+1;',
        []
      );
      console.log('A positional WORKED:', JSON.stringify(a));
      break;
    } catch (e) {
      console.log('A failed:', e.message.slice(0, 120));
      break;
    }
  }

  // Wait for render, then try shapes
  await sleep(8000);
  try {
    const b = await driver.executeScript('return document.title;', []);
    console.log('B positional title:', JSON.stringify(b));
  } catch (e) { console.log('B fail', e.message.slice(0,200)); }

  try {
    const c = await driver.executeScript({ script: 'return document.title;', args: [] });
    console.log('C object title:', JSON.stringify(c));
  } catch (e) { console.log('C fail', e.message.slice(0,200)); }

  try {
    await driver.deleteSession();
  } catch (e) {}
}
main().catch(e => { console.error('FATAL', e.message); process.exit(1); });
