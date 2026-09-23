import { test, expect, type Page } from '@playwright/test';

// ---------------------------------------------------------------------------
// Cross-platform parity QA for Journal (t_8b915df2).
//
// Verifies parity between mobile (small + large) and desktop for the Journal
// panel: navigation affordances (Prev/Next day, calendar), the today editor,
// persistence (navigating to another day and back, create-on-demand), and the
// error state (ensure_today_journal failure surfaces a retry path).
//
// The mock seeds today's + previous days' journal pages with blocks so the
// panel renders a real editor, mirroring the sibling implementations'
// journal-parity harnesses.
// ---------------------------------------------------------------------------

const MOBILE_SMALL = { width: 360, height: 740 };
const MOBILE_LARGE = { width: 428, height: 900 };
const DESKTOP = { width: 1280, height: 720 };

const MOBILE_UA =
  'Mozilla/5.0 (Linux; Android 14; Pixel 7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Mobile Safari/537.36';

function localDate(d: Date): string {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, '0');
  const day = String(d.getDate()).padStart(2, '0');
  return `${y}-${m}-${day}`;
}

type JournalPages = { today: string; yesterday: string; twoDays: string; serialized: Array<{ path: string; slug: string; title: string; block_count: number; modified_at: string }> };

function buildJournalSeed(): JournalPages {
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
  const serialized = [
    mkPage(`journals/${today}.md`, 'Today Journal', 1),
    mkPage(`journals/${yesterday}.md`, 'Yesterday', 1),
    mkPage(`journals/${twoDays}.md`, 'Two Days Ago', 1),
  ];
  return { today, yesterday, twoDays, serialized };
}

async function mockJournalApp(page: Page, opts: { failEnsure?: boolean } = {}) {
  const seed = buildJournalSeed();
  await page.addInitScript(
    ({ seed, failEnsure }: { seed: JournalPages; failEnsure: boolean }) => {
      const block = (id: string, content: string, leftId: string | null = null) => ({
        id,
        content,
        parent_id: null,
        left_id: leftId,
        properties: [],
        marker: null,
        priority: null,
        collapsed: false,
        heading_level: null,
      });
      const blocksByPath: Record<string, unknown[]> = {
        [`journals/${seed.today}.md`]: [block('t1', 'Journal entry for today')],
        [`journals/${seed.yesterday}.md`]: [block('y1', 'Yesterday journal entry')],
        [`journals/${seed.twoDays}.md`]: [block('z1', 'Two days ago journal entry')],
      };
      const now = new Date().toISOString();
      // When failEnsure is set, omit today's page from the listing so the
      // panel's ensure-today path is exercised (and rejects).
      const pages = failEnsure
        ? seed.serialized.filter(p => p.path !== `journals/${seed.today}.md`)
        : seed.serialized;
      const win = window as any;
      win.__TAURI_INTERNALS__ = {
        metadata: { currentWindow: { label: 'main' }, currentWebview: { windowLabel: 'main', label: 'main' } },
        invoke(cmd: string, args: Record<string, unknown>) {
          const handlers: Record<string, () => unknown> = {
            get_vault_info: () => ({ path: '/mock/vault', block_count: 3, page_count: 3 }),
            init_vault: () => ({ path: '/mock/vault', block_count: 3, page_count: 3 }),
            init_default_vault: () => ({ path: '/mock/vault', block_count: 3, page_count: 3 }),
            list_pages: () => ({ pages }),
            open_page: () => ({ path: String(args?.path), slug: String(args?.path).toLowerCase().replace(/[^a-z0-9]+/g, '-'), title: String(args?.path), block_count: 0, modified_at: now, blocks: [] }),
            get_blocks: () => ({ blocks: blocksByPath[String(args?.pagePath)] ?? [] }),
            get_page_backlinks: () => [],
            get_backlink_snippet: () => ({ context: '', snippet: '' }),
            suggest_connections: () => [],
            resolve_link_target: () => null,
            ensure_today_journal: () => {
              if (failEnsure) throw new Error('journal create failed');
              return pages[0];
            },
            create_page: () => ({ path: String(args?.path ?? 'x'), slug: 'x', title: String(args?.path ?? 'x'), block_count: 0, modified_at: now }),
            save_blocks: () => undefined,
            save_page: () => undefined,
            delete_page: () => undefined,
            build_markdown: () => '# M',
            get_settings: () => ({ vault_path: '/mock/vault', theme: { dark_mode: true, primary_color: '#f97316', secondary_color: '#6b7280', font_size: 16 }, ai: { provider: 'ollama', endpoint: null, api_key: null, api_key_from_env: false, model: '', models: [], rag_enabled: false, rag_chunk_count: 3 }, graph: { show_connected: true, show_orphaned: true, show_tags: true, charge_strength: -4, link_distance: 40, alpha_decay: 0.15, velocity_decay: 0.4, link_curvature: 0.15, node_cap: 0 }, sync: { mode: 'manual', remote_url: null, branch: 'main', auto_commit_interval_secs: 300, auto_sync_interval_secs: 1800, ssh_key_path: null, commit_template: '' }, research: { searxng_endpoint: '', max_results: 3, max_depth: 2 }, stt: { endpoint: '', api_key: null, model: 'whisper-1', diarize_model: '', language: null, diarize: true, auto_summarize: true, auto_identify: true } }),
            save_settings: () => undefined,
            get_sync_status: () => ({ status: 'clean', branch: 'main', ahead: 0, behind: 0, conflicts: [], last_sync_time: now, last_sync_success: true, pending_commits: 0 }),
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
    { seed, failEnsure: !!opts.failEnsure },
  );
}

test.describe('QA parity — Journal', () => {
  for (const [label, viewport, ua] of [
    ['mobile-small', MOBILE_SMALL, MOBILE_UA],
    ['mobile-large', MOBILE_LARGE, MOBILE_UA],
    ['desktop', DESKTOP, undefined],
  ] as const) {
    test.describe(`viewport: ${label}`, () => {
      test.use({ viewport, userAgent: ua });

      test('navigation: prev/next arrows + calendar affordances present', async ({ page }) => {
        await mockJournalApp(page);
        await page.goto('/journal');
        await expect(page.locator('.blocknote-editor-container').first()).toBeVisible({ timeout: 20000 });

        // Prev/Next day arrows exist on both variants.
        await expect(page.getByLabel('Previous day')).toBeVisible();
        await expect(page.getByLabel('Next day')).toBeVisible();

        // Calendar affordance: mobile uses "Open calendar" IconButton; desktop
        // uses a clickable date header opening an anchored popover.
        if (label === 'desktop') {
          await expect(page.getByLabel('Calendar').or(page.getByLabel('Open calendar'))).toBeVisible();
        } else {
          await expect(page.getByLabel('Open calendar')).toBeVisible();
        }
      });

      test('navigation: prev-day arrow creates + navigates to the previous day', async ({ page }) => {
        await mockJournalApp(page);
        await page.goto('/journal');
        await expect(page.locator('.blocknote-editor-container').first()).toBeVisible({ timeout: 20000 });
        await page.getByLabel('Previous day').click();
        // Navigating to a date adds ?date=YYYY-MM-DD to the URL.
        await page.waitForTimeout(700);
        expect(page.url()).toMatch(/date=\d{4}-\d{2}-\d{2}/);
      });

      test('persistence: navigating away and back still renders the editor', async ({ page }) => {
        await mockJournalApp(page);
        await page.goto('/journal');
        await expect(page.locator('.blocknote-editor-container').first()).toBeVisible({ timeout: 20000 });

        // Go to yesterday via the calendar-less Prev arrow, then back via Next
        // arrow — the editor must render in both states (create-on-demand).
        await page.getByLabel('Previous day').click();
        await page.waitForTimeout(700);
        await expect(page.locator('.blocknote-editor-container').first()).toBeVisible({ timeout: 20000 });

        await page.getByLabel('Next day').click();
        await page.waitForTimeout(700);
        await expect(page.locator('.blocknote-editor-container').first()).toBeVisible({ timeout: 20000 });
      });

      test('error state: ensure_today_journal failure surfaces Retry + Repair', async ({ page }) => {
        await mockJournalApp(page, { failEnsure: true });
        await page.goto('/journal');
        // The panel surfaces an error Alert with Retry + Repair database.
        await expect(page.getByText(/Error|failed/i).first()).toBeVisible({ timeout: 20000 });
        await expect(page.getByRole('button', { name: 'Retry' })).toBeVisible();
        await expect(page.getByRole('button', { name: /Repair database/i })).toBeVisible();
      });
    });
  }
});
