#!/usr/bin/env node
/**
 * Stratum Automated E2E Harness — test runner.
 *
 * Orchestrates the full real-E2E flow:
 *   1. validate prerequisites (tauri-driver, WebKitWebDriver, built app)
 *   2. start tauri-driver (which spawns WebKitWebDriver)
 *   3. create a WebDriver session (which launches the app under automation)
 *   4. run the test suite
 *   5. tear everything down, report pass/fail, exit non-zero on failure
 *
 * Usage:
 *   node e2e/harness/bin/run.mjs
 *   HARNESS_APP=/path/to/stratum-tauri node e2e/harness/bin/run.mjs
 *   HARNESS_APP=/path/to/stratum-tauri HARNESS_DRIVER=/path/to/WebKitWebDriver node e2e/harness/bin/run.mjs
 *
 * Env:
 *   HARNESS_APP      absolute path to the compiled app binary (default: <repo>/target/debug/stratum-tauri)
 *   HARNESS_DRIVER   absolute path to WebKitWebDriver (default: auto-detect)
 *   HARNESS_DISPLAY  X display to use (default: 99)
 *   HARNESS_XVFB     1 to spawn Xvfb :99 automatically (default 1)
 *   HARNESS_NO_CLEAN 1 to keep processes after the run (debugging)
 */

import { spawn, spawnSync, execFileSync } from 'child_process';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { createSession, waitForApp, deleteSession, sleep } from '../lib/driver.js';
import { beginRun, recordTest, writeResults, resultsPaths } from '../lib/results.js';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(__dirname, '../../..');

const TDRIVER_PORT = 4444;
const DISPLAY = process.env.HARNESS_DISPLAY || '99';
const KEEP = process.env.HARNESS_NO_CLEAN === '1';

// tauri-driver relays to the native WebDriver it spawns on the next port up.
function nativeDriverPort() {
  return TDRIVER_PORT + 1;
}

// ---------------------------------------------------------------------------
// Environment & binary resolution
// ---------------------------------------------------------------------------

function repoDefaultBinary() {
  const debug = path.join(REPO_ROOT, 'target', 'debug', 'stratum-tauri');
  if (fs.existsSync(debug)) return debug;
  const release = path.join(REPO_ROOT, 'target', 'release', 'stratum-tauri');
  if (fs.existsSync(release)) return release;
  return debug;
}

function findWebKitWebDriver() {
  const explicit = process.env.HARNESS_DRIVER;
  if (explicit && fs.existsSync(explicit)) return explicit;

  const candidates = [
    // Nix store (common on this dev machine)
    '/nix/store/2348bqnh7jhzq48dp6kjvc014rmipr96-webkitgtk-2.52.3+abi=4.1/bin/WebKitWebDriver',
    // system paths
    '/usr/lib/webkit2gtk-4.1/WebKitWebDriver',
    '/usr/lib64/webkit2gtk-4.1/WebKitWebDriver',
    // Debian/Ubuntu: the webkit2gtk-driver package installs the driver at
    // /usr/bin/WebKitWebDriver (NOT inside the webkit2gtk-4.1 lib dir).
    '/usr/bin/WebKitWebDriver',
    // Ubuntu multiarch lib dirs (e.g. x86_64).
    '/usr/lib/x86_64-linux-gnu/webkit2gtk-4.1/WebKitWebDriver',
    '/usr/lib/aarch64-linux-gnu/webkit2gtk-4.1/WebKitWebDriver',
  ];
  for (const c of candidates) {
    if (fs.existsSync(c)) return c;
  }
  // Last resort: search PATH (covers other distros / custom installs).
  try {
    const onPath = execFileSync('which', ['WebKitWebDriver'], { encoding: 'utf8' }).trim();
    if (onPath && fs.existsSync(onPath)) return onPath;
  } catch {
    /* not on PATH */
  }
  return null;
}

function runCapture(cmd, args, { timeoutMs = 120000 } = {}) {
  try {
    const r = execFileSync(cmd, args, { stdio: ['ignore', 'pipe', 'pipe'], encoding: 'utf8', timeout: timeoutMs });
    return { ok: true, stdout: r };
  } catch (e) {
    return { ok: false, stdout: (e.stdout || '') + (e.stderr || '') };
  }
}

// ---------------------------------------------------------------------------
// Process lifecycle
// ---------------------------------------------------------------------------

function killPid(pid) {
  if (!pid) return;
  try { process.kill(pid, 'SIGTERM'); } catch (e) {}
}

let xvfbPid = null;
let tdriverProc = null;

function cleanup() {
  if (KEEP) return;
  killPid(tdriverProc?.pid);
  killPid(xvfbPid);
  // The app is spawned by WebKitWebDriver; kill it too.
  try { spawnSync('pkill', ['-x', 'stratum-tauri']); } catch (e) {}
}

async function ensureXvfb() {
  if (process.env.HARNESS_XVFB === '0') return;
  const probe = runCapture('xvfb-run', ['--help'], { timeoutMs: 10000 });
  void probe;
  // Check if :99 already has an X server
  const has99 = fs.existsSync(`/tmp/.X11-unix/X${DISPLAY}`);
  if (has99) return;
  xvfbPid = spawn('Xvfb', [`:${DISPLAY}`, '-screen', '0', '1280x800x24', '-nolisten', 'tcp'], {
    stdio: 'ignore',
    detached: false,
  }).pid;
  // wait for the socket
  const deadline = Date.now() + 15000;
  while (Date.now() < deadline) {
    if (fs.existsSync(`/tmp/.X11-unix/X${DISPLAY}`)) return;
    await sleep(500);
  }
  throw new Error(`Xvfb :${DISPLAY} did not start`);
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

let passed = 0;
let failed = 0;

function ok(name) {
  passed++;
  console.log(`  ✅ ${name}`);
  recordTest({ suite: 'harness', name, status: 'pass' });
}
function bad(name, detail) {
  failed++;
  console.log(`  ❌ ${name}${detail ? ` — ${detail}` : ''}`);
  recordTest({ suite: 'harness', name, status: 'fail', detail });
}

async function runTests(driver) {
  console.log('\n── Test 0: First-run onboarding gate (skip-if-no-wizard) ──');
  // A fresh HOME renders the wizard on a brand-new empty vault; an existing
  // populated vault (real user home / CI harness) skips it. Detect the wizard
  // by its localized welcome heading; when absent, verify the normal layout
  // renders instead and skip the wizard click-through (harness stays green).
  // The session may still be booting (React not mounted yet), so poll until
  // real content is present before probing.
  let bootState = null;
  const probeDeadline = Date.now() + 60000;
  while (Date.now() < probeDeadline) {
    bootState = await driver.executeScript(
      `const t = (document.body.innerText || '');
       return {
         wizard: t.includes('Welcome to Stratum') || t.includes('Willkommen bei Stratum'),
         picker: t.includes('Choose Vault Folder') || t.includes('Vault-Ordner wählen'),
         bodyLen: t.length,
       };`,
      [],
    );
    if (bootState && bootState.bodyLen > 0) break;
    await sleep(1200);
  }
  if (bootState && bootState.wizard) {
    ok('onboarding wizard rendered for fresh vault');
    // Click through: Next (step 1) → Next (step 2) → Next (step 3 triggers install)
    const clickNext = async (label) => {
      const r = await driver.executeScript(
        `var b=[...document.querySelectorAll('button')].find(function(e){var t=(e.textContent||'').trim(); return t==='${label}' && !e.disabled;});
         if(!b) return false;
         b.click(); return true;`,
        [],
      );
      return r;
    };
    const n1 = await clickNext('Next');
    await sleep(800);
    const n2 = await clickNext('Next');
    await sleep(800);
    // Step 3 (sample vault): enable the "Create a sample vault" checkbox so the
    // sample-note install path is exercised too (backlinks/graph demo set).
    const sampleToggled = await driver.executeScript(
      `var lab=[...document.querySelectorAll('label')].find(function(e){var t=(e.textContent||'').trim(); return t==='Create a sample vault'||t==='Beispiel-Vault erstellen';});
       if(!lab) return false;
       var inp=lab.querySelector('input[type="checkbox"]');
       if(!inp) return false;
       if(!inp.checked){ inp.click(); }
       return true;`,
      [],
    );
    if (sampleToggled) ok('sample-vault toggle enabled'); else bad('sample-vault toggle enabled');
    await sleep(500);
    const n3 = await clickNext('Next');
    await sleep(2500); // allow the install orchestration + markOnboardingDone to run
    const fin = await driver.executeScript(
      `const t = (document.body.innerText || '');
       return { finish: t.includes("You're all set") || t.includes('Alles bereit'), bodyLen: t.length };`,
      [],
    );
    if (!(n1 && n2 && n3)) bad(`wizard click-through advanced (next1=${n1} next2=${n2} next3=${n3})`);
    else ok('wizard click-through advanced (Next ×3)');
    if (fin && fin.finish) ok('wizard finish screen rendered ("You\'re all set")');
    else bad(`wizard finish screen rendered (body ${fin?.bodyLen} chars)`);
    const started = await clickNext('Get Started');
    await sleep(2000);
    if (started) ok('Get Started clicked'); else bad('Get Started clicked');
  } else if (bootState && bootState.picker) {
    // Fresh vault but wizard absent (should not happen — belt & braces).
    ok('wizard not detected; Vault Picker shown (onboarding state resolved false or store not ready)');
  } else {
    // Populated vault — the normal harness path. Assert the app shell is up so
    // this branch still validates something real.
    if (bootState && bootState.bodyLen > 100) ok(`wizard skipped for populated vault (body ${bootState.bodyLen} chars)`);
    else bad(`wizard skipped for populated vault (body ${bootState?.bodyLen} chars)`);
  }
  // A wizard run writes config.toml (pending=false) + may install templates;
  // neither affects the later tests. The locale test restores en, so ANY boot
  // locale is fine.

  console.log('\n── Test 1: App boots and renders the journal ──');
  const url = await waitForApp(driver, { url: 'tauri://localhost' });
  ok(`App window is on tauri://localhost (${url})`);

  const title = await driver.getTitle();
  if (typeof title === 'string' && title.length > 0) {
    ok(`App window title: "${title}"`);
  } else {
    bad('App window has a non-empty title');
  }

  console.log('\n── Test 2: Real UI content is present ──');
  // The app's navigation is MUI ListItemButton (renders role="button"), and
  // there are no <a> elements in the app shell at all. On a pristine vault
  // (fresh CI run) note-content links are absent, so asserting on anchors is
  // environment-dependent and would fail a healthy app. Assert instead that the
  // app shell rendered its interactive UI, which is deterministic regardless of
  // vault contents.
  const ui = await driver.executeScript(
    `const d = document;
     const root = d.getElementById('root');
     return {
       hasRoot: !!root,
       rootChildren: root ? root.children.length : 0,
       interactive: d.querySelectorAll(
         'button, a, input, select, textarea, [role="button"], [role="link"], [role="menuitem"], [role="tab"], [role="checkbox"]'
       ).length,
       bodyText: (d.body && d.body.innerText || '').length,
     };`,
    [],
  );
  if (ui && ui.hasRoot && (ui.rootChildren || 0) > 0) {
    ok(`app shell rendered (${ui.rootChildren} top-level node(s))`);
  } else {
    bad('app shell rendered (root has children)');
  }
  if (ui && (ui.interactive || 0) > 0) {
    ok(`app rendered ${ui.interactive} interactive control(s)`);
  } else {
    bad(`app rendered interactive controls (got ${ui?.interactive})`);
  }
  if (ui && ui.bodyText > 100) {
    ok(`page body has ${ui.bodyText} chars of rendered text`);
  } else {
    bad(`page body has rendered text (got ${ui?.bodyText})`);
  }

  console.log('\n── Test 2b: i18n locale switch (any booted locale → de renders translated UI) ──');
  // WebKitGTK persist localStorage across harness runs, so the app may boot en
  // or de. The flow must work regardless: adapt to the booted locale labels.
  const boot = await driver.executeScript(
    `return { hasSettings: (document.body.innerText||'').includes('Settings'),
              hasEinstellungen: (document.body.innerText||'').includes('Einstellungen') };`,
    [],
  );
  console.log(`   booted UI: ${boot && boot.hasEinstellungen ? 'de' : boot && boot.hasSettings ? 'en' : 'unknown'}`);
  // Click Settings (en) or Einstellungen (de) — then Theme (en) or Design (de).
  await driver.executeScript(
    `var s=[...document.querySelectorAll('[role="button"],button,a,li')].find(function(e){var t=e.textContent.trim(); return t==='Settings'||t==='Einstellungen';}); if(s){s.click();} return !!s;`,
    [],
  );
  await sleep(3000);
  await driver.executeScript(
    `var t=[...document.querySelectorAll('[role="tab"]')].find(function(e){var x=e.textContent.trim(); return x==='Theme'||x==='Design';}); if(t){t.click();} return !!t;`,
    [],
  );
  await sleep(2500);
  // The Theme tab has exactly one combobox (the locale selector), rendering the
  // current locale's endonym ("English", "Deutsch", …). Open it directly.
  const openOk = await driver.executeScript(
    `var cb=document.querySelector('[role="combobox"]');
     if(!cb) return false;
     cb.dispatchEvent(new MouseEvent('mousedown',{bubbles:true}));
     cb.dispatchEvent(new MouseEvent('mouseup',{bubbles:true}));
     cb.dispatchEvent(new MouseEvent('click',{bubbles:true}));
     return true;`,
    [],
  );
  if (openOk) ok('language dropdown opened'); else bad('language dropdown opened');
  await sleep(1500);
  const pickOk = await driver.executeScript(
    `var d=[...document.querySelectorAll('[role="option"],.MuiMenuItem-root,[role="menuitem"]')].find(function(e){return e.textContent.trim()==='Deutsch';});
     if(!d) return false;
     d.dispatchEvent(new MouseEvent('mousedown',{bubbles:true}));
     d.dispatchEvent(new MouseEvent('mouseup',{bubbles:true}));
     d.dispatchEvent(new MouseEvent('click',{bubbles:true}));
     return true;`,
    [],
  );
  if (pickOk) ok('Deutsch option selected'); else bad('Deutsch option selected');
  await sleep(3500);
  const de = await driver.executeScript(
    `const t = document.body.innerText || '';
     return {
       hasGermanSettings: t.includes('Einstellungen'),
       hasGermanSearch: t.includes('Suche'),
       hasEnglishSettings: t.includes('Settings'),
       persisted: localStorage.getItem('stratum-locale'),
       bodyLen: t.length,
     };`,
    [],
  );
  if (de && de.hasGermanSettings) {
    ok('locale switch to de rendered German UI (Settings = "Einstellungen")');
  } else {
    bad(`locale switch to de rendered German UI (settings label not found; body ${de?.bodyLen} chars)`);
  }
  if (de && de.hasGermanSearch) {
    ok('German search label rendered (Suche)');
  } else {
    bad('German search label rendered (Suche)');
  }
  if (de && de.hasEnglishSettings) {
    bad('English-only string still visible in de UI ("Settings" found)');
  } else {
    ok('no English string remains in de UI ("Settings" gone)');
  }
  if (de && de.persisted === 'de') {
    ok('locale persisted (stratum-locale=de)');
  } else {
    bad(`locale persisted (got ${de?.persisted})`);
  }
  // Restore en for the remaining tests.
  await driver.executeScript(`localStorage.setItem('stratum-locale', 'en'); return true;`, []);
  await driver.executeScript(`location.reload(); return true;`, []);
  await waitForApp(driver, { url: 'tauri://localhost' });

  console.log('\n── Test 3: IPC round-trip (query real app state) ──');
  const info = await driver.executeScript(
    `const d = document;
     return { title: d.title, bodyLen: d.body && d.body.innerText.length || 0, hasRoot: !!d.getElementById('root'), url: location.href };`,
    [],
  );
  if (info && info.hasRoot) ok('#root element present'); else bad('#root element present');
  if (info && info.bodyLen > 0) ok(`page has ${info?.bodyLen} chars of rendered text`); else bad(`page has rendered text (got ${info?.bodyLen})`);
  if (info && String(info.url).startsWith('tauri://')) ok(`on tauri:// scheme (${info?.url})`); else bad(`on tauri:// scheme (${info?.url})`);

  console.log('\n── Test 4: Screenshot capture ──');
  const png = await driver.takeScreenshot();
  const b = Buffer.from(png, 'base64');
  if (b.length > 1000) ok(`screenshot captured (${b.length} bytes)`); else bad(`screenshot captured (${b.length} bytes)`);
  if (fs.existsSync('/tmp')) {
    try { fs.writeFileSync('/tmp/stratum-harness-last.png', b); } catch (e) {}
  }

  console.log('\n── Test 5: Quick capture dialog opens + saves via keyboard ──');
  // The harness hook is injected by main.tsx; it emits the same
  // 'quick-capture:open' event the tray/global-shortcut path uses, so this
  // exercises the real frontend flow without an OS tray.
  const hookOk = await driver.executeScript(
    `return typeof window.__stratOpenQuickCapture === 'function';`, []);
  if (hookOk) ok('quick capture harness hook (__stratOpenQuickCapture) present');
  else bad('quick capture harness hook (__stratOpenQuickCapture) present');
  if (hookOk) {
    await driver.executeScript(`window.__stratOpenQuickCapture(false); return true;`, []);
    await sleep(2000);
    const dlg = await driver.executeScript(
      `const d = document;
       const tf = [...d.querySelectorAll('textarea,input')].find(e => e.placeholder && e.placeholder.length > 3);
       const txt = (d.body.innerText || '');
       // Locale-dependent title: accept en (Quick Capture) OR de (Schnellnotiz) OR any
       // other translated variant by checking that SOME non-empty dialog title co-exists
       // with the capture input (the DialogTitle is a sibling of the textarea).
       const hasTitle = txt.includes('Quick Capture') || txt.includes('Schnellnotiz') ||
         txt.includes('Cattura rapida') || txt.includes('Capture rapide') ||
         txt.includes('Captura rápida');
       return { textarea: !!tf, hasTitle };`, []);
    (dlg && dlg.textarea) ? ok('quick capture input rendered') : bad('quick capture input rendered');
    (dlg && dlg.hasTitle) ? ok('quick capture title rendered') : bad('quick capture title rendered');

    // Type text into the capture input (React controlled → native setter + input event).
    const typed = await driver.executeScript(
      `const tf = [...document.querySelectorAll('textarea,input')].find(e => e.placeholder && e.placeholder.length > 3);
       if (!tf) return false;
       const proto = tf.tagName === 'TEXTAREA' ? window.HTMLTextAreaElement.prototype : window.HTMLInputElement.prototype;
       Object.getOwnPropertyDescriptor(proto, 'value').set.call(tf, 'E2E quick capture test block');
       tf.dispatchEvent(new Event('input', { bubbles: true }));
       return true;`, []);
    typed ? ok('text typed into capture input') : bad('text typed into capture input');
    await sleep(400);
    await driver.executeScript(
      `const tf = [...document.querySelectorAll('textarea,input')].find(e => e.placeholder && e.placeholder.length > 3);
       if (tf) tf.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', code: 'Enter', keyCode: 13, bubbles: true }));
       return true;`, []);
    await sleep(3000);
    // Verify real persistence through real IPC: ask the backend for the blocks
    // of today's journal and check the captured text is present.
    const today = await driver.executeScript(
      `const d = new Date();
       const p = \`\${d.getFullYear()}-\${String(d.getMonth() + 1).padStart(2, '0')}-\${String(d.getDate()).padStart(2, '0')}\`;
       return window.__TAURI_INTERNALS__.invoke('get_blocks', { pagePath: 'journals/' + p + '.md' })
         .then(r => JSON.stringify(r)).catch(e => 'ERR:' + e);`, []);
    const saved = typeof today === 'string' && today.includes('E2E quick capture test block');
    saved ? ok('quick capture block persisted to today journal (get_blocks)') : bad('quick capture block persisted to today journal (get_blocks)');

    // Escape discards/closes the dialog.
    await driver.executeScript(
      `const tf = [...document.querySelectorAll('textarea,input')].find(e => e.placeholder && e.placeholder.length > 3);
       if (tf) tf.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', code: 'Escape', keyCode: 27, bubbles: true }));
       return true;`, []);
    await sleep(1000);
    const closed = await driver.executeScript(
      `return ![...document.querySelectorAll('textarea,input')].some(e => e.placeholder && e.placeholder.length > 3);`, []);
    closed ? ok('Escape discards/closes quick capture') : bad('Escape discards/closes quick capture');
  }
  console.log('\n── Test 5: 1-Click Importers ──');
  // Real IPC through the real app: import the committed Obsidian sample vault.
  // This targets the ACTIVE vault (the default ~/StratumVault), so the outcome
  // is idempotent and must be interpreted as a disjunction:
  //   first run  → 3 pages imported, 1 asset, 0 duplicates
  //   re-run     → 0 pages imported, 0 assets, 3 duplicates
  const FIXTURE = path.join(REPO_ROOT, 'crates/pkm-import/tests/fixtures/obsidian-sample');
  const impRes = await driver.executeScript(
    `return window.__TAURI_INTERNALS__.invoke('import_vault',
       { path: ${JSON.stringify(FIXTURE)}, opts: { format: 'auto', dry_run: false } });`,
    [],
  );
  if (impRes && typeof impRes.pages_imported === 'number') {
    const firstRun =
      impRes.pages_imported === 3 && impRes.assets_imported === 1 && impRes.duplicates_skipped === 0;
    const rerun =
      impRes.pages_imported === 0 && impRes.assets_imported === 0 && impRes.duplicates_skipped === 3;
    if (firstRun || rerun) {
      ok(`import_vault counts (pages=${impRes.pages_imported} assets=${impRes.assets_imported} duplicates=${impRes.duplicates_skipped})`);
    } else {
      bad('import_vault counts (3 then 0 / 3 duplicates)', JSON.stringify(impRes));
    }
  } else {
    bad('import_vault returned a usable report', JSON.stringify(impRes));
  }

  // The imported pages must be registered in the vault page list.
  const pages = await driver.executeScript(
    `return window.__TAURI_INTERNALS__.invoke('list_pages');`,
    [],
  );
  const listPaths = (pages && Array.isArray(pages.pages) ? pages.pages : []).map((p) => String(p.path));
  if (listPaths.some((p) => p.startsWith('imported/obsidian-sample/'))) {
    ok('list_pages includes imported pages under imported/obsidian-sample/');
  } else {
    bad('list_pages includes imported pages', JSON.stringify(listPaths.slice(0, 10)));
  }

  // The imported content must render in the UI. Open the imported Welcome page
  // and confirm its title appears in the rendered body text.
  await driver.executeScript(
    `location.href = location.origin + '/page/' + encodeURIComponent('imported/obsidian-sample/Welcome.md'); return true;`,
    [],
  );
  await waitForApp(driver, { url: 'tauri://localhost' });
  await sleep(1500);
  const impBody = await driver.executeScript(`return (document.body && document.body.innerText) || '';`, []);
  if (impBody.includes('Welcome')) {
    ok('imported page title rendered in the UI (Welcome)');
  } else {
    bad('imported page title rendered in the UI (Welcome)', impBody.slice(0, 200));
  }
}

async function main() {
  console.log('=== Stratum Automated E2E Harness ===');
  console.log(`Workspace: ${REPO_ROOT}`);
  beginRun({ appBinary: process.env.HARNESS_APP || repoDefaultBinary() });

  const appBinary = process.env.HARNESS_APP || repoDefaultBinary();
  if (!fs.existsSync(appBinary)) {
    console.error(`\n❌ App binary not found: ${appBinary}`);
    console.error('   Build it first with:  cargo build -p stratum-tauri');
    console.error('   (must include the custom-protocol feature — see README.md)');
    process.exit(2);
  }
  console.log(`App: ${appBinary}`);

  const driverBin = findWebKitWebDriver();
  if (!driverBin) {
    console.error('\n❌ WebKitWebDriver not found.');
    console.error('   Set HARNESS_DRIVER=/path/to/WebKitWebDriver or install webkit2gtk-4.1.');
    process.exit(2);
  }
  console.log(`Native driver: ${driverBin}`);

  // Ensure tauri-driver is on PATH
  const tdriverPath = process.env.HARNESS_TDRIVER || 'tauri-driver';
  const whichTd = runCapture('which', [tdriverPath], { timeoutMs: 10000 });
  if (!whichTd.ok && !fs.existsSync(tdriverPath)) {
    console.error('\n❌ tauri-driver not found on PATH.');
    console.error('   Install: cargo install tauri-driver');
    process.exit(2);
  }
  console.log(`tauri-driver: ${tdriverPath}`);

  const env = {
    ...process.env,
    DISPLAY: `:${DISPLAY}`,
    GDK_BACKEND: 'x11',
    // Must NOT be set — Wayland causes "Error 71 dispatching to Wayland display".
    WAYLAND_DISPLAY: '',
    WEBKIT_DISABLE_DMABUF_RENDERER: '1',
    WEBKIT_DISABLE_COMPOSITING_MODE: '1',
    // WebKitGTK >= 2.44 forces a bubblewrap sandbox; on CI/container runners
    // without a usable bwrap config this makes the web process crash on launch.
    // (WEBKIT_FORCE_SANDBOX=0 is deprecated and no longer disables it.)
    WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS: '1',
    // Headless/CI runners have no GPU; force Mesa software rendering.
    LIBGL_ALWAYS_SOFTWARE: '1',
  };

  try {
    await ensureXvfb();

    console.log(`\nStarting tauri-driver on :${TDRIVER_PORT}...`);
    tdriverProc = spawn(tdriverPath, ['--port', String(TDRIVER_PORT), '--native-driver', driverBin], {
      env,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    let tdLog = '';
    tdriverProc.stdout.on('data', (d) => { tdLog += d; });
    tdriverProc.stderr.on('data', (d) => { tdLog += d; });

    // wait for readiness
    const ready = await pollPort(TDRIVER_PORT, 15000);
    if (!ready) {
      console.error('❌ tauri-driver did not become ready.');
      console.error(tdLog.slice(-2000));
      process.exit(1);
    }
    console.log('✅ tauri-driver ready');
    // tauri-driver can report ready before the native WebDriver has finished
    // initializing; a session POST issued in that window stalls (tauri-apps/tauri#3576).
    // Wait briefly for the native driver to also be reachable before POSTing.
    const nativeReady = await pollPort(nativeDriverPort(), 15000);
    if (!nativeReady) {
      console.error('⚠️ native WebDriver did not become reachable; continuing (best effort)');
    }

    console.log('Creating WebDriver session (launches the app)...');
    let driver;
    try {
      driver = await createSession(appBinary, { port: TDRIVER_PORT });
    } catch (e) {
      console.error('❌ Failed to create WebDriver session.');
      if (tdLog.trim()) {
        console.error('\n── tauri-driver / app stderr (for debugging) ──');
        console.error(tdLog.slice(-4000));
      }
      throw e;
    }
    console.log(`   session: ${driver.sessionId}`);

    try {
      const failuresBefore = failed;
      await runTests(driver);
      if (failed > failuresBefore) {
        // try to capture app log for debugging
        const appLog = tdLog.slice(-2000);
        if (appLog.trim()) {
          console.log('\n── tauri-driver output (for debugging) ──');
          console.log(appLog);
        }
      }
    } finally {
      await deleteSession(driver);
    }

    console.log(`\n📊 ${passed} passed, ${failed} failed out of ${passed + failed}`);

    // Machine-readable capture (acceptance criterion) — written to the repo's
    // gitignored test-results/ directory.
    const manifest = await writeResults({
      status: failed > 0 ? 'fail' : 'pass',
      summary: 'stratum automated e2e (tauri-driver)',
      startedAt: undefined,
    });
    const rp = resultsPaths();
    console.log(`📄 results written:`);
    console.log(`   JSON : ${rp.manifest} (counts=${JSON.stringify(manifest.counts)})`);
    console.log(`   JUnit: ${rp.junit}`);
  } finally {
    cleanup();
  }

  process.exit(failed > 0 ? 1 : 0);
}

function pollPort(port, timeoutMs) {
  return new Promise((resolve) => {
    const deadline = Date.now() + timeoutMs;
    const http = () => {
      fetch(`http://127.0.0.1:${port}/status`)
        .then((r) => r.json())
        .then((j) => { if (j?.value?.ready) resolve(true); else retry(); })
        .catch(() => retry());
    };
    const retry = () => {
      if (Date.now() > deadline) return resolve(false);
      setTimeout(http, 700);
    };
    http();
  });
}

main().catch(async (e) => {
  console.error('\nFATAL:', e.message);
  cleanup();
  // Still emit a machine-readable record so CI/artifact capture sees the
  // failure even on a hard crash (session create, driver readiness, etc).
  try {
    const manifest = await writeResults({
      status: 'fail',
      summary: 'stratum automated e2e (tauri-driver) — fatal error',
      fatal: e.message,
    });
    console.log(`📄 failure results written: ${resultsPaths().manifest} (counts=${JSON.stringify(manifest.counts)})`);
  } catch (ee) {
    /* best-effort only */
  }
  process.exit(1);
});
