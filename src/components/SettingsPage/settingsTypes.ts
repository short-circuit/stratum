//! Settings page shared types.
//! Extracted from SettingsPage.shared.tsx during the E6 sizing-gate refactor so
//! the hook module stays small and types can be reused across sub-components.

export type SettingsTab = 'vault' | 'theme' | 'ai' | 'research' | 'developer' | 'sync';

export interface SettingsData {
  vault_path?: string;
  ai?: {
    provider: string;
    endpoint: string | null;
    api_key: string | null;
    api_key_from_env: boolean;
    model: string;
    models: { name: string; capabilities: string[] }[];
    rag_enabled: boolean;
    rag_chunk_count: number;
    embedding_dimensions: number;
    use_llm_gateway_and_auth: boolean;
  };
  research?: {
    searxng_endpoint: string;
    max_results: number;
    max_depth: number;
  };
  theme?: {
    dark_mode: boolean;
    primary_color: string;
    secondary_color: string;
    font_size: number;
  };
  sync?: {
    mode: string;
    remote_url: string | null;
    branch: string;
    auto_commit_interval_secs: number;
    auto_sync_interval_secs: number;
    ssh_key_path: string | null;
    commit_template: string;
  };
  [key: string]: unknown;
}
