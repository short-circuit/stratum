// TypeScript types matching the Rust DTOs from src-tauri.

export interface VaultInfo {
  path: string;
  block_count: number;
  page_count: number;
}

export interface PageDto {
  path: string;
  slug: string;
  title: string | null;
  block_count: number;
  modified_at: string;
}

export interface PageListDto {
  pages: PageDto[];
}

export interface BlockDto {
  id: string;
  content: string;
  parent_id: string | null;
  left_id: string | null;
  properties: [string, string][];
  marker: string | null;
  priority: string | null;
  collapsed: boolean;
  heading_level: number | null;
}

export interface BlockListDto {
  blocks: BlockDto[];
}

export interface SearchResultDto {
  block_id: string;
  content: string;
  page_path: string;
  snippet: string;
  score: number;
}

export interface SearchResultsDto {
  results: SearchResultDto[];
}

export interface QueryResultDto {
  columns: string[];
  rows: string[][];
}

export interface BacklinkItem {
  source_id: string;
  source_page: string;
  context: string;
  is_linked: boolean;
}

export interface SyncStatusDto {
  status: string;
  branch: string | null;
  ahead: number;
  behind: number;
  conflicts: string[];
}

// --- Graph types ---

export interface GraphNodeDto {
  id: string;
  title: string;
  path: string;
  tags: string[];
  degree: number;
}

export interface GraphEdgeDto {
  source: string;
  target: string;
  label: string | null;
}

export interface GraphDataDto {
  nodes: GraphNodeDto[];
  edges: GraphEdgeDto[];
  node_count: number;
  edge_count: number;
  vault_path: string;
}

export interface ComponentDto {
  nodes: GraphNodeDto[];
  size: number;
}

export interface OrphanDto {
  slug: string;
  title: string;
  path: string;
}

// --- Graph settings ---

export interface GraphSettings {
  show_connected: boolean;
  show_orphaned: boolean;
  show_tags: boolean;
  charge_strength: number;
  link_distance: number;
  alpha_decay: number;
  velocity_decay: number;
}

// --- Link resolution ---

export interface LinkTargetDto {
  page_path: string | null;
  slug: string | null;
  title: string | null;
}

export interface BacklinkContextDto {
  block_id: string;
  content: string;
  page_title: string | null;
}

// --- Connection suggestions ---

export interface ConnectionSuggestion {
  title: string;
  page_path: string;
  score: number;
  snippet: string;
}

// --- AI types ---

export type AiAction = 'rewrite' | 'format' | 'structure' | 'summarize' | 'connect';

export interface AiTransformResult {
  content: string;
}

export interface ResearchResult {
  findings: string;
  sources: ResearchSource[];
}

export interface ResearchSource {
  title: string;
  url: string;
  snippet: string;
}
