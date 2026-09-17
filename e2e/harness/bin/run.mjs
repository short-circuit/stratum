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
import { createSession, waitForApp, deleteSession, elementId, sleep } from '../lib/driver.js';
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
  const anchors = await driver.findElements('css selector', 'a');
  if (anchors.length > 0) {
    ok(`found ${anchors.length} anchor elements`);
  } else {
    bad('Rendered at least one anchor element');
  }

  if (anchors.length) {
    const t = await driver.getElementText(elementId(anchors[0]));
    if (t && t.length > 0) {
      ok(`first anchor has text "${t}"`);
    } else {
      bad('first anchor has non-empty text');
    }
  }

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
