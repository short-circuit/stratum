import { invoke } from '@tauri-apps/api/core';
import { getPlatform } from './platform';
import type {
  VaultInfo,
  PageDto,
  PageListDto,
  BlockDto,
  BlockListDto,
  SearchResultsDto,
  BacklinkItem,
  ConnectionSuggestion,
  QueryResultDto,
  SyncStatusDto,
  SyncSettings,
  CommitLogEntry,
  GraphDataDto,
  ComponentDto,
  OrphanDto,
  GraphPanelDataDto,
  AiAction,
  AiTransformResult,
  ResearchResult,
  GraphSettings,
  AutocompleteItem,
  KanbanBlockDto,
  KanbanDataDto,
  ReindexResult,
} from './types';

export async function getVaultInfo(): Promise<VaultInfo> {
  return invoke('get_vault_info');
}

export async function setVaultPath(path: string): Promise<void> {
  return invoke('set_vault_path', { path });
}

export async function pickVaultDirectory(): Promise<VaultInfo> {
  if (getPlatform().isMobile) {
    try {
      return await invoke('pick_android_directory');
    } catch {
      return await invoke('init_default_vault');
    }
  }
  const { open } = await import('@tauri-apps/plugin-dialog');
  const selected = await open({ directory: true, multiple: false });
  if (!selected) throw new Error('No directory selected');
  return invoke('init_vault', { path: selected });
}

export async function listPages(): Promise<PageListDto> {
  return invoke('list_pages');
}

export async function openPage(path: string): Promise<PageDto> {
  return invoke('open_page', { path });
}

export async function savePage(path: string, content: string): Promise<void> {
  return invoke('save_page', { path, content });
}

export async function createPage(path: string, title?: string): Promise<PageDto> {
  return invoke('create_page', { path, title });
}

export async function ensureTodayJournal(): Promise<PageDto> {
  return invoke('ensure_today_journal');
}

export async function deletePage(path: string): Promise<void> {
  return invoke('delete_page', { path });
}

export async function buildMarkdown(
  blocks: BlockDto[],
  title?: string,
): Promise<string> {
  return invoke('build_markdown', { blocks, title });
}

export async function saveBlocks(
  pagePath: string,
  blocks: BlockDto[],
  title?: string,
): Promise<void> {
  return invoke('save_blocks', { pagePath, blocks, title });
}

export async function getBlocks(pagePath: string): Promise<BlockListDto> {
  return invoke('get_blocks', { pagePath });
}

export async function updateBlock(pagePath: string, block: BlockDto): Promise<void> {
  return invoke('update_block', { pagePath, block });
}

export async function deleteBlock(blockId: string): Promise<void> {
  return invoke('delete_block', { blockId });
}

export async function insertBlock(
  pagePath: string,
  content: string,
  parentId?: string | null,
  afterId?: string | null,
): Promise<BlockDto> {
  return invoke('insert_block', { pagePath, content, parentId, afterId });
}

export async function searchBlocks(
  query: string,
  limit?: number,
): Promise<SearchResultsDto> {
  return invoke('search_blocks', { query, limit });
}

export async function searchByTag(tag: string): Promise<SearchResultsDto> {
  return invoke('search_by_tag', { tag });
}

export async function toggleBlockMarker(pagePath: string, blockId: string): Promise<string | null> {
  return invoke('toggle_block_marker', { pagePath, blockId });
}

export async function clearBlockMarker(pagePath: string, blockId: string): Promise<void> {
  return invoke('clear_block_marker', { pagePath, blockId });
}

export async function rebuildSearchIndex(): Promise<string> {
  return invoke('rebuild_search_index');
}

export async function getPageBacklinks(pagePath: string): Promise<BacklinkItem[]> {
  return invoke('get_page_backlinks', { pagePath });
}

export async function autocomplete(
  query: string,
  kind: string,
): Promise<AutocompleteItem[]> {
  return invoke('autocomplete', { query, kind });
}

export async function listTemplates(): Promise<{
  name: string; path: string; content: string; description?: string;
}[]> {
  return invoke('list_templates');
}

export async function saveTemplate(name: string, content: string): Promise<void> {
  return invoke('save_template', { name, content });
}

export async function applyTemplate(
  templateName: string,
  targetPage: string,
  variables: [string, string][],
): Promise<string> {
  return invoke('apply_template', { templateName, targetPage, variables });
}

export async function exportHtml(outputDir: string): Promise<{
  output_dir: string; pages_exported: number; assets_copied: number;
}> {
  return invoke('export_html', { outputDir });
}

export async function exportJson(outputDir: string): Promise<{
  output_dir: string; pages_exported: number; assets_copied: number;
}> {
  return invoke('export_json', { outputDir });
}

export async function generateFlashcards(): Promise<{
  id: string; front: string; back: string; page_path: string; ease_factor: number; interval_days: number; repetitions: number; next_review: string;
}[]> {
  return invoke('generate_flashcards');
}

export async function reviewCard(cardId: string, quality: number): Promise<{
  id: string; front: string; back: string; page_path: string; ease_factor: number; interval_days: number; repetitions: number; next_review: string;
}> {
  return invoke('review_card', { cardId, quality });
}

export async function listWhiteboards(): Promise<{
  name: string; path: string; content: string;
}[]> {
  return invoke('list_whiteboards');
}

export async function saveWhiteboard(name: string, content: string): Promise<void> {
  return invoke('save_whiteboard', { name, content });
}

export async function loadWhiteboard(name: string): Promise<string> {
  return invoke('load_whiteboard', { name });
}

export async function renameWhiteboard(oldName: string, newName: string): Promise<void> {
  return invoke('rename_whiteboard', { oldName, newName });
}

export async function deleteWhiteboard(name: string): Promise<void> {
  return invoke('delete_whiteboard', { name });
}

export async function saveLibrary(content: string): Promise<void> {
  return invoke('save_library', { content });
}

export async function loadLibrary(): Promise<string> {
  return invoke('load_library');
}

export async function loadExtraLibraries(): Promise<string> {
  return invoke('load_extra_libraries');
}

export async function getSettings(): Promise<{
  vault_path: string;
  theme: { dark_mode: boolean; primary_color: string; secondary_color: string; font_size: number };
  ai: {
    provider: string;
    endpoint: string | null;
    api_key: string | null;
    model: string;
    models: { name: string; capabilities: string[] }[];
    rag_enabled: boolean;
    rag_chunk_count: number;
  };
  graph: GraphSettings;
  sync: SyncSettings;
}> {
  return invoke('get_settings');
}

export async function saveSettings(settings: {
  vault_path: string;
  theme: { dark_mode: boolean; primary_color: string; secondary_color: string; font_size: number };
  ai: {
    provider: string;
    endpoint: string | null;
    api_key: string | null;
    model: string;
    models: { name: string; capabilities: string[] }[];
    rag_enabled: boolean;
    rag_chunk_count: number;
  };
  graph: GraphSettings;
  sync: SyncSettings;
}): Promise<void> {
  return invoke('save_settings', { settings });
}

export async function saveGraphSettings(graph: GraphSettings): Promise<void> {
  return invoke('save_graph_settings', { graph });
}

export async function fetchModels(): Promise<string[]> {
  return invoke('fetch_models');
}

export async function runQuery(datalog: string): Promise<QueryResultDto> {
  return invoke('run_query', { datalog });
}

export async function getSyncStatus(): Promise<SyncStatusDto> {
  return invoke('get_sync_status');
}

export async function syncVault(): Promise<SyncStatusDto> {
  return invoke('sync_vault');
}

export async function syncVaultWithPassphrase(passphrase: string): Promise<SyncStatusDto> {
  return invoke('sync_vault_with_passphrase', { passphrase });
}

export async function startSyncScheduler(): Promise<void> {
  return invoke('start_sync_scheduler');
}

export async function stopSyncScheduler(): Promise<void> {
  return invoke('stop_sync_scheduler');
}

export async function getCommitLog(): Promise<CommitLogEntry[]> {
  return invoke('get_commit_log');
}

export async function resolveConflictFile(path: string): Promise<void> {
  return invoke('resolve_conflict_file', { path });
}

export async function abortMerge(): Promise<void> {
  return invoke('abort_merge');
}

// --- AI ---

export async function aiTransformBlock(
  text: string,
  action: AiAction,
  pagePath?: string,
): Promise<AiTransformResult> {
  return invoke('ai_transform_block', { text, action, pagePath });
}

export async function aiResearch(query: string): Promise<ResearchResult> {
  return invoke('ai_research', { query });
}

export async function aiInterlinkNotes(
  text: string,
  pagePath?: string,
): Promise<AiTransformResult> {
  return invoke('ai_interlink_notes', { text, pagePath });
}

export async function generateMermaid(prompt: string): Promise<string> {
  const result = await invoke<AiTransformResult>('generate_mermaid', { prompt });
  return result.content;
}

// --- Connections ---

export async function suggestConnections(pagePath: string): Promise<ConnectionSuggestion[]> {
  return invoke('suggest_connections', { pagePath });
}

// --- Graph ---

export async function getGraphData(): Promise<GraphDataDto> {
  return invoke('get_graph_data');
}

export async function getConnectedComponents(): Promise<ComponentDto[]> {
  return invoke('get_connected_components');
}

export async function getOrphanedNotes(): Promise<OrphanDto[]> {
  return invoke('get_orphaned_notes');
}

export async function getGraphPanelData(): Promise<GraphPanelDataDto> {
  return invoke('get_graph_panel_data');
}

// --- Link resolution ---

export async function resolveLinkTarget(target: string): Promise<{
  page_path: string | null;
  slug: string | null;
  title: string | null;
}> {
  return invoke('resolve_link_target', { target });
}

export async function getBacklinkContext(
  targetPage: string,
  currentPage: string,
): Promise<{
  block_id: string;
  content: string;
  page_title: string | null;
} | null> {
  return invoke('get_backlink_context', { targetPage, currentPage });
}

// --- Reindex ---

export async function reindexVault(): Promise<ReindexResult> {
  return invoke('reindex_vault');
}

export async function reindexPage(path: string): Promise<ReindexResult> {
  return invoke('reindex_page', { path });
}

// --- Normalize ---

export async function normalizeFile(path: string): Promise<void> {
  return invoke('normalize_file', { path });
}

export async function normalizeAllFiles(): Promise<number> {
  return invoke('normalize_all_files');
}

// --- Kanban ---

export async function getKanbanBlocks(): Promise<KanbanDataDto> {
  return invoke('get_kanban_blocks');
}

export async function createKanbanBlock(
  content: string,
  marker: string,
): Promise<KanbanBlockDto> {
  return invoke('create_kanban_block', { content, marker });
}
