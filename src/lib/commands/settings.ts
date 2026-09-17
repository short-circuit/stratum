import { invoke } from '@tauri-apps/api/core';
import { getPlatform } from '../platform';
import type { VaultInfo, GraphSettings, SyncSettings } from '../types';

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

export async function getSettings(): Promise<{
  vault_path: string;
  theme: { dark_mode: boolean; primary_color: string; secondary_color: string; font_size: number };
  ai: {
    provider: string;
    endpoint: string | null;
    api_key: string | null;
    api_key_from_env: boolean;
    model: string;
    models: { name: string; capabilities: string[] }[];
    rag_enabled: boolean;
    rag_chunk_count: number;
  };
  graph: GraphSettings;
  sync: SyncSettings;
  research?: {
    searxng_endpoint: string;
    max_results: number;
    max_depth: number;
  };
  stt?: {
    endpoint: string;
    api_key: string | null;
    model: string;
    diarize_model: string;
    language: string | null;
    diarize: boolean;
    auto_summarize: boolean;
    auto_identify: boolean;
  };
  tts?: {
    endpoint: string;
    api_key: string | null;
    voice: string;
    format: string;
    speed: number;
  };
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
    api_key_from_env?: boolean;
    model: string;
    models: { name: string; capabilities: string[] }[];
    rag_enabled: boolean;
    rag_chunk_count: number;
  };
  graph: GraphSettings;
  sync: SyncSettings;
  stt?: {
    endpoint: string;
    api_key: string | null;
    model: string;
    diarize_model: string;
    language: string | null;
    diarize: boolean;
    auto_summarize: boolean;
    auto_identify: boolean;
  };
  tts?: {
    endpoint: string;
    api_key: string | null;
    voice: string;
    format: string;
    speed: number;
  };
}): Promise<void> {
  return invoke('save_settings', { settings });
}

export async function saveGraphSettings(graph: GraphSettings): Promise<void> {
  return invoke('save_graph_settings', { graph });
}

export async function fetchModels(): Promise<string[]> {
  return invoke('fetch_models');
}
