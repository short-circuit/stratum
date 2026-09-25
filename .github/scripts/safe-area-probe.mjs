#!/usr/bin/env node
// E5.E1 Android status-bar overlap regression probe (CDP over the Tauri WebView).
//
// Regression test for the safe-area fix (t_175cfb14). It asserts that the
// mobile top bar / page header is NOT rendered under the Android
// notification/status bar. On the broken build the bar is pinned at the
// viewport top (getBoundingClientRect().top == 0) while the status-bar inset
// is non-zero, so chrome is drawn underneath the system bar.
//
// The probe is a PASS/FAIL verdict:
//   - device mode  -> FAILS on the old behaviour, PASSES with the fix
//   - desktop mode -> tolerant: no mobile chrome => vacuously PASS (no false
//                     positive), because safe-area insets are 0 on desktop.
//
// Discovery (not this probe's job): the caller must `adb forward` the WebView
// devtools socket and resolve the ws:// debugger URL from /json. The probe
// receives the ws:// URL as argv[2]. Mode (device|desktop) is argv[3] and
// defaults to device.
//
// Usage: node safe-area-probe.mjs <ws://.../devtools/page/...> [device|desktop]
// Exit codes: 0 = PASS, 1 = FAIL (regression / assertion), 2 = usage, 3 = conn,
//             4 = page evaluation error.

const url = process.argv[2];
const mode = (process.argv[3] || 'device').toLowerCase();
if (!url || !url.startsWith('ws://')) {
  console.error('usage: node safe-area-probe.mjs <ws://...> [device|desktop]');
  process.exit(2);
}

// WebSocket client. Node >= 22 exposes a WHATWG WebSocket global, but the CI
// android-smoke runner (and this repo's developer shell) may carry an older
// node where the global is absent — the GH-hosted image does not pin node via
// setup-node, and only node >= 22 ships the global. We resolve the `ws`
// package from the repo (already a dependency through the frontend) when the
// global is missing, so the probe behaves identically on every supported
// runtime instead of crashing with `WebSocket is not defined`.
let WebSocketImpl = globalThis.WebSocket;
if (typeof WebSocketImpl !== 'function') {
  try {
    // resolve from the repo root / this script's repo tree, not from the CWD
    const { createRequire } = await import('module');
    const requireFromProbe = createRequire(import.meta.url);
    WebSocketImpl = requireFromProbe('ws').WebSocket;
  } catch (e) {
    console.error(
      'FATAL: no global WebSocket available and the `ws` package could not be ' +
        `resolved from the repo: ${e.message}`,
    );
    process.exit(3);
  }
  if (typeof WebSocketImpl !== 'function') {
    console.error('FATAL: resolved `ws` package does not expose a WebSocket class');
    process.exit(3);
  }
}
const ws = new WebSocketImpl(url);
let id = 0;
const pending = new Map();
const failures = [];

function send(method, params = {}) {
  return new Promise((resolve) => {
    const mid = ++id;
    pending.set(mid, resolve);
    ws.send(JSON.stringify({ id: mid, method, params }));
  });
}
ws.onmessage = (ev) => {
  const msg = JSON.parse(ev.data);
  if (msg.id && pending.has(msg.id)) {
    pending.get(msg.id)(msg.result);
    pending.delete(msg.id);
  }
};
ws.onerror = (e) => {
  console.error(`websocket error: ${e.message || e}`);
  process.exit(3);
};

async function evaluate(expression) {
  const r = await send('Runtime.evaluate', { expression, returnByValue: true });
  if (r.exceptionDetails) {
    throw new Error(`page exception: ${JSON.stringify(r.exceptionDetails)}`);
  }
  return r.result.value;
}

// failGood(condIsGood, msg, expected, actual): records a failure when the
// condition is NOT satisfied.
function failGood(cond, msg, expected, actual) {
  if (cond) return;
  failures.push(`${msg} (expected ${expected}, got ${actual})`);
}

const ALLOWED_TITLES_JS = JSON.stringify([
  'Journal', 'Pages', 'Search', 'Stratum', 'Kanban', 'Query', 'Graph',
  'Plugins', 'Settings', 'Flashcards', 'Templates', 'Whiteboards',
  'Ask your notes',
]);

ws.onopen = async () => {
  try {
    await send('Runtime.enable');
    const raw = await evaluate(`(() => {
      const ALLOWED = new Set(${ALLOWED_TITLES_JS});
      const cs = getComputedStyle(document.documentElement);
      const readPx = (name) => {
        const raw = cs.getPropertyValue(name);
        if (!raw) return 0;
        const v = parseFloat(raw);
        return Number.isFinite(v) ? v : 0;
      };
      const safeAreaTop = readPx('--safe-area-top');
      const safeAreaBottom = readPx('--safe-area-bottom');
      // Mobile top bar: the 48px absolutely-positioned app chrome bar.
      const topBar = [...document.querySelectorAll('div')].find(d => {
        const s = getComputedStyle(d);
        if (s.position !== 'absolute') return false;
        if (s.height !== '48px') return false;
        const t = (d.innerText || '').trim();
        return t && t.length < 40 && ALLOWED.has(t);
      });
      const rect = topBar ? topBar.getBoundingClientRect() : null;
      return JSON.stringify({
        safeAreaTop,
        safeAreaBottom,
        topBarFound: !!topBar,
        topBarTop: rect ? rect.top : null,
        topBarHeight: rect ? rect.height : null,
        innerHeight: window.innerHeight,
        innerWidth: window.innerWidth,
      });
    })()`);
    let p;
    try { p = typeof raw === 'string' ? JSON.parse(raw) : raw; }
    catch { p = raw; }

    const evidence = {
      mode,
      safeAreaTop: p.safeAreaTop,
      safeAreaBottom: p.safeAreaBottom,
      topBarFound: p.topBarFound,
      topBarTop: p.topBarTop,
      topBarHeight: p.topBarHeight,
      viewport: { h: p.innerHeight, w: p.innerWidth },
    };

    if (mode === 'device') {
      // On-device the platform MUST have reported a non-zero status-bar inset,
      // otherwise this assertion would be vacuous (nothing to prove).
      failGood(p.safeAreaTop > 0, 'status-bar inset present on device', '>0', String(p.safeAreaTop));
      failGood(p.topBarFound, 'mobile top bar present', 'found', 'not found');
      if (p.topBarFound) {
        failGood(
          Math.abs((p.topBarTop ?? -1) - p.safeAreaTop) < 1.5,
          'top bar not under the status bar (topBarTop == safeAreaTop)',
          p.safeAreaTop.toFixed(2),
          p.topBarTop?.toFixed(2),
        );
        failGood(
          Math.abs((p.topBarHeight ?? 0) - 48) < 1,
          'top bar height intact',
          '48',
          String(p.topBarHeight),
        );
      }
    } else if (mode === 'desktop') {
      // Desktop: MobileLayout chrome is not rendered (desktop uses the
      // sidebar layout), and safe-area insets resolve to 0. Missing top bar
      // is the EXPECTED desktop state, so no assertion fires — no false
      // positive. If a top bar happens to exist, only the no-overlap
      // invariant is checked (both sides 0 => trivially satisfied).
      if (p.topBarFound) {
        failGood(
          Math.abs((p.topBarTop ?? 0) - p.safeAreaTop) < 1.5,
          'top bar not under any inset (topBarTop == safeAreaTop)',
          p.safeAreaTop.toFixed(2),
          p.topBarTop?.toFixed(2),
        );
      }
    } else {
      console.error(`unknown mode: ${mode} (use device|desktop)`);
      process.exit(2);
    }

    console.log('=== SAFE-AREA PROBE RESULT ===');
    console.log(JSON.stringify(evidence, null, 2));
    if (failures.length) {
      console.error(`SAFE_AREA_PROBE FAIL (${mode} mode):`);
      for (const f of failures) console.error('  - ' + f);
      process.exit(1);
    }
    console.log(`SAFE_AREA_PROBE PASS (${mode} mode): no chrome under the status bar.`);
    process.exit(0);
  } catch (e) {
    console.error(`probe failed: ${e.message || e}`);
    process.exit(4);
  }
};
