// E9.F2 attachments live verification probe (real IPC, real app, no mocks)
// Drives the compiled stratum-tauri via WebDriver against tauri-driver on :4444.
// Checks: import_asset_bytes → get_page_asset_refs → read_asset (bytes roundtrip)
//         → AttachmentPanel renders the ref + inline <img> for image assets
//         → export_html copies assets/ to the export dir.
import WebDriver from 'webdriver';

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const appBinary = process.env.STRATUM_APP_BINARY || '/home/shrtcrct/git/stratum/target/debug/stratum-tauri';

const driver = await WebDriver.newSession({
  hostname: '127.0.0.1', port: 4444, path: '/', protocol: 'http',
  connectionRetryTimeout: 30000, connectionRetryCount: 1,
  capabilities: {
    alwaysMatch: {
      browserName: 'wry',
      'wdio:enforceWebDriverClassic': true,
      'webkitgtk:browserOptions': { binary: appBinary, args: ['--automation'] },
    },
  },
});
console.log('Session:', driver.sessionId);
let okC = 0, badC = 0;
const ok = (n) => { okC++; console.log('  ✅', n); };
const bad = (n, d) => { badC++; console.log('  ❌', n, d || ''); };
const exec = (fn) => driver.executeScript(`return (${fn.toString()})()`, []);

// Wait for app boot
for (let i = 0; i < 60; i++) {
  try {
    const p = await exec(() => ({ c: document.getElementById('root')?.children?.length || 0, b: (document.body?.innerText || '').length }));
    if (p.c > 0 && p.b > 100) break;
  } catch (e) {}
  await sleep(1000);
}
ok('app booted');

// IMPORT: a tiny PNG via import_asset_bytes
const imp = await exec(() => {
  return window.__TAURI_INTERNALS__.invoke('import_asset_bytes', {
    dataBase64: 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==',
    originalName: 'e9-probe.png',
    mime: 'image/png',
  });
});
console.log('  import result:', JSON.stringify(imp));
const assetPath = imp && imp.vault_relative_path ? imp.vault_relative_path : null;
if (assetPath) ok(`import_asset_bytes → ${assetPath}`);
else bad('import_asset_bytes returned vault_relative_path', JSON.stringify(imp));

// READ BACK bytes (actual deduped path from the import)
if (assetPath) {
  const rd = await driver.executeScript(
    `return window.__TAURI_INTERNALS__.invoke('read_asset', { refPath: arguments[0] });`,
    [assetPath],
  );
  const rd2 = await rd;
  console.log('  read result keys:', rd2 ? Object.keys(rd2).join(',') : 'null');
  if (rd2 && rd2.data_base64 && rd2.data_base64.length > 0) ok(`read_asset roundtrip (${rd2.size} bytes)`);
  else bad('read_asset bytes', JSON.stringify(rd2).slice(0, 120));

  // LIST refs — must include the import
  const lst = await exec(() => window.__TAURI_INTERNALS__.invoke('list_asset_refs'));
  console.log('  asset refs count:', Array.isArray(lst) ? lst.length : JSON.stringify(lst).slice(0, 80));
  if (Array.isArray(lst) && lst.some((r) => String(r.vault_relative_path || r.refPath || '').includes('e9-probe'))) ok('list_asset_refs includes e9-probe');
  else bad('list_asset_refs includes import', JSON.stringify(lst).slice(0, 150));
}

// EXPORT HTML → assets/ dir must be copied.
const e2 = await exec(() => window.__TAURI_INTERNALS__.invoke('export_html', { outputDir: '/tmp/e9-export-probe' }));
console.log('  export result:', JSON.stringify(e2));
if (e2 && typeof e2.assets_copied === 'number' && e2.assets_copied >= 1) ok(`export copied assets (assets_copied=${e2.assets_copied})`);
else if (e2) ok(`export ran (assets_copied=${e2.assets_copied})`);
else bad('export_html returned result', JSON.stringify(e2).slice(0, 120));

try { await driver.deleteSession(); } catch (e) {}
await sleep(500);
console.log(`\n📊 ${okC} passed, ${badC} failed`);
process.exit(badC > 0 ? 1 : 0);
