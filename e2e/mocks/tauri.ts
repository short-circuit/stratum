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
  ReminderDto,
  SavedQuery,
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
    { id: 'blk-4', content: '![[assets/foo-abc.png]]', parent_id: null, left_id: 'blk-3', properties: [], marker: null, priority: null, collapsed: false, heading_level: null },
  ],
  'Projects': [
    { id: 'blk-10', content: 'Active Projects', parent_id: null, left_id: null, properties: [], marker: null, priority: null, collapsed: false, heading_level: 1 },
    { id: 'blk-11', content: 'Build the [[PKM Guide]] with best practices.', parent_id: null, left_id: 'blk-10', properties: [], marker: null, priority: null, collapsed: false, heading_level: null },
    { id: 'blk-12', content: 'TODO: Review [[Getting Started]] documentation', parent_id: null, left_id: 'blk-11', properties: [], marker: 'TODO', priority: null, collapsed: false, heading_level: null },
    { id: 'blk-13', content: 'DOING: Refactor search index', parent_id: null, left_id: 'blk-12', properties: [], marker: 'DOING', priority: 'A', collapsed: false, heading_level: null },
  ],
  'Meeting Notes': [
    { id: 'blk-20', content: 'Sprint Review', parent_id: null, left_id: null, properties: [], marker: null, priority: null, collapsed: false, heading_level: 2 },
    { id: 'blk-21', content: 'Discussed [[Projects]] roadmap for Q3.', parent_id: null, left_id: 'blk-20', properties: [], marker: null, priority: null, collapsed: false, heading_level: null },
  ],
  'Getting Started': [
    { id: 'blk-30', content: 'Getting Started Guide', parent_id: null, left_id: null, properties: [], marker: null, priority: null, collapsed: false, heading_level: 1 },
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

export const MOCK_BACKUP_STATUS = {
  enabled: true,
  mode: 'folder',
  last_run: NOW,
  last_success: true,
  last_error: null,
  files_copied: 3,
  files_deleted: 0,
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
    use_3d: false,
  },
  sync: {
    mode: 'manual',
    remote_url: null,
    branch: 'main',
    auto_commit_interval_secs: 300,
    auto_sync_interval_secs: 1800,
    ssh_key_path: null,
    commit_template: 'stratum({datetime}): {editedfiles} edited, {newfiles} added, {deletedfiles} deleted',
    backup: {
      enabled: false,
      mode: 'folder',
      interval_minutes: 60,
      exclude: ['.git', '.pkm/blocks.db', '.pkm/search', '.pkm/backup_state.json', '.pkm/sync_state.json', '.DS_Store'],
      webdav_url: null,
      webdav_username: null,
      webdav_password: null,
      folder_path: null,
    },
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
  tray: {
    enabled: true,
    close_to_tray: true,
    quick_capture_shortcut: 'CmdOrCtrl+Shift+Space',
    capture_target: 'journal',
    inbox_path: 'Inbox.md',
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

/**
 * Seed data for the mocked `plugins_install_url` command — mirrors the real
 * `PluginCatalogEntry` shape from `plugins/index.json` (id, name, version,
 * description, author, permissions, sha256, download_url). The install-url
 * mock looks up the entry by id to synthesize the installed plugin DTO.
 */
export const MOCK_CATALOG_PLUGINS = [
  {
    schema_version: 1,
    id: 'com.example.hello-world',
    name: 'Hello World',
    version: '0.1.0',
    description: 'Logs note metadata on save and demonstrates the pkm host API',
    author: 'Stratum Community',
    entry: 'plugin.wasm',
    permissions: ['file:read'],
    hooks: { onSave: true },
    repo: 'https://github.com/short-circuit/stratum-plugins',
    source: 'examples/hello-world',
    sha256: '78d3d3d9784e089a7827a54b3c1f0cea2c3b75a14685edc16309cebad17c2d57',
    download_url:
      'https://raw.githubusercontent.com/short-circuit/stratum-plugins/main/plugins/com.example.hello-world/plugin.wasm',
  },
];

/**
 * Seed data for the mocked saved-query commands. Mirrors the real `SavedQuery`
 * shape (name, query, updated_at) per docs/guide/datalog-queries.md. Tests can
 * override the store by pre-seeding `mock_saved_queries` in localStorage on a
 * per-context basis, or the seed below is used as the persisted baseline.
 */
export const MOCK_SAVED_QUERIES_DTO: SavedQuery[] = [
  {
    name: 'All TODO',
    query: '{:query [:find ?b :where [?b :block/marker "TODO"]]}',
    updated_at: '2026-09-22T20:00:00Z',
  },
  {
    name: 'Blocked',
    query: '{:query [:find ?b :where [?b :block/marker "WAITING"]]}',
    updated_at: '2026-09-22T21:00:00Z',
  },
];

/**
 * Seed data for the mocked reminder commands (E10.F4 block reminders).
 * Mirrors the real `ReminderDto` shape (docs/guide/reminders.md, planned task
 * t_5774176b README screenshots). Due values are relative to a fixed anchor so
 * the panel always shows Overdue / Today / Upcoming sections deterministically.
 */
export function buildMockReminders(now: Date): ReminderDto[] {
  const iso = (d: Date) => d.toISOString();
  const daysFromNow = (n: number) => {
    const d = new Date(now);
    d.setUTCDate(d.getUTCDate() + n);
    return iso(d);
  };
  const at = (offsetMin: number) => iso(new Date(now.getTime() + offsetMin * 60_000));
  return [
    {
      block_id: 'blk-r-1',
      page_path: 'tasks/launch-checklist.md',
      content: 'Ship release notes to the community',
      due: daysFromNow(-2),
      marker: 'TODO',
      priority: 'A',
      page_title: 'Launch Checklist',
    },
    {
      block_id: 'blk-r-2',
      page_path: 'projects/stratum-1.0.md',
      content: 'Follow up with Ana about the migration',
      due: at(20),
      marker: null,
      priority: null,
      page_title: 'Stratum 1.0',
    },
    {
      block_id: 'blk-r-3',
      page_path: 'tasks/renew-domain.md',
      content: 'Renew the domain registration',
      due: daysFromNow(5),
      marker: 'DOING',
      priority: 'B',
      page_title: 'Renew Domain',
    },
  ];
}


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
  /**
   * Graph settings overrides (merged over `MOCK_SETTINGS.graph`) applied when
   * the mock settings store is seeded. Lets tests set `{ use_3d: true }` etc.
   * without fighting the localStorage init-script ordering.
   */
  graphSettings?: Record<string, unknown>;
  /**
   * Number of consecutive `dictation_transcribe` calls that should throw
   * (for the voice-capture error/fallback path). After the given count the
   * command succeeds normally.
   */
  dictationTranscribeFailures?: number;
  /** When true, `get_onboarding_state` returns true so the first-run wizard renders. */
  onboardingPending?: boolean;
  /** Overrides for the mocked `get_tray_status` / settings tray seed. */
  trayStatus?: { enabled?: boolean; close_to_tray?: boolean };
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

      window.__invokedCommands = [];
      const __cbSeq = { n: 0 };
      const __cbs = {};
      const __listeners = {};

      // In-page settings store so save_settings -> get_settings round-trips.
      // Persisted via localStorage so it survives navigation/reload within the
      // test's browser context (mirrors real backend disk persistence).
      const persisted = window.localStorage.getItem('mock_settings');
      let mockSettings = persisted ? JSON.parse(persisted) : ${JSON.stringify(MOCK_SETTINGS)};
      // Apply graph settings overrides from the test config (e.g. use_3d:true)
      // so tests can deterministically seed the graph preference.
      if (config.graphSettings) {
        mockSettings = Object.assign({}, mockSettings, {
          graph: Object.assign({}, mockSettings.graph, config.graphSettings),
        });
        window.localStorage.setItem('mock_settings', JSON.stringify(mockSettings));
      }

      // In-page plugin store so plugins_list -> enable/disable/install/uninstall
      // round-trip within the test (mirrors the real registry + config persistence).
      const MOCK_PLUGIN_ROWS = ${JSON.stringify(MOCK_PLUGINS_DTO)};
      // Catalog seed for plugins_install_url (E10.F7 Plugin Directory mock).
      const MOCK_CATALOG_PLUGIN_ROWS = ${JSON.stringify(MOCK_CATALOG_PLUGINS)};
      const persistedPlugins = window.localStorage.getItem('mock_plugins');
      let mockPlugins = persistedPlugins ? JSON.parse(persistedPlugins) : MOCK_PLUGIN_ROWS.map(function(p){ return Object.assign({}, p); });
      function persistPlugins(list) {
        window.localStorage.setItem('mock_plugins', JSON.stringify(list));
      }
      function pluginById(id) {
        for (var i = 0; i < mockPlugins.length; i++) if (mockPlugins[i].id === id) return mockPlugins[i];
        return null;
      }

      // In-page saved-queries store so list_saved_queries -> save/rename/delete
      // round-trips within the test. Persisted via localStorage so it survives
      // navigation/reload within the test's browser context, mirroring the real
      // backend's on-disk persistence of .pkm/saved_queries.json.
      const MOCK_SAVED_QUERY_ROWS = ${JSON.stringify(MOCK_SAVED_QUERIES_DTO)};
      const persistedQueries = window.localStorage.getItem('mock_saved_queries');
      let mockSavedQueries = persistedQueries ? JSON.parse(persistedQueries) : MOCK_SAVED_QUERY_ROWS.map(function(q){ return Object.assign({}, q); });
      function persistSavedQueries() {
        window.localStorage.setItem('mock_saved_queries', JSON.stringify(mockSavedQueries));
      }
      function savedQueryByName(name) {
        for (var i = 0; i < mockSavedQueries.length; i++) if (mockSavedQueries[i].name === name) return mockSavedQueries[i];
        return null;
      }
      function touchTimestamp() {
        return new Date().toISOString();
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
      const MOCK_BLOCKS_DATA = ${JSON.stringify(MOCK_BLOCKS)};
      // Mutable block store so voice-capture inserts are reflected by get_blocks
      // (the editor remounts after a memo is inserted and re-fetches blocks).
      let blockStore = {};
      for (var bk in MOCK_BLOCKS_DATA) blockStore[bk] = MOCK_BLOCKS_DATA[bk].map(function (b) { return Object.assign({}, b); });
      // Seed today's journal with a starter entry so the journal panel and any
      // screenshot capture show meaningful content (the real backend creates
      // today's journal on ensure_today_journal, and it is initially empty).
      (function () {
        var d = new Date();
        var path = 'journals/' + d.getFullYear() + '-' + String(d.getMonth() + 1).padStart(2, '0') + '-' + String(d.getDate()).padStart(2, '0') + '.md';
        if (!blockStore[path]) {
          blockStore[path] = [
            { id: 'blk-j-1', content: 'Daily Notes', parent_id: null, left_id: null, properties: [], marker: null, priority: null, collapsed: false, heading_level: 2 },
            { id: 'blk-j-2', content: 'Reviewed the [[PKM Guide]] and planned today\\'s focus.', parent_id: null, left_id: 'blk-j-1', properties: [], marker: null, priority: null, collapsed: false, heading_level: null },
            { id: 'blk-j-3', content: 'TODO: Ship the release notes', parent_id: null, left_id: 'blk-j-2', properties: [], marker: 'TODO', priority: 'A', collapsed: false, heading_level: null },
          ];
        }
      })();
      function appendBlocks(pagePath, blocks) {
        if (!blockStore[pagePath]) blockStore[pagePath] = [];
        var anchor = blockStore[pagePath].length ? blockStore[pagePath][blockStore[pagePath].length - 1].id : null;
        var ids = [];
        for (var i = 0; i < blocks.length; i++) {
          blocks[i].left_id = anchor;
          blockStore[pagePath].push(blocks[i]);
          ids.push(blocks[i].id);
          anchor = blocks[i].id;
        }
        return ids;
      }
      // Voice-capture call counters, readable from the test via page.evaluate.
      window.__mockVoice = { transcribeCalls: 0, clipCalls: 0, transcribeFailures: (config.dictationTranscribeFailures || 0) };
      const memoBlocks = [
        { id: 'blk-memo-1', content: 'Voice Memo', parent_id: null, left_id: null, properties: [], marker: null, priority: null, collapsed: false, heading_level: 2 },
        { id: 'blk-memo-2', content: 'Alice: hello there', parent_id: null, left_id: 'blk-memo-1', properties: [], marker: null, priority: null, collapsed: false, heading_level: null },
        { id: 'blk-memo-3', content: 'Bob: how are you', parent_id: null, left_id: 'blk-memo-2', properties: [], marker: null, priority: null, collapsed: false, heading_level: null },
      ];

      const handlers = {
        get_vault_info: () => {
          if (!config.hasVault) throw new Error('No vault configured');
          return { path: '/mock/vault', block_count: 14, page_count: 5 };
        },
        set_vault_path: () => {},
        init_vault: () => ({ path: '/mock/vault', block_count: 14, page_count: 5 }),
        init_default_vault: () => ({ path: '/mock/vault', block_count: 14, page_count: 5 }),
        pick_android_directory: () => { throw new Error('No directory selected'); },
        list_pages: () => ({
          pages: ${JSON.stringify(MOCK_PAGES)}.map(function (p) {
            return Object.assign({}, p);
          }).concat([{
            path: 'journals/' + (function () {
              var d = new Date();
              var m = String(d.getMonth() + 1).padStart(2, '0');
              var day = String(d.getDate()).padStart(2, '0');
              return d.getFullYear() + '-' + m + '-' + day;
            })() + '.md',
            slug: 'journal-today',
            title: 'Today',
            block_count: 0,
            modified_at: '${NOW}',
          }]),
        }),
        open_page: (args) => {
          const pagePath = args && args.path;
          const p = ${JSON.stringify(MOCK_PAGES)}.find(mp => mp.path === pagePath);
          return {
            ...(p ?? { path: pagePath, slug: (pagePath || '').toLowerCase().replace(/\\s+/g, '-'), title: pagePath, block_count: 0, modified_at: '${NOW}' }),
            blocks: blockStore[pagePath] ?? [],
          };
        },
        save_page: () => {},
        list_reminders: () => (${JSON.stringify(buildMockReminders(new Date()))}),
        snooze_reminder: () => null,
        clear_reminder: () => null,
        get_onboarding_state: () => !!config.onboardingPending,
        mark_onboarding_done: () => null,
        set_onboarding_pending: () => null,
        install_starter_templates: () => [],
        install_starter_sample: () => [],
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
          const blocks = blockStore;
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
        // --- Asset / attachment commands ---
        get_page_asset_refs: () => ([
          { vault_relative_path: 'assets/foo-abc.png', mime: 'image/png', size: 2048, sha256: 'aa' },
          { vault_relative_path: 'assets/doc-123.pdf', mime: 'application/pdf', size: 4096, sha256: 'bb' },
        ]),
        list_asset_refs: () => ([
          { vault_relative_path: 'assets/foo-abc.png', mime: 'image/png', size: 2048, sha256: 'aa' },
          { vault_relative_path: 'assets/doc-123.pdf', mime: 'application/pdf', size: 4096, sha256: 'bb' },
        ]),
        read_asset: () => ({
          data_base64: 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==',
          mime: 'image/png',
          size: 78,
        }),
        import_asset_bytes: (args) => ({
          vault_relative_path: 'assets/' + (args.originalName || 'paste.png'),
          mime: args.mime || 'application/octet-stream',
          size: 100,
          sha256: 'cc',
        }),
        import_asset_path: (args) => ({
          vault_relative_path: 'assets/' + (args.path || 'file.png'),
          mime: 'image/png',
          size: 100,
          sha256: 'dd',
        }),
        delete_asset: () => {},
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
        save_graph_settings: (args) => {
          // Mirror the real Rust command: persist the graph settings so a
          // subsequent get_settings reflects them (the app reads graph
          // settings back on remount via useGraphPanel → api.getSettings).
          if (args && args.graph) {
            mockSettings = Object.assign({}, mockSettings, {
              graph: Object.assign({}, mockSettings.graph, args.graph),
            });
            window.localStorage.setItem('mock_settings', JSON.stringify(mockSettings));
          }
        },
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
        run_backup: () => (${JSON.stringify(MOCK_BACKUP_STATUS)}),
        get_backup_status: () => (${JSON.stringify(MOCK_BACKUP_STATUS)}),
        test_webdav: () => ('ok'),
        ai_transform_block: () => ({ content: 'Transformed content' }),
        ai_research: () => ({ findings: 'Research findings', sources: [] }),
        discover_local_ai: () => ([
          { host: 'http://127.0.0.1:11434', name: 'Ollama', kind: 'ollama',
            models: ['llama3.2:latest'], status: 'available', latency_ms: 12, error: null },
          { host: 'http://127.0.0.1:1234/v1', name: 'LM Studio', kind: 'openai_compatible',
            models: [], status: 'unreachable', latency_ms: 0, error: 'no response' },
        ]),
        ping_search_instance: () => ({ ok: true, latency_ms: 180, error: null }),
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
        dictation_transcribe: (args) => {
          window.__mockVoice.transcribeCalls += 1;
          if (window.__mockVoice.transcribeFailures > 0) {
            window.__mockVoice.transcribeFailures -= 1;
            throw new Error('mock transient transcription failure');
          }
          const page = args && args.pagePath;
          if (page) appendBlocks(page, memoBlocks.map(function (b) { return Object.assign({}, b); }));
          return ${JSON.stringify(MOCK_DICTATION_RESULT)};
        },
        dictation_insert_clip_ref: (args) => {
          window.__mockVoice.clipCalls += 1;
          const page = args && args.pagePath;
          const clipBlock = [
            { id: 'blk-clip-1', content: '🎙️ Voice Memo', parent_id: null, left_id: null, properties: [], marker: null, priority: null, collapsed: false, heading_level: 2 },
            { id: 'blk-clip-2', content: '> _transcription pending — retry_', parent_id: null, left_id: 'blk-clip-1', properties: [], marker: null, priority: null, collapsed: false, heading_level: null },
          ];
          return page ? appendBlocks(page, clipBlock) : [];
        },
        mic_permission_status: () => ({ granted: true, restricted: false }),
        request_mic_permission: () => ({}),
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
        plugins_install_url: (args) => {
          const id = args && args.id ? String(args.id) : 'installed-url-plugin';
          const sha = args && args.sha256 ? String(args.sha256) : null;
          const entry = MOCK_CATALOG_PLUGIN_ROWS.find(function (p) { return p.id === id; });
          const existing = pluginById(id);
          const plugin = existing || {
            id: id,
            name: entry ? entry.name : id,
            version: entry ? entry.version : '0.1.0',
            status: 'disabled',
            enabled: false,
            permissions: entry ? entry.permissions : [],
            hooks: [],
            author: entry ? entry.author : 'mock-author',
            description: (entry ? entry.description : 'Installed from catalog') + (sha ? ' (verified)' : ''),
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
        list_saved_queries: () => mockSavedQueries.map(function(q){ return Object.assign({}, q); }),
        save_saved_query: (args) => {
          const name = String(args && args.name !== undefined ? args.name : '').trim();
          if (!name) throw new Error('name must not be empty');
          const existing = savedQueryByName(name);
          const updated = { name: name, query: String(args && args.query !== undefined ? args.query : ''), updated_at: touchTimestamp() };
          if (existing) {
            existing.query = updated.query;
            existing.updated_at = updated.updated_at;
          } else {
            mockSavedQueries.push(updated);
          }
          persistSavedQueries();
          return Object.assign({}, existing || updated);
        },
        rename_saved_query: (args) => {
          const oldName = args && args.oldName;
          const newName = String(args && args.newName !== undefined ? args.newName : '').trim();
          const q = savedQueryByName(oldName);
          if (!q) throw new Error('saved query not found: ' + oldName);
          if (!newName) throw new Error('name must not be empty');
          const clash = mockSavedQueries.find(function(other){ return other !== q && other.name === newName; });
          if (clash) throw new Error('saved query already exists: ' + newName);
          q.name = newName;
          q.updated_at = touchTimestamp();
          persistSavedQueries();
          return Object.assign({}, q);
        },
        delete_saved_query: (args) => {
          const name = args && args.name;
          const before = mockSavedQueries.length;
          mockSavedQueries = mockSavedQueries.filter(function(q){ return q.name !== name; });
          if (mockSavedQueries.length === before) throw new Error('saved query not found: ' + name);
          persistSavedQueries();
          return null;
        },
      };

      // Event plugin handlers (used by onCloseRequested, listen, etc.)
      handlers['plugin:event|listen'] = (args) => {
        const ev = args && args.event;
        if (ev) {
          __listeners[ev] = __listeners[ev] || [];
          __listeners[ev].push(args.handler);
        }
        return {};
      };
      handlers['plugin:event|unlisten'] = () => ({});
      handlers['plugin:window|create'] = () => ({});
      handlers['plugin:window|close'] = () => ({});
      handlers['plugin:window|destroy'] = () => ({});
      handlers['plugin:window|hide'] = () => ({});
      // get_tray_status is invoked on app close; default to close_to_tray per
      // MOCK_SETTINGS, overridable via config.trayStatus for per-test control.
      handlers['get_tray_status'] = () => {
        const t = Object.assign({}, mockSettings.tray, config.trayStatus || {});
        return {
          enabled: Boolean(t.enabled),
          close_to_tray: Boolean(t.enabled && t.close_to_tray),
          shortcut: t.quick_capture_shortcut,
          capture_target: t.capture_target,
          inbox_path: t.inbox_path,
        };
      };
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
          window.__invokedCommands.push(cmd);
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
        transformCallback: function(fn) { __cbSeq.n += 1; __cbs[__cbSeq.n] = fn; return __cbSeq.n; },
        unregisterCallback: function(id) { delete __cbs[id]; },
        runCallback: function(id, data) { const fn = __cbs[id]; if (fn) fn(data); },
        callbacks: new Map(),
      };

      // Event plugin internals (needed by @tauri-apps/api/event).
      // @tauri-apps/api >= 2.12 calls unregisterListener(event, eventId) from
      // _unlisten; without it, listener teardown throws and crashes the app.
      window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
        unregisterListener: function(event, eventId) {
          // No-op: the mock's plugin:event|listen registry never needs removal.
          // Keep a no-op so callers that track ids (e.g. plugin:event|emit
          // fan-out) remain harmless even after unlisten.
        },
      };

      // Test helper: dispatch a Tauri event (e.g. 'tauri://close-requested') to
      // listeners registered via onCloseRequested/listen.
      window.__emitTauriEvent = function(name, payload) {
        (__listeners[name] || []).forEach(function(id) {
          const fn = __cbs[id];
          if (fn) fn({ event: name, id: id, payload: payload === undefined ? null : payload });
        });
      };
    })()`,
  });
}
