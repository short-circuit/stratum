import { chromium } from 'playwright';
import { fileURLToPath } from 'url';
import path from 'path';
import fs from 'fs';

// ---------------------------------------------------------------------------
// Stratum Settings mobile screenshot capture.
// Drives the Vite preview (dist) with the mocked Tauri IPC surface (mocks/tauri)
// at realistic mobile viewports, so before/after parity screenshots can be
// produced headlessly without a device or the Rust runtime.
//
// Usage:
//   npm run build                      # must exist before the capture
//   npm run preview &                  # or the script starts its own preview
//   node e2e/capture/capture-settings.mjs <outdir> [before|after]
// ---------------------------------------------------------------------------

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const SKILL_DIR = path.resolve(__dirname);
const REPO = path.resolve(SKILL_DIR, '../..');

// The mock init-script injection source lives in e2e/mocks/tauri.ts. We can't
// import it directly (it's TS). Instead we parse the mock's inline handler by
// evaluating the same payload used by the spec suite. To avoid re-implementing
// the mock, we read the TS file and extract the addInitScript content string.
// Simpler and robust: re-implement a minimal equivalent mock inline here
// targeting exactly the commands the Settings screen calls. This keeps the
// capture self-contained and decoupled from test internals.
function buildMockSettings() {
  return {
    vault_path: '/storage/emulated/0/Documents/StratumVault',
    theme: { dark_mode: true, primary_color: '#f97316', secondary_color: '#6b7280', font_size: 16 },
    ai: {
      provider: 'ollama',
      endpoint: 'http://localhost:11434',
      api_key: null,
      api_key_from_env: false,
      model: 'qwen2.5:7b',
      models: [
        { name: 'qwen2.5:7b', capabilities: ['chat'] },
        { name: 'nomic-embed-text', capabilities: ['embedding'] },
      ],
      rag_enabled: true,
      rag_chunk_count: 5,
      embedding_dimensions: 0,
    },
    graph: { show_connected: true, show_orphaned: true, show_tags: true, charge_strength: -30, link_distance: 100, alpha_decay: 0.02, velocity_decay: 0.4 },
    sync: {
      mode: 'auto_commit',
      remote_url: 'git@github.com:user/vault.git',
      branch: 'main',
      auto_commit_interval_secs: 300,
      auto_sync_interval_secs: 1800,
      ssh_key_path: '/storage/emulated/0/keys/id_ed25519',
      commit_template: 'stratum({datetime}): {editedfiles} edited, {newfiles} added, {deletedfiles} deleted',
    },
    research: { searxng_endpoint: 'http://localhost:8888', max_results: 3, max_depth: 2 },
    stt: {
      endpoint: 'http://localhost:8081',
      api_key: null,
      model: 'whisper-1',
      diarize_model: 'pyannote-diarization',
      language: null,
      diarize: true,
      auto_summarize: true,
      auto_identify: true,
    },
    tts: { endpoint: '', api_key: null, voice: 'alloy', format: 'mp3', speed: 1.0 },
  };
}

function mockInitScript() {
  const settings = buildMockSettings();
  return `(function() {
    const mockSettings = ${JSON.stringify(settings)};
    const handlers = {
      get_vault_info: () => ({ path: '/storage/emulated/0/Documents/StratumVault', block_count: 14, page_count: 5 }),
      list_pages: () => ({ pages: [
        { path: 'Welcome', slug: 'welcome', title: 'Welcome', block_count: 3, modified_at: '2026-07-28T12:00:00Z' },
        { path: 'Projects', slug: 'projects', title: 'Projects', block_count: 4, modified_at: '2026-07-28T12:00:00Z' },
      ]}),
      get_settings: () => mockSettings,
      save_settings: (args) => {},
      fetch_models: () => ['qwen2.5:7b', 'nomic-embed-text'],
      reindex_vault: () => ({ processed: 14, succeeded: 14, failed: 0, errors: [] }),
      repair_db_from_disk: () => ({ processed: 14, succeeded: 14, failed: 0, errors: [] }),
      normalize_all_files: () => 14,
      get_sync_status: () => ({ status: 'ok', branch: 'main', ahead: 2, behind: 1, conflicts: [], last_sync_time: '2026-07-28T12:00:00Z', last_sync_success: true, pending_commits: 0 }),
      sync_vault: () => ({ status: 'ok', branch: 'main', ahead: 0, behind: 0, conflicts: [], last_sync_time: '2026-07-28T12:00:00Z', last_sync_success: true, pending_commits: 0 }),
      start_sync_scheduler: () => {},
      get_commit_log: () => [
        { hash: 'a1b2c3d', author: 'dev', message: 'feat: add graph view', timestamp: '2026-07-27T10:00:00Z' },
        { hash: 'e4f5g6h', author: 'dev', message: 'fix: search indexing', timestamp: '2026-07-26T14:00:00Z' },
      ],
      stt_test_connection: () => ({ ok: true, models: ['whisper-1'], latency_ms: 320, error: null }),
      tts_generate: () => ({ mime: 'audio/mpeg', audio_b64: '', byte_len: 0 }),
    };
    window.__TAURI_INTERNALS__ = {
      metadata: { currentWindow: { label: 'main' }, currentWebview: { windowLabel: 'main', label: 'main' } },
      invoke: function(cmd, args, opts) {
        const h = handlers[cmd];
        if (!h) { console.warn('[capture mock] unhandled command:', cmd); return Promise.resolve(null); }
        try { return Promise.resolve(h(args)); } catch (e) { return Promise.reject(e); }
      },
      convertFileSrc: function(p) { return 'asset://' + p; },
      transformCallback: function() { return 0; },
      unregisterCallback: function() {},
      runCallback: function() {},
      callbacks: new Map(),
    };
    window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {};
  })()`;
}

const VIEWPORTS = [
  { name: 'small', width: 360, height: 740, isMobile: true },
  { name: 'large', width: 428, height: 900, isMobile: true },
];

async function main() {
  const outdir = process.argv[2];
  const phase = process.argv[3] || 'after';
  if (!outdir) {
    console.error('usage: node e2e/capture/capture-settings.mjs <outdir> [before|after]');
    process.exit(2);
  }
  fs.mkdirSync(outdir, { recursive: true });

  const baseURL = process.env.CAPTURE_BASEURL || 'http://localhost:4173';
  const browser = await chromium.launch();
  const results = [];

  for (const vp of VIEWPORTS) {
    const context = await browser.newContext({
      viewport: { width: vp.width, height: vp.height },
      // Force the mobile split: the app's useResponsive gates on getPlatform()
      // which reads the UA (android/ios) or width<768. For a webkit browser on
      // desktop we must emulate a mobile UA + touch so `isMobile` is true.
      userAgent:
        'Mozilla/5.0 (Linux; Android 14; Pixel 7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0 Mobile Safari/537.36 Stratum/0.7.1',
      hasTouch: true,
      isMobile: true,
      deviceScaleFactor: 2,
    });
    const page = await context.newPage();
    await page.addInitScript({ content: mockInitScript() });
    await page.goto(`${baseURL}/settings`, { waitUntil: 'networkidle' });
    await page.waitForTimeout(1500);

    const file = path.join(outdir, `settings-${phase}-${vp.name}-${vp.width}x${vp.height}.png`);
    await page.screenshot({ path: file, fullPage: true });
    results.push(file);
    console.log(`captured: ${file}`);

    // Expand the AI accordion and capture the model-fetch/capability+RAG area.
    const aiHandle = page.getByText('Configure AI provider');
    if (await aiHandle.count()) {
      await aiHandle.first().click();
      await page.waitForTimeout(400);
      // Populate the model capability editor by clicking "Fetch Available Models".
      const fetchBtn = page.getByText('Fetch Available Models');
      if (await fetchBtn.count()) {
        await fetchBtn.first().click();
        await page.waitForTimeout(500);
      }
      const aiFile = path.join(
        outdir,
        `settings-${phase}-${vp.name}-ai-${vp.width}x${vp.height}.png`
      );
      await page.screenshot({ path: aiFile, fullPage: true });
      results.push(aiFile);
      console.log(`captured: ${aiFile}`);
    }

    // Expand Speech & Audio and capture the STT/TTS config area.
    const sttHandle = page.getByText('Configure dictation & voice');
    if (await sttHandle.count()) {
      await sttHandle.first().click();
      await page.waitForTimeout(400);
      const sttFile = path.join(
        outdir,
        `settings-${phase}-${vp.name}-stt-${vp.width}x${vp.height}.png`
      );
      await page.screenshot({ path: sttFile, fullPage: true });
      results.push(sttFile);
      console.log(`captured: ${sttFile}`);
    }

    // Scroll the settings container to the bottom (Sync section lands in view).
    const scrolled = await page.evaluate(() => {
      const divs = Array.from(document.querySelectorAll('div'));
      const scrollable = divs.find(
        d =>
          d.scrollHeight > d.clientHeight + 50 &&
          getComputedStyle(d).overflowY === 'auto'
      );
      if (scrollable) {
        scrollable.scrollTop = scrollable.scrollHeight;
        return { ok: true, sh: scrollable.scrollHeight, ch: scrollable.clientHeight };
      }
      return { ok: false };
    });
    await page.waitForTimeout(500);
    const syncFile = path.join(
      outdir,
      `settings-${phase}-${vp.name}-sync-${vp.width}x${vp.height}.png`
    );
    await page.screenshot({ path: syncFile });
    results.push(syncFile);
    console.log(
      `captured: ${syncFile} (scrolled=${JSON.stringify(scrolled)})`
    );

    await context.close();
  }

  await browser.close();
  console.log('DONE');
}

main().catch(err => {
  console.error(err);
  process.exit(1);
});
