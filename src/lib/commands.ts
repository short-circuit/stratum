import { invoke } from '@tauri-apps/api/core';
import type {
  VaultInfo,
  PageDto,
  PageListDto,
  BlockDto,
  BlockListDto,
  SearchResultsDto,
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

export async function getBacklinks(blockId: string): Promise<SearchResultsDto> {
  return invoke('get_backlinks', { blockId });
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
