#!/usr/bin/env node
/**
 * Layer 3 — Real E2E Test Runner
 * 
 * Connects to tauri-driver via the WebDriver protocol to test the
 * compiled Tauri app with real IPC, real SQLite, real filesystem.
 * 
 * Requires:
 *   - tauri-driver (cargo install tauri-driver)
 *   - WebKitWebDriver (from webkit2gtk-4.1 package)
 *   - xvfb-run (for headless display)
 * 
 * Usage:
 *   bash e2e/real/run.sh
 */

import { remote } from 'webdriver';
import { execSync } from 'child_process';
import path from 'path';
import { fileURLToPath } from 'url';
import fs from 'fs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const STRATUM_DIR = path.resolve(__dirname, '../..');
const DRIVER_PORT = 4444;

let passed = 0;
let failed = 0;

function assert(condition, message) {
  if (condition) { passed++; console.log(`  ✅ ${message}`); }
  else { failed++; console.log(`  ❌ ${message}`); }
}

function sleep(ms) { return new Promise(r => setTimeout(r, ms)); }

async function main() {
  console.log('=== Stratum PKM — Real E2E Tests ===\n');

  // Verify tauri-driver is running
  try {
    const resp = await fetch(`http://localhost:${DRIVER_PORT}/status`);
    if (!resp.ok) throw new Error(`Status ${resp.status}`);
  } catch {
    console.error(`❌ tauri-driver not responding on port ${DRIVER_PORT}`);
    console.log('\nMake sure tauri-driver is running:');
    console.log('  tauri-driver --port 4444');
    console.log('\nIf WebKitWebDriver is missing, install it:');
    console.log('  Arch:  sudo pacman -S webkit2gtk-4.1');
    console.log('  Ubuntu: sudo apt install webkit2gtk-4.1-dev');
    process.exit(1);
  }

  // Connect to tauri-driver
  let driver;
  try {
    driver = await remote({
      protocol: 'http',
      hostname: 'localhost',
      port: DRIVER_PORT,
      path: '/',
      capabilities: {
        browserName: 'tauri',
        'tauri:options': {
          application: path.join(STRATUM_DIR, 'target', 'debug', 'stratum-tauri'),
        },
      },
    });
  } catch (e) {
    console.error(`❌ Failed to create WebDriver session: ${e.message}`);
    process.exit(1);
  }

  try {
    // ── Test 1: App boots ────────────────────────────
    console.log('── Test 1: App boots and renders ──');
    await driver.url('tauri://localhost');
    await sleep(3000);

    const title = await driver.getTitle();
    assert(typeof title === 'string', `Page has title: "${title}"`);

    const source = await driver.getPageSource();
    assert(source.length > 0, 'Page has HTML content');

    // ── Test 2: Root element exists ──────────────────
    console.log('\n── Test 2: Root element is visible ──');
    const root = await driver.findElement('css selector', '#root');
    const displayed = await root.isDisplayed();
    assert(displayed, '#root element is displayed');

    const rootText = await root.getText();
    assert(rootText.length > 0, '#root has text content');
    console.log(`  Content: "${rootText.substring(0, 200)}"`);

    console.log(`\n📊 ${passed} passed, ${failed} failed out of ${passed + failed}\n`);
  } finally {
    if (driver) try { await driver.deleteSession(); } catch {}
  }

  process.exit(failed > 0 ? 1 : 0);
}

main().catch(e => {
  console.error('Fatal:', e.message);
  process.exit(1);
});
