import type { Page } from '@playwright/test';
import type {
  VaultInfo,
  PageDto,
  PageListDto,
  BlockDto,
  BlockListDto,
  SearchResultDto,
  SearchResultsDto,
  BacklinkItem,
  ConnectionSuggestion,
  SyncStatusDto,
  CommitLogEntry,
  GraphPanelDataDto,
  GraphNodeDto,
  GraphEdgeDto,
  AutocompleteItem,
  KanbanBlockDto,
  KanbanDataDto,
} from '../../src/lib/types';

// ---------------------------------------------------------------------------
// Mock data
// ---------------------------------------------------------------------------

const NOW = '2026-07-28T12:00:00Z';

export const MOCK_PAGES: PageDto[] = [
  { path: 'Welcome', slug: 'welcome', title: 'Welcome', block_count: 3, modified_at: NOW },
  { path: 'Projects', slug: 'projects', title: 'Projects', block_count: 4, modified_at: NOW },
  { path: 'Meeting Notes', slug: 'meeting-notes', title: 'Meeting Notes', block_count: 2, modified_at: NOW },
  { path: 'Getting Started', slug: 'getting-started', title: 'Getting Started', block_count: 3, modified_at: NOW },
  { path: 'PKM Guide', slug: 'pkm-guide', title: 'PKM Guide', block_count: 2, modified_at: NOW },
];

export const MOCK_BLOCKS: Record<string, BlockDto[]> = {
  'Welcome': [
    { id: 'blk-1', content: 'Welcome to your **knowledge base**!', parent_id: null, left_id: null, properties: [], marker: null, priority: null, collapsed: false, heading_level: 1 },
    { id: 'blk-2', content: 'Check out [[Projects]] for ongoing work.', parent_id: null, left_id: 'blk-1', properties: [], marker: null, priority: null, collapsed: false, heading_level: null },
    { id: 'blk-3', content: 'See [[Meeting Notes]] for recent discussions.', parent_id: null, left_id: 'blk-2', properties: [], marker: null, priority: null, collapsed: false, heading_level: null },
  ],
  'Projects': [
    { id: 'blk-10', content: '## Active Projects', parent_id: null, left_id: null, properties: [], marker: null, priority: null, collapsed: false, heading_level: 1 },
    { id: 'blk-11', content: 'Build the [[PKM Guide]] with best practices.', parent_id: null, left_id: 'blk-10', properties: [], marker: null, priority: null, collapsed: false, heading_level: null },
    { id: 'blk-12', content: 'TODO: Review [[Getting Started]] documentation', parent_id: null, left_id: 'blk-11', properties: [], marker: 'TODO', priority: null, collapsed: false, heading_level: null },
    { id: 'blk-13', content: 'DOING: Refactor search index', parent_id: null, left_id: 'blk-12', properties: [], marker: 'DOING', priority: 'A', collapsed: false, heading_level: null },
  ],
  'Meeting Notes': [
    { id: 'blk-20', content: '## Sprint Review', parent_id: null, left_id: null, properties: [], marker: null, priority: null, collapsed: false, heading_level: 2 },
    { id: 'blk-21', content: 'Discussed [[Projects]] roadmap for Q3.', parent_id: null, left_id: 'blk-20', properties: [], marker: null, priority: null, collapsed: false, heading_level: null },
  ],
  'Getting Started': [
    { id: 'blk-30', content: '# Getting Started Guide', parent_id: null, left_id: null, properties: [], marker: null, priority: null, collapsed: false, heading_level: 1 },
    { id: 'blk-31', content: 'Wiki-links let you connect ideas: [[Welcome]]', parent_id: null, left_id: 'blk-30', properties: [], marker: null, priority: null, collapsed: false, heading_level: null },
    { id: 'blk-32', content: 'Use #tags to categorize your notes.', parent_id: null, left_id: 'blk-31', properties: [], marker: null, priority: null, collapsed: false, heading_level: null },
  ],
  'PKM Guide': [
    { id: 'blk-40', content: 'A personal knowledge management system.', parent_id: null, left_id: null, properties: [], marker: null, priority: null, collapsed: false, heading_level: null },
    { id: 'blk-41', content: 'Related: [[Getting Started]] for first steps.', parent_id: null, left_id: 'blk-40', properties: [], marker: null, priority: null, collapsed: false, heading_level: null },
  ],
};

export function getPageDto(pagePath: string): PageDto {
  const p = MOCK_PAGES.find(mp => mp.path === pagePath);
  return p ?? { path: pagePath, slug: pagePath.toLowerCase().replace(/\s+/g, '-'), title: pagePath, block_count: 0, modified_at: NOW };
}

export function getBlocks(pagePath: string): BlockListDto {
  return { blocks: MOCK_BLOCKS[pagePath] ?? [] };
}

export function getMockPageWithBlocks(pagePath: string): PageDto & { blocks: BlockDto[] } {
  const page = getPageDto(pagePath);
  const { blocks } = getBlocks(pagePath);
  return { ...page, blocks };
}

// Graph data
const GRAPH_NODES: GraphNodeDto[] = [
  { id: 'Welcome', title: 'Welcome', path: 'Welcome', tags: ['pkm'], degree: 2 },
  { id: 'Projects', title: 'Projects', path: 'Projects', tags: ['work'], degree: 3 },
  { id: 'Meeting Notes', title: 'Meeting Notes', path: 'Meeting Notes', tags: ['meeting'], degree: 1 },
  { id: 'Getting Started', title: 'Getting Started', path: 'Getting Started', tags: ['docs'], degree: 2 },
  { id: 'PKM Guide', title: 'PKM Guide', path: 'PKM Guide', tags: ['pkm', 'guide'], degree: 2 },
];

const GRAPH_EDGES: GraphEdgeDto[] = [
  { source: 'Welcome', target: 'Projects', label: null },
  { source: 'Welcome', target: 'Meeting Notes', label: null },
  { source: 'Projects', target: 'PKM Guide', label: null },
  { source: 'Projects', target: 'Getting Started', label: null },
  { source: 'Getting Started', target: 'Welcome', label: null },
  { source: 'PKM Guide', target: 'Getting Started', label: null },
];

export const MOCK_GRAPH_PANEL_DATA: GraphPanelDataDto = {
  graph: {
    nodes: GRAPH_NODES,
    edges: GRAPH_EDGES,
    node_count: GRAPH_NODES.length,
    edge_count: GRAPH_EDGES.length,
    vault_path: '/mock/vault',
  },
  components: [
    { nodes: GRAPH_NODES, size: GRAPH_NODES.length },
  ],
  orphans: [],
};

// Search results
export const MOCK_SEARCH_RESULTS: SearchResultDto[] = [
  { block_id: 'blk-11', content: 'Build the PKM Guide with best practices.', page_path: 'Projects', snippet: 'Build the <b>PKM Guide</b> with best practices.', score: 0.95 },
  { block_id: 'blk-10', content: 'Active Projects', page_path: 'Projects', snippet: '<b>Active Projects</b>', score: 0.85 },
  { block_id: 'blk-21', content: 'Discussed Projects roadmap for Q3.', page_path: 'Meeting Notes', snippet: 'Discussed <b>Projects</b> roadmap for Q3.', score: 0.72 },
];

export const MOCK_BACKLINKS: BacklinkItem[] = [
  { source_id: 'blk-2', source_page: 'Welcome', context: 'Check out [[Projects]] for ongoing work.', is_linked: true },
  { source_id: 'blk-11', source_page: 'Projects', context: 'Build the [[PKM Guide]] with best practices.', is_linked: true },
];

export const MOCK_CONNECTIONS: ConnectionSuggestion[] = [
  { title: 'PKM Guide', page_path: 'PKM Guide', score: 0.88, snippet: 'Personal knowledge management system' },
  { title: 'Getting Started', page_path: 'Getting Started', score: 0.76, snippet: 'Getting started guide' },
];

export const MOCK_AUTOCOMPLETE: AutocompleteItem[] = [
  { text: 'Welcome', kind: 'page', detail: 'welcome' },
  { text: 'Projects', kind: 'page', detail: 'projects' },
  { text: 'Meeting Notes', kind: 'page', detail: 'meeting-notes' },
  { text: 'Getting Started', kind: 'page', detail: 'getting-started' },
];

export const MOCK_TEMPLATES = [
  { name: 'daily', path: 'templates/daily.md', content: '# {{date}}\n\n', description: 'Daily journal template' },
  { name: 'meeting', path: 'templates/meeting.md', content: '# {{title}}\n\n## Attendees\n\n## Notes\n\n## Action Items\n', description: 'Meeting notes template' },
];

export const MOCK_FLASHCARDS = [
  { id: 'card-1', front: 'What is a wiki-link?', back: 'A [[WikiLink]] connects two notes.', page_path: 'Getting Started', ease_factor: 2.5, interval_days: 1, repetitions: 0, next_review: NOW },
  { id: 'card-2', front: 'What is PKM?', back: 'Personal Knowledge Management.', page_path: 'PKM Guide', ease_factor: 2.5, interval_days: 1, repetitions: 0, next_review: NOW },
];

export const MOCK_WHITEBOARDS = [
  { name: 'Architecture', path: 'whiteboards/architecture.excalidraw', content: '{"elements":[],"appState":{}}' },
  { name: 'Mind Map', path: 'whiteboards/mindmap.excalidraw', content: '{"elements":[],"appState":{}}' },
];

export const MOCK_KANBAN_BLOCKS: KanbanBlockDto[] = [
  { id: 'kan-1', content: 'Setup CI pipeline', parent_id: null, left_id: null, properties: [], marker: 'TODO', priority: null, collapsed: false, heading_level: null, page_path: 'Projects', page_title: 'Projects' },
  { id: 'kan-2', content: 'Write documentation', parent_id: null, left_id: 'kan-1', properties: [], marker: 'DOING', priority: 'A', collapsed: false, heading_level: null, page_path: 'Projects', page_title: 'Projects' },
  { id: 'kan-3', content: 'Deploy to production', parent_id: null, left_id: 'kan-2', properties: [], marker: 'DONE', priority: null, collapsed: false, heading_level: null, page_path: 'Projects', page_title: 'Projects' },
];

export const MOCK_COMMITS: CommitLogEntry[] = [
  { hash: 'a1b2c3d', author: 'dev', message: 'feat: add graph view', timestamp: '2026-07-27T10:00:00Z' },
  { hash: 'e4f5g6h', author: 'dev', message: 'fix: search indexing', timestamp: '2026-07-26T14:00:00Z' },
];

export const MOCK_SYNC_STATUS: SyncStatusDto = {
  status: 'clean',
  branch: 'main',
  ahead: 0,
  behind: 0,
  conflicts: [],
  last_sync_time: NOW,
  last_sync_success: true,
  pending_commits: 0,
};

export const MOCK_SETTINGS = {
  vault_path: '/mock/vault',
  theme: { dark_mode: true, primary_color: '#f97316', secondary_color: '#6b7280', font_size: 16 },
  ai: {
    provider: 'ollama',
    endpoint: null,
    api_key: null,
    api_key_from_env: false,
    model: '',
    models: [],
    rag_enabled: false,
    rag_chunk_count: 3,
  },
  graph: {
    show_connected: true,
    show_orphaned: true,
    show_tags: true,
    charge_strength: -4,
    link_distance: 40,
    alpha_decay: 0.15,
    velocity_decay: 0.4,
    link_curvature: 0.15,
    node_cap: 0,
  },
  sync: {
    mode: 'manual',
    remote_url: null,
    branch: 'main',
    auto_commit_interval_secs: 300,
    auto_sync_interval_secs: 1800,
    ssh_key_path: null,
    commit_template: 'stratum({datetime}): {editedfiles} edited, {newfiles} added, {deletedfiles} deleted',
  },
  research: {
    searxng_endpoint: 'http://localhost:8888',
    max_results: 3,
    max_depth: 2,
  },
};

// ---------------------------------------------------------------------------
// Config object for tests to control mock behavior
// ---------------------------------------------------------------------------
export interface MockConfig {
  /** When false, `get_vault_info` throws, showing vault picker */
  hasVault: boolean;
  /** Error to throw for specific commands (key = command name) */
  commandErrors: Record<string, string>;
}

/** Default: vault configured, no errors */
export const DEFAULT_MOCK_CONFIG: MockConfig = {
  hasVault: true,
  commandErrors: {},
};

// ---------------------------------------------------------------------------
// Core mock setup
// ---------------------------------------------------------------------------

/**
 * Registers `page.addInitScript()` that sets up `window.__TAURI_INTERNALS__`
 * with a mocked `invoke()` that returns realistic data for all commands.
 *
 * Call this in `test.beforeEach()` before navigating.
 */
export async function mockTauriInvoke(page: Page, config: MockConfig = DEFAULT_MOCK_CONFIG): Promise<void> {
  await page.addInitScript({
    content: `(function() {
      const config = ${JSON.stringify(config)};

      // Command handler registry
      const handlers = {
        get_vault_info: () => {
          if (!config.hasVault) throw new Error('No vault configured');
          return { path: '/mock/vault', block_count: 14, page_count: 5 };
        },
        set_vault_path: () => {},
        init_vault: () => ({ path: '/mock/vault', block_count: 14, page_count: 5 }),
        init_default_vault: () => ({ path: '/mock/vault', block_count: 14, page_count: 5 }),
        pick_android_directory: () => { throw new Error('No directory selected'); },
        list_pages: () => (${JSON.stringify({ pages: MOCK_PAGES })}),
        open_page: (args) => {
          const pagePath = args && args.path;
          const p = ${JSON.stringify(MOCK_PAGES)}.find(mp => mp.path === pagePath);
          const blocks = ${JSON.stringify(MOCK_BLOCKS)};
          return {
            ...(p ?? { path: pagePath, slug: (pagePath || '').toLowerCase().replace(/\\s+/g, '-'), title: pagePath, block_count: 0, modified_at: '${NOW}' }),
            blocks: blocks[pagePath] ?? [],
          };
        },
        save_page: () => {},
        create_page: (args) => ({
          path: args.path,
          slug: (args.path || '').toLowerCase().replace(/\\s+/g, '-'),
          title: args.title || args.path,
          block_count: 0,
          modified_at: '${NOW}',
        }),
        ensure_today_journal: () => ({
          path: 'journal/2026-07-28',
          slug: 'journal-2026-07-28',
          title: 'Jul 28, 2026',
          block_count: 1,
          modified_at: '${NOW}',
        }),
        delete_page: () => {},
        build_markdown: () => '# Mock markdown',
        save_blocks: () => {},
        get_blocks: (args) => {
          const blocks = ${JSON.stringify(MOCK_BLOCKS)};
          return { blocks: blocks[args && args.pagePath] ?? [] };
        },
        update_block: () => {},
        delete_block: () => {},
        insert_block: (args) => ({
          id: 'blk-new-' + Date.now(),
          content: args.content,
          parent_id: args.parentId || null,
          left_id: args.afterId || null,
          properties: [],
          marker: null,
          priority: null,
          collapsed: false,
          heading_level: null,
        }),
        search_blocks: (args) => {
          const q = (args && args.query || '').toLowerCase();
          const results = ${JSON.stringify(MOCK_SEARCH_RESULTS)};
          if (!q) return { results: [] };
          return { results: results.filter(r => r.content.toLowerCase().includes(q) || r.page_path.toLowerCase().includes(q)) };
        },
        search_by_tag: () => ({ results: ${JSON.stringify(MOCK_SEARCH_RESULTS)} }),
        toggle_block_marker: () => 'DONE',
        clear_block_marker: () => {},
        rebuild_search_index: () => 'Index rebuilt with 14 blocks from 5 pages',
        get_page_backlinks: () => (${JSON.stringify(MOCK_BACKLINKS)}),
        autocomplete: (args) => {
          const q = (args && args.query || '').toLowerCase();
          const items = ${JSON.stringify(MOCK_AUTOCOMPLETE)};
          return items.filter(i => i.text.toLowerCase().includes(q) || (i.detail && i.detail.toLowerCase().includes(q)));
        },
        list_templates: () => (${JSON.stringify(MOCK_TEMPLATES)}),
        save_template: () => {},
        apply_template: () => '# Applied template content',
        export_html: () => ({ output_dir: '/tmp/stratum-export', pages_exported: 5, assets_copied: 2 }),
        export_json: () => ({ output_dir: '/tmp/stratum-export', pages_exported: 5, assets_copied: 2 }),
        generate_flashcards: () => (${JSON.stringify(MOCK_FLASHCARDS)}),
        review_card: (args) => ({
          id: args.cardId,
          front: 'Reviewed card',
          back: 'Card content',
          page_path: '',
          ease_factor: 2.5,
          interval_days: 7,
          repetitions: 1,
          next_review: '${NOW}',
        }),
        list_whiteboards: () => (${JSON.stringify(MOCK_WHITEBOARDS)}),
        save_whiteboard: () => {},
        load_whiteboard: () => '{"elements":[],"appState":{}}',
        rename_whiteboard: () => {},
        delete_whiteboard: () => {},
        save_library: () => {},
        load_library: () => '{"elements":[]}',
        load_extra_libraries: () => '{}',
        get_settings: () => (${JSON.stringify(MOCK_SETTINGS)}),
        save_settings: () => {},
        save_graph_settings: () => {},
        fetch_models: () => [],
        run_query: () => ({ columns: ['col1', 'col2'], rows: [['a', 'b']] }),
        get_sync_status: () => (${JSON.stringify(MOCK_SYNC_STATUS)}),
        sync_vault: () => (${JSON.stringify(MOCK_SYNC_STATUS)}),
        sync_vault_with_passphrase: () => (${JSON.stringify(MOCK_SYNC_STATUS)}),
        start_sync_scheduler: () => {},
        stop_sync_scheduler: () => {},
        get_commit_log: () => (${JSON.stringify(MOCK_COMMITS)}),
        resolve_conflict_file: () => {},
        abort_merge: () => {},
        ai_transform_block: () => ({ content: 'Transformed content' }),
        ai_research: () => ({ findings: 'Research findings', sources: [] }),
        ai_interlink_notes: () => ({ content: 'Interlinked content' }),
        generate_mermaid: () => ({ content: 'graph TD; A-->B;' }),
        suggest_connections: () => (${JSON.stringify(MOCK_CONNECTIONS)}),
        get_graph_data: () => ({
          nodes: ${JSON.stringify(GRAPH_NODES)},
          edges: ${JSON.stringify(GRAPH_EDGES)},
          node_count: ${GRAPH_NODES.length},
          edge_count: ${GRAPH_EDGES.length},
          vault_path: '/mock/vault',
        }),
        get_connected_components: () => ([{ nodes: ${JSON.stringify(GRAPH_NODES)}, size: ${GRAPH_NODES.length} }]),
        get_orphaned_notes: () => [],
        get_graph_panel_data: () => (${JSON.stringify(MOCK_GRAPH_PANEL_DATA)}),
        resolve_link_target: (args) => {
          const target = args && args.target;
          const match = ${JSON.stringify(MOCK_PAGES)}.find(p => p.path === target || p.slug === target);
          return match ? { page_path: match.path, slug: match.slug, title: match.title } : { page_path: null, slug: null, title: null };
        },
        get_backlink_context: () => null,
        reindex_vault: () => ({ processed: 14, succeeded: 14, failed: 0, errors: [] }),
        reindex_page: () => ({ processed: 3, succeeded: 3, failed: 0, errors: [] }),
        normalize_file: () => {},
        normalize_all_files: () => 14,
        get_kanban_blocks: () => ({ blocks: ${JSON.stringify(MOCK_KANBAN_BLOCKS)} }),
        create_kanban_block: (args) => ({
          id: 'kan-new-' + Date.now(),
          content: args.content,
          parent_id: null,
          left_id: null,
          properties: [],
          marker: args.marker || 'TODO',
          priority: null,
          collapsed: false,
          heading_level: null,
          page_path: 'Projects',
          page_title: 'Projects',
        }),
      };

      // Event plugin handlers (used by onCloseRequested, listen, etc.)
      handlers['plugin:event|listen'] = () => ({});
      handlers['plugin:event|unlisten'] = () => ({});
      handlers['plugin:window|create'] = () => ({});
      handlers['plugin:window|close'] = () => ({});

      // Tauri internals mock (matching @tauri-apps/api v2 expectations)
      window.__TAURI_INTERNALS__ = {
        metadata: {
          currentWindow: { label: 'main' },
          currentWebview: { windowLabel: 'main', label: 'main' },
        },
        invoke: function(cmd, args, options) {
          const handler = handlers[cmd];
          if (!handler) {
            console.warn('[mock tauri] unhandled command:', cmd, args);
            return Promise.resolve(null);
          }
          try {
            const result = handler(args);
            return Promise.resolve(result);
          } catch (e) {
            return Promise.reject(e);
          }
        },
        convertFileSrc: function(path) { return 'asset://' + path; },
        transformCallback: function(fn, once) { return 0; },
        unregisterCallback: function(id) {},
        runCallback: function(id, data) {},
        callbacks: new Map(),
      };

      // Event plugin internals (needed by @tauri-apps/api/event)
      window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {};
    })()`,
  });
}
