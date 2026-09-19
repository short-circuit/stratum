import type { Page } from '@playwright/test';
import type {
  PageDto,
  BlockDto,
  BlockListDto,
  SearchResultDto,
  BacklinkItem,
  ConnectionSuggestion,
  SyncStatusDto,
  CommitLogEntry,
  GraphPanelDataDto,
  GraphNodeDto,
  GraphEdgeDto,
  AutocompleteItem,
  KanbanBlockDto,
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
};

/** Transcript produced by the mocked `dictation_transcribe` command. */
export const MOCK_DICTATION_RESULT = {
  markdown: '## Voice memo\n\nAlice: hello there\nBob: how are you',
  inserted_block_ids: ['blk-dict-1', 'blk-dict-2'],
  turns: [
    { speaker: 'speaker_0', start: 0, end: 2, text: 'hello there' },
    { speaker: 'speaker_1', start: 2, end: 4, text: 'how are you' },
  ],
  speaker_names: {},
  num_speakers: 2,
  diarized: true,
  summary: 'Greetings exchanged.',
  related: ['Projects'],
  tags: ['voice-memo'],
  clip_rel_path: 'recordings/memo-2026-07-28.flac',
  duration_secs: 42,
};

/**
 * Seed data for the mocked `plugins_*` commands. Mirrors the real
 * `PluginInfoDto` shape (id, name, version, status, enabled, permissions,
 * hooks, author, description, error) per docs/advanced/plugins.md §9.
 */
export const MOCK_PLUGINS_DTO = [
  {
    id: 'dev-dashboard',
    name: 'Developer Dashboard',
    version: '0.1.0',
    status: 'ready',
    enabled: true,
    permissions: ['file:read', 'network'],
    hooks: ['onSave'],
    author: 'stratum-team',
    description: 'Collects development telemetry and posts it to a local endpoint.',
    error: null,
  },
  {
    id: 'daily-summary',
    name: 'Daily Summary',
    version: '0.2.1',
    status: 'disabled',
    enabled: false,
    permissions: ['file:read', 'file:write'],
    hooks: ['onOpen', 'onSave'],
    author: 'acme-lab',
    description: 'Summarizes the day\u2019s notes into a single digest page.',
    error: null,
  },
  {
    id: 'broken-example',
    name: 'Broken Example',
    version: '0.0.4',
    status: 'error',
    enabled: true,
    permissions: ['file:read'],
    hooks: [],
    author: '',
    description: '',
    error: "runtime_error: import 'pkm.note_write' not found (plugin compiled with a different ABI)",
  },
];


// ---------------------------------------------------------------------------
// Config object for tests to control mock behavior
// ---------------------------------------------------------------------------
export interface MockConfig {
  /** When false, `get_vault_info` throws, showing vault picker */
  hasVault: boolean;
  /** Error to throw for specific commands (key = command name) */
  commandErrors: Record<string, string>;
  /** When true, `plugins_list` returns an empty list (for the empty state) */
  emptyPlugins?: boolean;
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

      // In-page settings store so save_settings -> get_settings round-trips.
      // Persisted via localStorage so it survives navigation/reload within the
      // test's browser context (mirrors real backend disk persistence).
      const persisted = window.localStorage.getItem('mock_settings');
      let mockSettings = persisted ? JSON.parse(persisted) : ${JSON.stringify(MOCK_SETTINGS)};

      // In-page plugin store so plugins_list -> enable/disable/install/uninstall
      // round-trip within the test (mirrors the real registry + config persistence).
      const MOCK_PLUGIN_ROWS = ${JSON.stringify(MOCK_PLUGINS_DTO)};
      const persistedPlugins = window.localStorage.getItem('mock_plugins');
      let mockPlugins = persistedPlugins ? JSON.parse(persistedPlugins) : MOCK_PLUGIN_ROWS.map(function(p){ return Object.assign({}, p); });
      function persistPlugins(list) {
        window.localStorage.setItem('mock_plugins', JSON.stringify(list));
      }
      function pluginById(id) {
        for (var i = 0; i < mockPlugins.length; i++) if (mockPlugins[i].id === id) return mockPlugins[i];
        return null;
      }
      function syncPluginStatus(list) {
        for (var i = 0; i < list.length; i++) {
          var p = list[i];
          if (!p.enabled && p.status !== 'error') p.status = 'disabled';
          if (p.enabled && p.status === 'disabled') p.status = 'ready';
        }
        return list;
      }
      function upsertMockPlugin(plugin) {
        var idx = -1;
        for (var i = 0; i < mockPlugins.length; i++) if (mockPlugins[i].id === plugin.id) { idx = i; break; }
        if (idx === -1) mockPlugins.push(plugin); else mockPlugins[idx] = plugin;
        mockPlugins = syncPluginStatus(mockPlugins);
        persistPlugins(mockPlugins);
        return plugin;
      }

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
        get_settings: () => mockSettings,
        save_settings: (args) => {
          if (args && args.settings) {
            mockSettings = args.settings;
            window.localStorage.setItem('mock_settings', JSON.stringify(args.settings));
          }
        },
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
        // --- Dictation (voice memos) ---
        dictation_start: () => ({
          recording_path: '/mock/vault/recordings/memo-2026-07-28.flac',
          device_name: 'Mock Microphone',
          sample_rate: 48000,
        }),
        dictation_stop: () => ({
          recording_path: '/mock/vault/recordings/memo-2026-07-28.flac',
          duration_secs: 42,
        }),
        dictation_cancel: () => {},
        dictation_transcribe: () => (${JSON.stringify(MOCK_DICTATION_RESULT)}),
        speaker_list: () => [],
        speaker_assign: (args) => ({
          name: args && args.name,
          enrolled: !!(args && args.enroll),
          markdown: '## Voice memo\\n\\nAlice: hello there\\nBob: how are you',
          speaker_names: args && args.speakerId ? { [args.speakerId]: args.name } : {},
          inserted_block_ids: ['blk-dict-1', 'blk-dict-2'],
        }),
        speaker_delete: () => {},
        stt_test_connection: () => ({
          ok: true,
          models: ['whisper-1'],
          latency_ms: 320,
          error: null,
        }),
        plugins_list: () => {
          if (config.emptyPlugins) return { plugins: [] };
          return { plugins: mockPlugins.map(function (p) { return Object.assign({}, p); }) };
        },
        plugins_status: (args) => {
          const p = pluginById(args && args.id);
          if (!p) throw new Error("plugin_not_found: no plugin with id '" + (args && args.id) + "'");
          return Object.assign({}, p);
        },
        plugins_enable: (args) => {
          const p = pluginById(args && args.id);
          if (!p) throw new Error("plugin_not_found: no plugin with id '" + (args && args.id) + "'");
          p.enabled = true;
          mockPlugins = syncPluginStatus(mockPlugins);
          persistPlugins(mockPlugins);
          return Object.assign({}, upsertMockPlugin(p));
        },
        plugins_disable: (args) => {
          const p = pluginById(args && args.id);
          if (!p) throw new Error("plugin_not_found: no plugin with id '" + (args && args.id) + "'");
          p.enabled = false;
          mockPlugins = syncPluginStatus(mockPlugins);
          persistPlugins(mockPlugins);
          return Object.assign({}, upsertMockPlugin(p));
        },
        plugins_reload: (args) => {
          const p = pluginById(args && args.id);
          if (!p) throw new Error("plugin_not_found: no plugin with id '" + (args && args.id) + "'");
          p.status = p.enabled ? 'ready' : 'disabled';
          p.error = null;
          return Object.assign({}, upsertMockPlugin(p));
        },
        plugins_install: (args) => {
          const path = args && args.path;
          const base = String(path || '').split(/[\\\\/]/).pop() || 'plugin';
          const id = ('installed-' + base).replace(/\\.wasm$/i, '');
          const existing = pluginById(id);
          const plugin = existing || {
            id: id,
            name: id,
            version: '0.1.0',
            status: 'disabled',
            enabled: false,
            permissions: [],
            hooks: [],
            author: 'mock-author',
            description: 'Installed from ' + path,
            error: null,
          };
          plugin.status = 'disabled';
          plugin.enabled = false;
          upsertMockPlugin(plugin);
          return Object.assign({}, plugin);
        },
        plugins_uninstall: (args) => {
          const id = args && args.id;
          mockPlugins = mockPlugins.filter(function (p) { return p.id !== id; });
          persistPlugins(mockPlugins);
          return { plugins: mockPlugins.map(function (p) { return Object.assign({}, p); }) };
        },
        plugin_note_read: (args) => ({
          path: args && args.path,
          content: '# ' + (args && args.path) + '\\n\\n(mocked plugin note read)',
          mtime: '${NOW}',
        }),
        plugin_http_request: () => ({
          status: 200,
          headers: [['content-type', 'application/json']],
          body: JSON.stringify({ mocked: true }),
        }),
      };

      // Event plugin handlers (used by onCloseRequested, listen, etc.)
      handlers['plugin:event|listen'] = () => ({});
      handlers['plugin:event|unlisten'] = () => ({});
      handlers['plugin:window|create'] = () => ({});
      handlers['plugin:window|close'] = () => ({});
      // Native file dialog (used by @tauri-apps/plugin-dialog open() for
      // plugin install). Returns a fixed path so the install flow is testable.
      handlers['plugin:dialog|open'] = () => '/mock/vault/.pkm/installed-extras.wasm';

      // Tauri internals mock (matching @tauri-apps/api v2 expectations)
      window.__TAURI_INTERNALS__ = {
        metadata: {
          currentWindow: { label: 'main' },
          currentWebview: { windowLabel: 'main', label: 'main' },
        },
        invoke: function(cmd, args, options) {
          // Tests can force a command failure via config.commandErrors.
          if (config.commandErrors && config.commandErrors[cmd]) {
            return Promise.reject(new Error(config.commandErrors[cmd]));
          }
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
