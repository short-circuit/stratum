import { test, expect, type Page } from '@playwright/test';
import path from 'path';

// Before/after screenshot spec for the Journal parity work (t_4c53166a).
async function mockJournalApp(page: Page, today: string) {
  await page.addInitScript(
    ({ today }: { today: string }) => {
      const pad = (n: number) => String(n).padStart(2, '0');
      const dateStr = (d: Date) => `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
      const yPath = `journals/${dateStr(new Date(Date.now() - 86400000))}.md`;
      const twoPath = `journals/${dateStr(new Date(Date.now() - 2 * 86400000))}.md`;
      const jPath = `journals/${today}.md`;
      const pageDto = (p: string, title: string, blockCount: number) => ({ path: p, slug: title.toLowerCase().replace(/[^a-z0-9]+/g, '-'), title, block_count: blockCount, modified_at: new Date().toISOString() });
      const pages = [pageDto(jPath, 'Today Journal', 3), pageDto(yPath, 'Yesterday', 1), pageDto(twoPath, 'Two Days Ago', 1)];
      const block = (id: string, content: string, leftId?: string | null) => ({ id, content, parent_id: null, left_id: leftId ?? null, properties: [], marker: null, priority: null, collapsed: false, heading_level: null });
      const blocksByPath: Record<string, unknown[]> = {
        [jPath]: [block('t1', 'Journal entry for today', null), block('t2', 'Another line of today content', 't1'), block('t3', 'Final today block', 't2')],
        [yPath]: [block('y1', 'Yesterday journal entry', null)],
        [twoPath]: [block('z1', 'Two days ago journal entry', null)],
      };
      const now = new Date().toISOString();
      const win = window as any;
      win.__TAURI_INTERNALS__ = {
        metadata: { currentWindow: { label: 'main' }, currentWebview: { windowLabel: 'main', label: 'main' } },
        invoke(cmd: string, args: Record<string, unknown>) {
          const handlers: Record<string, () => unknown> = {
            get_vault_info: () => ({ path: '/mock/vault', block_count: 5, page_count: 3 }),
            init_vault: () => ({ path: '/mock/vault', block_count: 5, page_count: 3 }),
            init_default_vault: () => ({ path: '/mock/vault', block_count: 5, page_count: 3 }),
            list_pages: () => ({ pages }),
            open_page: () => ({ path: String(args?.path), slug: String(args?.path).toLowerCase().replace(/[^a-z0-9]+/g, '-'), title: String(args?.path), block_count: 0, modified_at: now, blocks: [] }),
            get_blocks: () => ({ blocks: blocksByPath[String(args?.pagePath)] ?? [] }),
            get_page_backlinks: () => [],
            get_backlink_snippet: () => ({ context: '', snippet: '' }),
            suggest_connections: () => [],
            resolve_link_target: () => null,
            ensure_today_journal: () => pageDto(jPath, 'Today Journal', 3),
            create_page: () => pageDto('new', 'New Page', 0),
            save_blocks: () => undefined,
            save_page: () => undefined,
            delete_page: () => undefined,
            build_markdown: () => '# Markdown',
            get_settings: () => ({ vault_path: '/mock/vault', theme: { dark_mode: true, primary_color: '#f97316', secondary_color: '#6b7280', font_size: 16 }, ai: { provider: 'ollama', endpoint: null, api_key: null, api_key_from_env: false, model: '', models: [], rag_enabled: false, rag_chunk_count: 3 }, graph: { show_connected: true, show_orphaned: true, show_tags: true, charge_strength: -4, link_distance: 40, alpha_decay: 0.15, velocity_decay: 0.4, link_curvature: 0.15, node_cap: 0 }, sync: { mode: 'manual', remote_url: null, branch: 'main', auto_commit_interval_secs: 300, auto_sync_interval_secs: 1800, ssh_key_path: null, commit_template: '' }, research: { searxng_endpoint: '', max_results: 3, max_depth: 2 }, stt: { endpoint: '', api_key: null, model: 'whisper-1', diarize_model: '', language: null, diarize: true, auto_summarize: true, auto_identify: true } }),
            save_settings: () => undefined,
            get_sync_status: () => ({ status: 'clean', branch: 'main', ahead: 0, behind: 0, conflicts: [], last_sync_time: now, last_sync_success: true, pending_commits: 0 }),
            start_sync_scheduler: () => undefined,
            stop_sync_scheduler: () => undefined,
          };
          const h = handlers[cmd];
          if (!h) { console.warn('[mock] unhandled:', cmd); return Promise.resolve(null); }
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
    { today },
  );
}

const todayStr = () => { const d = new Date(); return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`; };
// Output directory for the parity evidence screenshots. Defaults to a repo-local
// (gitignored via test-results/) directory; override with SCREENSHOT_DIR for CI.
const OUT = process.env.SCREENSHOT_DIR || 'test-results/journal-parity';

test('parity-shot mobile journal', async ({ page }, testInfo) => {
  test.setTimeout(60000);
  await mockJournalApp(page, todayStr());
  await page.goto('/journal');
  await expect(page.locator('.blocknote-editor-container').first()).toBeVisible({ timeout: 20000 });
  await page.waitForTimeout(800);
  await page.screenshot({ path: path.join(OUT, testInfo.title.replace(/[^a-z0-9]/gi, '_') + '.png'), fullPage: true });
});

test.describe('mobile viewport', () => {
  test.use({ viewport: { width: 390, height: 844 } });
  test('parity-shot mobile', async ({ page }, testInfo) => {
    test.setTimeout(60000);
    await mockJournalApp(page, todayStr());
    await page.goto('/journal');
    await expect(page.locator('.blocknote-editor-container').first()).toBeVisible({ timeout: 20000 });
    await page.waitForTimeout(800);
    await page.screenshot({ path: path.join(OUT, testInfo.title.replace(/[^a-z0-9]/gi, '_') + '.png'), fullPage: true });
  });
});

test.describe('desktop viewport', () => {
  test.use({ viewport: { width: 1280, height: 720 } });
  test('parity-shot desktop', async ({ page }, testInfo) => {
    test.setTimeout(60000);
    await mockJournalApp(page, todayStr());
    await page.goto('/journal');
    await expect(page.locator('.blocknote-editor-container').first()).toBeVisible({ timeout: 20000 });
    await page.waitForTimeout(800);
    await page.screenshot({ path: path.join(OUT, testInfo.title.replace(/[^a-z0-9]/gi, '_') + '.png'), fullPage: true });
  });
});
