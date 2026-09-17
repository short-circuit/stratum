import { invoke } from '@tauri-apps/api/core';
import type {
  PageDto,
  PageListDto,
  BlockDto,
  BlockListDto,
  ReindexResult,
} from '../types';

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

export async function toggleBlockMarker(pagePath: string, blockId: string): Promise<string | null> {
  return invoke('toggle_block_marker', { pagePath, blockId });
}

export async function clearBlockMarker(pagePath: string, blockId: string): Promise<void> {
  return invoke('clear_block_marker', { pagePath, blockId });
}

export async function reindexVault(): Promise<ReindexResult> {
  return invoke('reindex_vault');
}

export async function reindexPage(path: string): Promise<ReindexResult> {
  return invoke('reindex_page', { path });
}

export async function repairDbFromDisk(): Promise<ReindexResult> {
  return invoke('repair_db_from_disk');
}

export async function normalizeFile(path: string): Promise<void> {
  return invoke('normalize_file', { path });
}

export async function normalizeAllFiles(): Promise<number> {
  return invoke('normalize_all_files');
}
