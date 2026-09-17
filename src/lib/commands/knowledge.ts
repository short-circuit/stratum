import { invoke } from '@tauri-apps/api/core';
import type {
  SearchResultsDto,
  BacklinkItem,
  ConnectionSuggestion,
  QueryResultDto,
  GraphDataDto,
  ComponentDto,
  OrphanDto,
  GraphPanelDataDto,
  AutocompleteItem,
} from '../types';

export async function searchBlocks(
  query: string,
  limit?: number,
): Promise<SearchResultsDto> {
  return invoke('search_blocks', { query, limit });
}

export async function searchByTag(tag: string): Promise<SearchResultsDto> {
  return invoke('search_by_tag', { tag });
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

export async function runQuery(datalog: string): Promise<QueryResultDto> {
  return invoke('run_query', { datalog });
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
