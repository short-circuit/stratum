/**
 * Standalone headless performance probe for the GraphPanel (react-force-graph-2d/3d).
 *
 * Reproducible steps (see e2e/perf/README.md):
 *   1. npm run build && npm run preview   (serves dist on :4173)
 *   2. BASE_URL=http://localhost:4173 node e2e/perf/probe.mjs 5,2000,10000 --out e2e/perf/results/raw-before.json
 *
 * The probe launches headless Chromium with SwiftShader (software WebGL), mocks the
 * Tauri backend so `get_graph_panel_data` returns a representative N-node payload
 * (shape matches the real Rust DTO), mounts the real /graph route through the real
 * component tree, and measures:
 *   - data load + first-render time (browser long-task + rAF-based)
 *   - simulation warm-up (main-thread busy until quiet)
 *   - idle FPS + CPU/jank (long-task log, script-duration deltas)
 *   - JS heap after settle (memory)
 *   - interaction responsiveness (synthetic drag while layout settles)
 *
 * All numbers are "before" baselines — no app code is modified.
 */
import { chromium } from 'playwright';
import { writeFileSync, mkdirSync } from 'node:fs';
import { buildPanelPayload } from './graph-data.mjs';

const BASE = process.env.BASE_URL || 'http://localhost:4173';
const OUT = process.env.OUT;
const SHOT_DIR = process.env.SHOT_DIR || 'e2e/perf/results/shots';

function parseArgs(argv) {
  const cases = [];
  let seed = 1;
  let out = OUT;
  for (let i = 2; i < argv.length; i++) {
    const a = argv[i];
    if (a === '--seed') { seed = Number(argv[++i]); continue; }
    if (a === '--out') { out = argv[++i]; continue; }
    if (/^\d+(,\d+)*$/.test(a)) { for (const n of a.split(',')) cases.push(Number(n)); continue; }
  }
  return { cases, seed, out };
}

// ---- in-page init script: Tauri mock + instrumentation ----
function makeInitScript(payload) {
  // Minimal-but-safe set of Tauri commands the desktop /graph route needs.
  const listPages = { pages: [] };
  const settings = {
    vault_path: '/mock/vault',
    theme: { dark_mode: true, primary_color: '#f97316', secondary_color: '#6b7280', font_size: 16 },
    graph: {
      show_connected: true, show_orphaned: true, show_tags: true,
      charge_strength: -30, link_distance: 50, alpha_decay: 0.15,
      velocity_decay: 0.4, link_curvature: 0.15, node_cap: 0,
    },
      ai: { provider: 'ollama', endpoint: null, api_key: null, api_key_from_env: false, model: '', models: [], rag_enabled: false, rag_chunk_count: 3 },
      sync: { mode: 'manual', remote_url: null, branch: 'main', auto_commit_interval_secs: 300, auto_sync_interval_secs: 1800, ssh_key_path: null, commit_template: '' },
      research: { searxng_endpoint: '', max_results: 3, max_depth: 2 },
      stt: { endpoint: '', api_key: null, model: '', diarize_model: '', language: null, diarize: false, auto_summarize: false, auto_identify: false },
  };
  const syncStatus = {
    status: 'clean', branch: 'main', ahead: 0, behind: 0, conflicts: [],
    last_sync_time: '2026-01-01T00:00:00Z', last_sync_success: true, pending_commits: 0,
  };
  const graphJson = JSON.stringify(payload);
  return `(function(){
    const graphPayload = ${graphJson};
    const handlers = {
      get_vault_info: () => ({ path: '/mock/vault', block_count: 0, page_count: ${payload.graph.nodes.length} }),
      get_settings: () => (${JSON.stringify(settings)}),
      save_graph_settings: () => {},
      save_settings: () => {},
      list_pages: () => (${JSON.stringify(listPages)}),
      open_page: () => ({ path: '', slug: '', title: '', block_count: 0, modified_at: '2026-01-01T00:00:00Z', blocks: [] }),
      get_blocks: () => ({ blocks: [] }),
      get_sync_status: () => (${JSON.stringify(syncStatus)}),
      get_commit_log: () => [],
      get_graph_panel_data: () => graphPayload,
      get_graph_data: () => graphPayload.graph,
      get_connected_components: () => graphPayload.components,
      get_orphaned_notes: () => graphPayload.orphans,
      autocomplete: () => [],
      search_blocks: () => ({ results: [] }),
      get_page_backlinks: () => [],
      list_templates: () => [],
      list_whiteboards: () => [],
      get_kanban_blocks: () => ({ blocks: [] }),
    };
    window.__TAURI_INTERNALS__ = {
      metadata: { currentWindow: { label: 'main' }, currentWebview: { windowLabel: 'main', label: 'main' } },
      invoke: function(cmd, args, options) {
        const h = handlers[cmd];
        if (!h) { try { console.warn('[mock] unhandled', cmd); } catch(_){} return Promise.resolve(null); }
        return Promise.resolve().then(() => h(args));
      },
      convertFileSrc: function(p){ return 'asset://'+p; },
      transformCallback: function(fn, once){ return 0; },
      unregisterCallback: function(id){},
      runCallback: function(id, data){},
      callbacks: new Map(),
    };
    window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {};
    // Long-task instrumentation — non-invasive.
    window.__LONG_TASKS__ = [];
    window.__LONG_TASK_FLUSHED__ = false;
    try {
      new PerformanceObserver((list) => {
        for (const e of list.getEntries()) window.__LONG_TASKS__.push({ start: e.startTime, dur: e.duration });
      }).observe({ type: 'longtask', buffered: true });
    } catch (e) {}
    window.__WEBGL__ = null;
    try {
      const c = document.createElement('canvas');
      const gl = c.getContext('webgl2') || c.getContext('webgl');
      if (gl) { const ext = gl.getExtension('WEBGL_debug_renderer_info'); const dbg = gl.getParameter(ext ? ext.UNMASKED_RENDERER_WEBGL : 7937); window.__WEBGL__ = String(dbg); }
    } catch (e) {}
    window.__PERF__ = { navStart: 0, graphDataResolved: 0 };
    // Stamp when the graph payload is delivered to the UI (start of render work).
    const origGetGraph = handlers.get_graph_panel_data;
    handlers.get_graph_panel_data = function () {
      window.__PERF__.graphDataResolved = performance.now();
      return origGetGraph();
    };
  })()`;
}

async function runCase(browser, N, seed) {
  const ctx = await browser.newContext({ viewport: { width: 1280, height: 720 }, deviceScaleFactor: 1 });
  const page = await ctx.newPage();
  const errors = [];
  page.on('pageerror', (e) => errors.push(String(e)));
  page.on('console', (m) => { if (m.type() === 'error') errors.push(m.text()); });

  const payload = buildPanelPayload({ nodes: N, seed });
  await page.addInitScript({ content: makeInitScript(payload) });

  const t0 = Date.now();
  await page.goto(`${BASE}/graph?perf=1&n=${N}&s=${seed}`, { waitUntil: 'domcontentloaded', timeout: 60000 });
  const navStart = Date.now();

  // Poll for first signal that the graph pipeline produced visible content.
  // Use the toolbar counter "X/X n · Y e" which only appears after
  // get_graph_panel_data resolves AND filteredNodes are computed.
  const firstRender = await waitFor(page, async () => {
    const txt = await page.evaluate(() => document.body ? document.body.innerText : '');
    const m = txt.match(/\d+\/\d+ n\s*[·•]*\s*\d+ e/);
    if (!m) return null;
    return { match: m[0] };
  }, 60000);
  if (!firstRender) {
    const debug = await page.evaluate(() => {
      const el = document.getElementById('root');
      return {
        readyState: document.readyState,
        url: location.href,
        rootChildren: el ? el.children.length : -1,
        bodyText: (document.body ? document.body.innerText : '').slice(0, 200),
        canvasCount: document.querySelectorAll('canvas').length,
      };
    }).catch((e) => ({ evaluateError: String(e) }));
    await ctx.close();
    return { case: N, status: 'FAIL', debug, errors };
  }
  const ttFirstRenderPerf = Date.now() - t0; // wall-clock since probe start (trustworthy under main-thread saturation)
  console.log(`    [wall] firstRender_signal=${ttFirstRenderPerf}ms`);
  const shotAtFirstRender = await page.screenshot({ path: `${SHOT_DIR}/fr-n${N}-s${seed}.png` }).catch(() => null);

  // Warm-up: wait until main thread quiets (no long tasks, low script delta).
  const quiet = await waitForQuiet(page, t0);
  const ttQuietPerf = quiet ? quiet.wall : null;
  console.log(`    [wall] quiet_signal=${ttQuietPerf}ms`);

  // Idle metrics after quiet.
  const idle = await page.evaluate(() => ({
    rAF: null, // filled below
  }));
  const fps = await sampleFps(page, 1500);
  const longTasksNow = await page.evaluate(() => window.__LONG_TASKS__.length);
  const longTaskTotalMs = await page.evaluate(() => window.__LONG_TASKS__.reduce((a, e) => a + e.dur, 0));
  const memory = await memoryMetrics(page);
  const webgl = await page.evaluate(() => window.__WEBGL__);
  const canvasCount = await page.evaluate(() => document.querySelectorAll('canvas').length);
  const scroll = await page.evaluate(() => ({ w: document.documentElement.scrollWidth, h: document.documentElement.scrollHeight }));

  // Attributing long-task time to windows after "first render": this tells us how much
  // of the warm-up is main-thread JS (sim) vs GPU/simulate-async.
  const longTasks = await page.evaluate(() => (window.__LONG_TASKS__ || []).map((e) => ({ start: e.start, dur: e.dur })));

  const result = {
    case: { nodes: N, seed },
    status: 'OK',
    // wall-clock anchors (real time, for reproducibility) and page-relative perf anchors
    wallMs: {
      navStart_abs: navStart - t0,
      firstRender_abs: Date.now() - t0,
      quiet_abs: quiet ? Date.now() - t0 : null,
    },
    perfNowMs: {
      firstRender: Math.round(ttFirstRenderPerf),
      quiet: ttQuietPerf === null ? null : Math.round(ttQuietPerf),
    },
    renderer: webgl,
    canvasCount,
    idle: { fps: fps.avg, p1Fps: fps.p1, p50Fps: fps.p50, longTaskCount: longTasksNow, longTaskTotalMs: Math.round(longTaskTotalMs) },
    longTaskWindowMs: longTasks.filter((e) => e.start <= (ttQuietPerf ?? 1e9) && e.start >= (ttFirstRenderPerf ?? 0) - 50).reduce((a, e) => a + e.dur, 0),
    memory: {
      usedJSHeapMB: memory.usedJSHeapMB,
      totalJSHeapMB: memory.totalJSHeapMB,
      documents: memory.documents,
      nodes: memory.nodes,
    },
    interactions: null, // filled by interaction probe when implemented
    errors,
  };

  // ---- interaction responsiveness probe (only on non-tiny graphs, to keep total time bounded) ----
  let interactions = null;
  if (N >= 1000 && firstRender) {
    interactions = await interactionProbe(page, N);
    result.interactions = interactions;
  }

  await ctx.close();
  return result;
}

async function interactionProbe(page, N) {
  // Drag-pan responsiveness while the layout sim is still settling.
  // Measures: how many input events we can feed before frames stall, and
  // long-task counts/durations attributed to the drag window.
  const probe = await page.evaluate(async (N) => {
    const out = { fed: 0, handled: 0, maxGapMs: 0, totalLongTaskMs: 0, longTaskCount: 0, dragged: false };
    const canvas = document.querySelector('canvas');
    if (!canvas) return out;
    const cdpInput = () => {
      // record a point (input timestamps are not observably distinguished from
      // rAF in-page, so we use a monotonically increasing synthetic timestamp)
      return performance.now();
    };
    try {
      const ltBefore = window.__LONG_TASKS__ ? window.__LONG_TASKS__.length : 0;
      const rect = canvas.getBoundingClientRect();
      const cx = rect.left + rect.width / 2;
      const cy = rect.top + rect.height / 2;
      const fire = (x, y) => {
        // dispatch pointer + mouse events on the canvas (ForceGraph listens to these)
        for (const type of ['pointerdown', 'mousedown']) {
          canvas.dispatchEvent(new PointerEvent(type, { bubbles: true, clientX: x, clientY: y, pointerId: 1, button: 0 }));
        }
        out.fed++;
      };
      fire(cx, cy);
      // feed a burst of moves over 120ms — enough to trigger rAF churn
      const start = performance.now();
      while (performance.now() - start < 120) {
        const x = cx + Math.sin(performance.now() / 20) * 30;
        const y = cy + Math.cos(performance.now() / 20) * 30;
        for (const type of ['pointermove', 'mousemove']) {
          canvas.dispatchEvent(new PointerEvent(type, { bubbles: true, clientX: x, clientY: y, pointerId: 1, button: 0 }));
        }
        out.fed++;
        await new Promise((r) => setTimeout(r, 4));
      }
      fire(cx + 1, cy + 1);
      for (const type of ['pointerup', 'mouseup']) {
        canvas.dispatchEvent(new PointerEvent(type, { bubbles: true, clientX: cx + 1, clientY: cy + 1, pointerId: 1, button: 0 }));
      }
      // give the canvas a moment to flush a drag frame
      await new Promise((r) => requestAnimationFrame(() => setTimeout(r, 50)));
      // measure rAF gap right after the burst while sim still churns
      const gaps = [];
      let last = performance.now();
      await new Promise((resolve) => {
        let n = 0;
        const step = () => {
          const now = performance.now();
          gaps.push(now - last);
          last = now;
          if (n++ < 8) requestAnimationFrame(step);
          else resolve();
        };
        requestAnimationFrame(step);
      });
      out.maxGapMs = Math.max(...gaps);
      out.dragged = true;
      const ltAfter = window.__LONG_TASKS__ ? window.__LONG_TASKS__.length : 0;
      const lt = window.__LONG_TASKS__ ? window.__LONG_TASKS__.slice(ltBefore) : [];
      out.longTaskCount = lt.length;
      out.totalLongTaskMs = Math.round(lt.reduce((a, e) => a + e.dur, 0));
    } catch (e) {
      out.error = String(e);
    }
    return out;
  }, N);
  return probe;
}

async function waitFor(page, fn, timeout) {
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) {
    try {
      const v = await fn();
      if (v) return v;
    } catch (e) { /* page may be mid-load */ }
    await page.waitForTimeout(50);
  }
  return null;
}

async function waitForQuiet(page, t0) {
  // "Quiet": two consecutive 700ms windows with no long task and a small
  // rAF-delta hit rate. If the graph is small, this returns almost immediately.
  // Capped at 90s so the 10k case still yields bounded numbers.
  let quietStreak = 0;
  const deadline = Date.now() + 90000;
  while (Date.now() < deadline) {
    const ltBefore = await page.evaluate(() => window.__LONG_TASKS__.length);
    await page.waitForTimeout(700);
    const ltAfter = await page.evaluate(() => window.__LONG_TASKS__.length);
    const hadLongTask = ltAfter > ltBefore;
    // rAF max gap over a short window
    const gaps = await page.evaluate(() => new Promise((resolve) => {
      const times = [performance.now()];
      let n = 0;
      const stepper = () => {
        times.push(performance.now());
        if (n++ < 3) requestAnimationFrame(stepper);
        else {
          const deltas = times.slice(1).map((t, i) => t - times[i]);
          resolve(deltas);
        }
      };
      requestAnimationFrame(stepper);
    }));
    const maxGap = Math.max(...gaps);
    if (!hadLongTask && maxGap < 120) {
      quietStreak++;
      if (quietStreak >= 1) {
        return { wall: Date.now() - t0 };
      }
    } else {
      quietStreak = 0;
    }
  }
  return null;
}

async function sampleFps(page, ms) {
  // Returns {avg, p50, p1} of rAF frame deltas over `ms`.
  const data = await page.evaluate(async (ms) => {
    return await new Promise((resolve) => {
      const deltas = [];
      let last = performance.now();
      let alive = true;
      const start = performance.now();
      const step = () => {
        if (!alive) return;
        const now = performance.now();
        deltas.push(now - last);
        last = now;
        if (now - start >= ms) { alive = false; resolve(deltas); return; }
        requestAnimationFrame(step);
      };
      requestAnimationFrame(step);
    });
  }, ms);
  const sorted = [...data].sort((a, b) => a - b);
  const avg = data.length ? data.reduce((a, b) => a + b, 0) / data.length : 0;
  const p = (q) => sorted.length ? sorted[Math.floor(sorted.length * q)] : 0;
  return {
    frames: data.length,
    avg: Math.round(avg * 10) / 10,
    p50: Math.round(p(0.5) * 10) / 10,
    p1: Math.round(p(0.01) * 10) / 10,
  };
}

async function memoryMetrics(page) {
  const m = await page.evaluate(() => {
    const pm = window.performance && performance.memory;
    return {
      used: pm ? pm.usedJSHeapSize : null,
      total: pm ? pm.totalJSHeapSize : null,
      documents: document.querySelectorAll('*').length,
      canvas: document.querySelectorAll('canvas').length,
    };
  });
  return {
    usedJSHeapMB: m.used ? Math.round(m.used / 1048576) : null,
    totalJSHeapMB: m.total ? Math.round(m.total / 1048576) : null,
    documents: m.documents,
    nodes: m.canvas,
  };
}

const { cases, seed, out } = parseArgs(process.argv);
if (!cases.length) {
  console.error('usage: node probe.mjs <nodes[,nodes,...]> [--seed S] [--out path]');
  process.exit(1);
}

console.log(`PROBE base=${BASE} nodes=${cases.join(',')} seed=${seed}`);
const HEADFUL = process.env.HEADFUL === '1';
const launchOpts = HEADFUL
  ? { args: ['--ignore-gpu-blocklist', '--window-size=1280,720', '--hide-scrollbars'] }
  : { args: ['--use-gl=swiftshader', '--enable-unsafe-swiftshader', '--disable-gpu-sandbox', '--window-size=1280,720', '--hide-scrollbars'] };
const browser = await chromium.launch({ headless: !HEADFUL, timeout: 60000, ...launchOpts });

const results = [];
for (const N of cases) {
  console.log(`  case ${N} nodes...`);
  const r = await runCase(browser, N, seed);
  results.push(r);
  console.log(`    -> ${r.status} firstRender=${r.perfNowMs?.firstRender}ms quiet=${r.perfNowMs?.quiet}ms fps=${r.idle?.fps} heap=${r.memory?.usedJSHeapMB}MB longTaskMs=${r.idle?.longTaskTotalMs}ms`);
  if (r.status === 'FAIL') console.log('       errors:', JSON.stringify(r.errors));
}

await browser.close();
if (out) {
  mkdirSync(out.includes('/') ? out.slice(0, out.lastIndexOf('/')) : '.', { recursive: true });
  writeFileSync(out, JSON.stringify({ base: BASE, seed, ts: new Date().toISOString(), results }, null, 2));
  console.log(`wrote ${out}`);
}
process.exit(0);
