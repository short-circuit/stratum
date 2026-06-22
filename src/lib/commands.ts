import { invoke } from '@tauri-apps/api/core';
import type {
  VaultInfo,
  PageDto,
  PageListDto,
  BlockDto,
  BlockListDto,
  SearchResultsDto,
  BacklinkItem,
  QueryResultDto,
  SyncStatusDto,
} from './types';

export async function getVaultInfo(): Promise<VaultInfo> {
  return invoke('get_vault_info');
}

export async function setVaultPath(path: string): Promise<void> {
  return invoke('set_vault_path', { path });
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

export async function rebuildSearchIndex(): Promise<string> {
  return invoke('rebuild_search_index');
}

export async function getPageBacklinks(pagePath: string): Promise<BacklinkItem[]> {
  return invoke('get_page_backlinks', { pagePath });
}

export async function autocomplete(
  query: string,
  kind: string,
): Promise<{ text: string; kind: string; detail?: string }[]> {
  return invoke('autocomplete', { query, kind });
}

export async function getBlockProperties(
  blockId: string,
): Promise<{ block_id: string; properties: [string, string][]; marker?: string; priority?: string }> {
  return invoke('get_block_properties', { blockId });
}

export async function setBlockProperty(
  blockId: string,
  key: string,
  value: string,
): Promise<void> {
  return invoke('set_block_property', { blockId, key, value });
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

export async function getSettings(): Promise<{
  vault_path: string;
  ai: {
    provider: string;
    endpoint: string | null;
    api_key: string | null;
    model: string;
    models: { name: string; capabilities: string[] }[];
    rag_enabled: boolean;
    rag_chunk_count: number;
  };
}> {
  return invoke('get_settings');
}

export async function saveSettings(settings: {
  vault_path: string;
  ai: {
    provider: string;
    endpoint: string | null;
    api_key: string | null;
    model: string;
    models: { name: string; capabilities: string[] }[];
    rag_enabled: boolean;
    rag_chunk_count: number;
  };
}): Promise<void> {
  return invoke('save_settings', { settings });
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
