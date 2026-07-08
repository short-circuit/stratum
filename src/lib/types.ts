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
  last_sync_time: string | null;
  last_sync_success: boolean | null;
  pending_commits: number;
}

export interface SyncSettings {
  mode: string;
  remote_url: string | null;
  branch: string;
  auto_commit_interval_secs: number;
  auto_sync_interval_secs: number;
  ssh_key_path: string | null;
  commit_template: string;
}

export interface CommitLogEntry {
  hash: string;
  author: string;
  message: string;
  timestamp: string;
}

// --- Graph types ---

export interface GraphNodeDto {
  id: string;
  title: string;
  path: string;
  tags: string[];
  degree: number;
  /** Pre-computed layout position (set by Web Worker for progressive rendering). */
  x?: number;
  y?: number;
  z?: number;
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

export interface GraphPanelDataDto {
  graph: GraphDataDto;
  components: ComponentDto[];
  orphans: OrphanDto[];
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
  link_curvature: number;
  /** Maximum nodes to render before capping. 0 = unlimited. */
  node_cap: number;
}

// --- Connection suggestions ---

export interface ConnectionSuggestion {
  title: string;
  page_path: string;
  score: number;
  snippet: string;
}

// --- Link types ---

export interface LinkTargetDto {
  page_path: string | null;
  slug: string | null;
  title: string | null;
}

export interface AutocompleteItem {
  text: string;
  kind: string;
  detail?: string;
}

export interface BacklinkContextDto {
  block_id: string;
  content: string;
  page_title: string | null;
}

// --- AI types ---

export type AiAction = 'rewrite' | 'format' | 'structure' | 'summarize' | 'connect' | 'mermaid';

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

export interface KanbanBlockDto {
  id: string;
  content: string;
  parent_id: string | null;
  left_id: string | null;
  properties: [string, string][];
  marker: string | null;
  priority: string | null;
  collapsed: boolean;
  heading_level: number | null;
  page_path: string;
  page_title: string | null;
}

export interface KanbanDataDto {
  blocks: KanbanBlockDto[];
}

export interface ReindexResult {
  processed: number;
  succeeded: number;
  failed: number;
  errors: string[];
}
