import { test, expect, type Page } from '@playwright/test';
import path from 'path';
import { mockTauriInvoke, DEFAULT_MOCK_CONFIG } from '../mocks/tauri';

// ---------------------------------------------------------------------------
// QA parity screenshot capture (t_8b915df2): Settings + Journal on mobile
// small/large and desktop, for visual-consistency inspection.
// ---------------------------------------------------------------------------

const MOBILE_SMALL = { width: 360, height: 740 };
const MOBILE_LARGE = { width: 428, height: 900 };
const DESKTOP = { width: 1280, height: 720 };

const MOBILE_UA =
  'Mozilla/5.0 (Linux; Android 14; Pixel 7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Mobile Safari/537.36';

const OUT = process.env.SCREENSHOT_DIR || 'test-results/qa-parity-shots';

function localDate(d: Date): string {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, '0');
  const day = String(d.getDate()).padStart(2, '0');
  return `${y}-${m}-${day}`;
}

// --- Journal mock (matches qj-parity-journal-qa) ---------------------------
async function mockJournalApp(page: Page) {
  const today = localDate(new Date());
  const offset = (n: number) => {
    const d = new Date();
    d.setDate(d.getDate() - n);
    return localDate(d);
  };
  const yesterday = offset(1);
  const twoDays = offset(2);
  const now = new Date().toISOString();
  const mkPage = (p: string, title: string, bc: number) => ({
    path: p,
    slug: title.toLowerCase().replace(/[^a-z0-9]+/g, '-'),
    title,
    block_count: bc,
    modified_at: now,
  });
  const pages = [
    mkPage(`journals/${today}.md`, 'Today Journal', 1),
    mkPage(`journals/${yesterday}.md`, 'Yesterday', 1),
    mkPage(`journals/${twoDays}.md`, 'Two Days Ago', 1),
  ];
  await page.addInitScript(
    ({ pages, today }: { pages: unknown[]; today: string }) => {
      const block = (id: string, content: string, leftId: string | null = null) => ({
        id, content, parent_id: null, left_id: leftId, properties: [], marker: null, priority: null, collapsed: false, heading_level: null,
      });
      const blocksByPath: Record<string, unknown[]> = {
        [`journals/${today}.md`]: [block('t1', 'Journal entry for today'), block('t2', 'Second journal line for today', 't1')],
      };
      const win = window as any;
      win.__TAURI_INTERNALS__ = {
        metadata: { currentWindow: { label: 'main' }, currentWebview: { windowLabel: 'main', label: 'main' } },
        invoke(cmd: string, args: Record<string, unknown>) {
          const handlers: Record<string, () => unknown> = {
            get_vault_info: () => ({ path: '/mock/vault', block_count: 3, page_count: 3 }),
            init_vault: () => ({ path: '/mock/vault', block_count: 3, page_count: 3 }),
            init_default_vault: () => ({ path: '/mock/vault', block_count: 3, page_count: 3 }),
            list_pages: () => ({ pages }),
            open_page: () => ({ path: String(args?.path), slug: 'x', title: String(args?.path), block_count: 0, modified_at: new Date().toISOString(), blocks: [] }),
            get_blocks: () => ({ blocks: blocksByPath[String(args?.pagePath)] ?? [] }),
            get_page_backlinks: () => [],
            get_backlink_snippet: () => ({ context: '', snippet: '' }),
            suggest_connections: () => [],
            resolve_link_target: () => null,
            ensure_today_journal: () => ({ path: `journals/${today}.md`, slug: 'today', title: 'Today Journal', block_count: 1, modified_at: new Date().toISOString() }),
            create_page: () => ({ path: String(args?.path ?? 'x'), slug: 'x', title: String(args?.path ?? 'x'), block_count: 0, modified_at: new Date().toISOString() }),
            save_blocks: () => undefined,
            save_page: () => undefined,
            delete_page: () => undefined,
            build_markdown: () => '# M',
            get_settings: () => ({ vault_path: '/mock/vault', theme: { dark_mode: true, primary_color: '#f97316', secondary_color: '#6b7280', font_size: 16 }, ai: { provider: 'ollama', endpoint: null, api_key: null, api_key_from_env: false, model: '', models: [], rag_enabled: false, rag_chunk_count: 3 }, graph: { show_connected: true, show_orphaned: true, show_tags: true, charge_strength: -4, link_distance: 40, alpha_decay: 0.15, velocity_decay: 0.4, link_curvature: 0.15, node_cap: 0 }, sync: { mode: 'manual', remote_url: null, branch: 'main', auto_commit_interval_secs: 300, auto_sync_interval_secs: 1800, ssh_key_path: null, commit_template: '' }, research: { searxng_endpoint: '', max_results: 3, max_depth: 2 }, stt: { endpoint: '', api_key: null, model: 'whisper-1', diarize_model: '', language: null, diarize: true, auto_summarize: true, auto_identify: true } }),
            save_settings: () => undefined,
            get_sync_status: () => ({ status: 'clean', branch: 'main', ahead: 0, behind: 0, conflicts: [], last_sync_time: new Date().toISOString(), last_sync_success: true, pending_commits: 0 }),
            start_sync_scheduler: () => undefined,
            stop_sync_scheduler: () => undefined,
          };
          const h = handlers[cmd];
          if (!h) return Promise.resolve(null);
          try { return Promise.resolve(h()); } catch (e) { return Promise.reject(e); }
        },
        convertFileSrc(p: string) { return 'asset://' + p; },
        transformCallback() { return 0; },
        unregisterCallback() {},
        runCallback() {},
        callbacks: new Map(),
      };
      win.__TAURI_EVENT_PLUGIN_INTERNALS__ = {};
    },
    { pages, today },
  );
}

const shot = async (page: Page, name: string) => {
  await page.screenshot({ path: path.join(OUT, name + '.png'), fullPage: true });
};

for (const [label, viewport, ua] of [
  ['mobile-small', MOBILE_SMALL, MOBILE_UA],
  ['mobile-large', MOBILE_LARGE, MOBILE_UA],
  ['desktop', DESKTOP, undefined],
] as const) {
  test.describe(`capture ${label}`, () => {
    test.use({ viewport, userAgent: ua });

    test(`settings`, async ({ page }) => {
      await mockTauriInvoke(page, DEFAULT_MOCK_CONFIG);
      await page.goto('/settings');
      await expect(page.getByRole('button', { name: /^Save$/ })).toBeVisible({ timeout: 15000 });
      await page.waitForTimeout(500);
      await shot(page, `settings-${label}`);
    });

    test(`journal`, async ({ page }) => {
      await mockJournalApp(page);
      await page.goto('/journal');
      await expect(page.locator('.blocknote-editor-container').first()).toBeVisible({ timeout: 20000 });
      await page.waitForTimeout(800);
      await shot(page, `journal-${label}`);
    });
  });
}
