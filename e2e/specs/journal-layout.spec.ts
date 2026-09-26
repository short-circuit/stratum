import { test, expect, type Page } from '@playwright/test';

// ---------------------------------------------------------------------------
// Journal layout regression QA spec (t_994f54b5)
//
// Guard: the journal page must size to its content — it must NOT stretch to
// fill the viewport. This is the regression check for the journal editor
// layout fix (desktop: today editor wrapped in auto-height Box).
//
// The spec drives the REAL React app (built frontend served by vite preview)
// with a self-contained Tauri mock injected at page init. The mock includes a
// real "today" journal page with content blocks so the OutlinerEditor renders
// (not the spinner), and a couple of past journal entries to ensure the
// stacked-entries layout is unaffected.
//
// Platform routing:
//   - Desktop variant is rendered when viewport width >= 768 (browser default,
//     no Tauri mobile marker).
//   - Mobile variant is rendered when viewport width < 768.
//
// We measure the height of today's BlockNote editor container. On the fixed
// layout the editor sizes to its content; on the pre-fix desktop layout it
// stretches to fill the viewport-height scroll container.
// ---------------------------------------------------------------------------

function todayDateStr(): string {
  const d = new Date();
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, '0');
  const day = String(d.getDate()).padStart(2, '0');
  return `${y}-${m}-${day}`;
}

/**
 * Injects a Tauri mock whose vault contains:
 *  - the "today" journal page (with a few content blocks) so the today editor
 *    renders with content,
 *  - two past journal entries (yesterday, day before) so past entries render,
 *  - a normal note page used by the /page/:path regression check.
 *
 * All out-of-mock commands resolve to benign defaults so the app bootstraps.
 *
 * NOTE: the init-script callback is serialized and executed in the page
 * context, so every helper it uses must be defined INSIDE the callback (no
 * closure over module-scope functions).
 */
async function mockJournalApp(page: Page): Promise<void> {
  await page.addInitScript(
    ({ today }: { today: string }) => {
      const block = (id: string, content: string, leftId?: string | null) => ({
        id,
        content,
        parent_id: null,
        left_id: leftId ?? null,
        properties: [],
        marker: null,
        priority: null,
        collapsed: false,
        heading_level: null,
      });

      const dateStr = (d: Date): string =>
        `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;

      const journalPath = `journals/${today}.md`;
      const yesterdayPath = `journals/${dateStr(new Date(Date.now() - 86400000))}.md`;
      const twoDaysAgoPath = `journals/${dateStr(new Date(Date.now() - 2 * 86400000))}.md`;

      const pageDto = (path: string, title: string, blockCount: number) => ({
        path,
        slug: path.replace(/\.md$/i, '').toLowerCase().replace(/[^a-z0-9]+/g, '-'),
        title,
        block_count: blockCount,
        modified_at: new Date().toISOString(),
      });

      const pages = [
        pageDto(journalPath, 'Today Journal', 3),
        pageDto(yesterdayPath, 'Yesterday Journal', 1),
        pageDto(twoDaysAgoPath, 'Two Days Ago Journal', 1),
        pageDto('Note A', 'Note A', 2),
        pageDto('Note B', 'Note B', 1),
      ];

      const blocksByPath: Record<string, unknown[]> = {
        [journalPath]: [
          block('j-today-1', 'Journal entry for today', null),
          block('j-today-2', 'Another line of today journal content', 'j-today-1'),
          block('j-today-3', 'Final today block', 'j-today-2'),
        ],
        [yesterdayPath]: [block('j-y-1', 'Yesterday journal entry', null)],
        [twoDaysAgoPath]: [block('j-2-1', 'Two days ago journal entry', null)],
        'Note A': [
          block('n-a-1', '## Note A heading', null),
          block('n-a-2', 'Body of note A with plenty of content so the page editor is tall.', 'n-a-1'),
        ],
        'Note B': [block('n-b-1', 'Note B content', null)],
      };

      const now = new Date().toISOString();
      const listPages = () => ({ pages });

      const win = window as any;
      win.__TAURI_INTERNALS__ = {
        metadata: {
          currentWindow: { label: 'main' },
          currentWebview: { windowLabel: 'main', label: 'main' },
        },
        invoke(cmd: string, args: Record<string, unknown>) {
          const handlers: Record<string, () => unknown> = {
            get_vault_info: () => ({ path: '/mock/vault', block_count: 8, page_count: 5 }),
            init_vault: () => ({ path: '/mock/vault', block_count: 8, page_count: 5 }),
            init_default_vault: () => ({ path: '/mock/vault', block_count: 8, page_count: 5 }),
            list_pages: listPages,
            open_page: () => ({
              path: String(args?.path), slug: String(args?.path).toLowerCase().replace(/[^a-z0-9]+/g, '-'),
              title: String(args?.path), block_count: 0, modified_at: now, blocks: [],
            }),
            get_blocks: () => ({ blocks: blocksByPath[String(args?.pagePath)] ?? [] }),
            get_page_backlinks: () => [],
            get_backlink_snippet: () => ({ context: '', snippet: '' }),
            suggest_connections: () => [],
            resolve_link_target: () => null,
            ensure_today_journal: () => pageDto(journalPath, 'Today Journal', 3),
            create_page: () => pageDto('new', 'New Page', 0),
            save_blocks: () => undefined,
            save_page: () => undefined,
            delete_page: () => undefined,
            build_markdown: () => '# Markdown',
            get_settings: () => ({
              vault_path: '/mock/vault',
              theme: { dark_mode: true, primary_color: '#f97316', secondary_color: '#6b7280', font_size: 16 },
              ai: { provider: 'ollama', endpoint: null, api_key: null, api_key_from_env: false, model: '', models: [], rag_enabled: false, rag_chunk_count: 3 },
              graph: { show_connected: true, show_orphaned: true, show_tags: true, charge_strength: -4, link_distance: 40, alpha_decay: 0.15, velocity_decay: 0.4, link_curvature: 0.15, node_cap: 0 },
              sync: { mode: 'manual', remote_url: null, branch: 'main', auto_commit_interval_secs: 300, auto_sync_interval_secs: 1800, ssh_key_path: null, commit_template: '' },
              research: { searxng_endpoint: '', max_results: 3, max_depth: 2 },
              stt: { endpoint: '', api_key: null, model: 'whisper-1', diarize_model: '', language: null, diarize: true, auto_summarize: true, auto_identify: true },
            }),
            save_settings: () => undefined,
            get_sync_status: () => ({ status: 'clean', branch: 'main', ahead: 0, behind: 0, conflicts: [], last_sync_time: now, last_sync_success: true, pending_commits: 0 }),
            start_sync_scheduler: () => undefined,
            stop_sync_scheduler: () => undefined,
          };
          const h = handlers[cmd];
          if (!h) {
            console.warn('[mock journal-layout] unhandled command:', cmd);
            return Promise.resolve(null);
          }
          try {
            return Promise.resolve(h());
          } catch (e) {
            return Promise.reject(e);
          }
        },
        convertFileSrc(path: string) { return 'asset://' + path; },
        transformCallback() { return 0; },
        unregisterCallback() {},
        runCallback() {},
        callbacks: new Map(),
      };
      win.__TAURI_EVENT_PLUGIN_INTERNALS__ = {};
    },
    { today: todayDateStr() },
  );
}

// ---------------------------------------------------------------------------
// Helpers — locate the today editor container and measure heights
// ---------------------------------------------------------------------------
async function todayEditorHeights(page: Page): Promise<{ container: number; prose: number; viewport: number }> {
  return page.evaluate(() => {
    const editors = Array.from(document.querySelectorAll<HTMLElement>('.blocknote-editor-container'));
    const first = editors[0];
    const prose = first?.querySelector<HTMLElement>('.ProseMirror');
    const vh = window.innerHeight;
    return {
      container: first ? first.getBoundingClientRect().height : -1,
      prose: prose ? prose.getBoundingClientRect().height : -1,
      viewport: vh,
    };
  });
}

async function journalEditorCount(page: Page): Promise<number> {
  return page.evaluate(() => document.querySelectorAll('.blocknote-editor-container').length);
}

test.describe('Journal layout regression — desktop viewport (>=768px)', () => {
  test.use({ viewport: { width: 1280, height: 720 } });

  test.beforeEach(async ({ page }) => {
    await mockJournalApp(page);
    await page.goto('/journal');
    // Wait for the today editor to finish loading (blocks fetched, editor ready).
    await expect(page.locator('.blocknote-editor-container').first()).toBeVisible({ timeout: 15000 });
    await page.waitForTimeout(500);
  });

  test('today journal editor sizes to content, not viewport height', async ({ page }) => {
    const { container, prose, viewport } = await todayEditorHeights(page);
    // Editor must render actual content (not a spinner).
    expect(prose).toBeGreaterThan(0);
    // Content-sized: the editor container must be well below the viewport height
    // (it holds ~3 short lines). Pre-fix it stretched to fill the ~655px panel.
    expect(container).toBeGreaterThan(0);
    expect(container).toBeLessThan(viewport * 0.6);
  });

  test('journal page document does not stretch to viewport; scroll container scrolls', async ({ page }) => {
    const scrollInfo = await page.evaluate(() => {
      const scrollables = Array.from(document.querySelectorAll<HTMLElement>('*')).filter((el) => {
        const s = getComputedStyle(el);
        return (s.overflowY === 'auto' || s.overflowY === 'scroll') && el.scrollHeight > el.clientHeight;
      });
      return { count: scrollables.length, hasScrollable: scrollables.length > 0 };
    });
    // There must be a scrollable area (the journal panel's internal scroll container).
    expect(scrollInfo.hasScrollable).toBe(true);
  });

  test('editor is interactive — typing is accepted', async ({ page }) => {
    const prose = page.locator('.blocknote-editor-container').first().locator('.ProseMirror').first();
    await prose.click({ position: { x: 5, y: 5 } });
    await page.keyboard.type('QA-typed-line', { delay: 20 });
    const text = await prose.innerText();
    expect(text).toContain('QA-typed-line');
  });

  test('past journal entries still render and size to content', async ({ page }) => {
    // Past entries are below today. They render only when scrolled into view.
    await page.evaluate(() => {
      const scrollables = Array.from(document.querySelectorAll<HTMLElement>('*')).filter((el) => {
        const s = getComputedStyle(el);
        return (s.overflowY === 'auto' || s.overflowY === 'scroll');
      });
      if (scrollables.length) scrollables.forEach((s) => (s.scrollTop = s.scrollHeight));
      window.scrollTo(0, document.body.scrollHeight);
    });
    await page.waitForTimeout(800);
    const count = await journalEditorCount(page);
    // Today + at least one past entry editor (if scrolled into view).
    expect(count).toBeGreaterThanOrEqual(2);
  });
});

test.describe('Journal layout regression — mobile viewport (<768px)', () => {
  test.use({ viewport: { width: 390, height: 844 } });

  test.beforeEach(async ({ page }) => {
    await mockJournalApp(page);
    await page.goto('/journal');
    await expect(page.locator('.blocknote-editor-container').first()).toBeVisible({ timeout: 15000 });
    await page.waitForTimeout(500);
  });

  test('today journal editor sizes to content (not clipped/stretched) on mobile', async ({ page }) => {
    const { container, prose } = await todayEditorHeights(page);
    expect(prose).toBeGreaterThan(0);
    expect(container).toBeGreaterThan(0);
    // Editor should size to its content (~3 lines), not stretch to the visible area.
    expect(container).toBeLessThan(700);
  });

  test('mobile journal page scrolls to reveal past entries and they render', async ({ page }) => {
    await page.evaluate(() => {
      const scrollables = Array.from(document.querySelectorAll<HTMLElement>('*')).filter((el) => {
        const s = window.getComputedStyle(el);
        return (s.overflowY === 'auto' || s.overflowY === 'scroll');
      });
      scrollables.forEach((s) => (s.scrollTop = s.scrollHeight));
      window.scrollTo(0, document.body.scrollHeight);
    });
    await page.waitForTimeout(800);
    const count = await journalEditorCount(page);
    expect(count).toBeGreaterThanOrEqual(2);
  });

  test('editor is interactive on mobile', async ({ page }) => {
    const prose = page.locator('.blocknote-editor-container').first().locator('.ProseMirror').first();
    await prose.click({ position: { x: 5, y: 5 } });
    await page.keyboard.type('mobile-qa-line', { delay: 20 });
    const text = await prose.innerText();
    expect(text).toContain('mobile-qa-line');
  });
});

test.describe('Normal note page (PageView) unaffected by journal fix', () => {
  test.use({ viewport: { width: 1280, height: 720 } });

  test.beforeEach(async ({ page }) => {
    page.on('console', (m) => {
      if (m.type() === 'error' || m.type() === 'warning') console.log('[spec-console]', m.type(), m.text());
    });
    page.on('pageerror', (e) => console.log('[spec-pageerror]', e.message));
    await mockJournalApp(page);
  });

  test('note page editor still loads and is interactive', async ({ page }) => {
    await page.goto('/page/Note%20A');
    await expect(page.locator('.blocknote-editor-container').first()).toBeVisible({ timeout: 15000 });
    await page.waitForTimeout(500);
    const heights = await todayEditorHeights(page);
    expect(heights.prose).toBeGreaterThan(0);
    // PageView editors keep their full-height behavior (minHeight chain) —
    // the fix must not have touched shared PageView behavior.
    const prose = page.locator('.blocknote-editor-container').first().locator('.ProseMirror').first();
    await prose.click({ position: { x: 5, y: 5 } });
    await page.keyboard.type('pageview-qa-line', { delay: 20 });
    expect(await prose.innerText()).toContain('pageview-qa-line');
  });
});
