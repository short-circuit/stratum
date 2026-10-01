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
  await sleep(4000);
  const tests = {
    'str': `return "hello";`,
    'num': `return 42;`,
    'arr-str': `return ["a","b"];`,
    'arr-obj-js': `return JSON.stringify([{a:1},{b:2}]);`,
    'obj-json': `return JSON.stringify({x:1,y:"z"});`,
    'arr-of-obj-expr': `return [1,2].map(n=>({v:n}));`,
    'typeof-json': `return typeof JSON;`,
    'arr-str-map': `return Array.from(document.querySelectorAll('*')).slice(0,5).map(e=>e.tagName);`,
    'root-kids-arr': `return (function(){var r=document.getElementById('root'); if(!r) return 'noroot'; return Array.from(r.children).map(function(e){return e.tagName;});})();`,
  };
  for (const [k, script] of Object.entries(tests)) {
    try {
      const r = await driver.executeScript(script, []);
      console.log(k, '=>', JSON.stringify(r));
    } catch (e) { console.log(k, 'ERR', e.message.slice(0,120)); }
  }
  try { await driver.deleteSession(); } catch(e){}
}
main().catch(e => { console.error('FATAL', e.message); process.exit(1); });
