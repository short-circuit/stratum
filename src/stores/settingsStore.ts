import { create } from 'zustand';
import type { GraphSettings, SyncSettings } from '../lib/types';
import * as api from '../lib/commands';

// --- Types ---

export interface AiConfig {
  provider: string;
  endpoint: string | null;
  api_key: string | null;
  api_key_from_env: boolean;
  model: string;
  models: { name: string; capabilities: string[] }[];
  rag_enabled: boolean;
  rag_chunk_count: number;
}

export interface ThemeSettings {
  dark_mode: boolean;
  primary_color: string;
  secondary_color: string;
  font_size: number;
}

export interface ResearchSettings {
  searxng_endpoint: string;
  max_results: number;
  max_depth: number;
}

export interface AppSettings {
  vault_path: string;
  theme: ThemeSettings;
  ai: AiConfig;
  graph: GraphSettings;
  sync: SyncSettings;
  research: ResearchSettings;
  [key: string]: unknown;
}

export interface SettingsState {
  settings: AppSettings | null;
  loading: boolean;
  error: string | null;

  loadSettings: () => Promise<void>;
  saveSettings: () => Promise<void>;
  updateTheme: (patch: Partial<ThemeSettings>) => void;
  updateAi: (patch: Partial<AiConfig>) => void;
  updateSync: (patch: Partial<SyncSettings>) => void;
  updateResearch: (patch: Partial<ResearchSettings>) => void;
  setSettings: (settings: AppSettings) => void;
}

const DEFAULT_THEME: ThemeSettings = {
  dark_mode: true,
  primary_color: '#f97316',
  secondary_color: '#6b7280',
  font_size: 16,
};

const DEFAULT_RESEARCH: ResearchSettings = {
  searxng_endpoint: 'http://localhost:8888',
  max_results: 3,
  max_depth: 2,
};

const DEFAULT_GRAPH: GraphSettings = {
  show_connected: true,
  show_orphaned: true,
  show_tags: true,
  charge_strength: -4,
  link_distance: 40,
  alpha_decay: 0.15,
  velocity_decay: 0.4,
  link_curvature: 0.15,
  node_cap: 0,
};

const DEFAULT_SYNC_SETTINGS: SyncSettings = {
  mode: 'manual',
  remote_url: null,
  branch: 'main',
  auto_commit_interval_secs: 300,
  auto_sync_interval_secs: 1800,
  ssh_key_path: null,
  commit_template:
    'stratum({datetime}): {editedfiles} edited, {newfiles} added, {deletedfiles} deleted',
};

const DEFAULT_AI: AiConfig = {
  provider: 'ollama',
  endpoint: null,
  api_key: null,
  api_key_from_env: false,
  model: '',
  models: [],
  rag_enabled: false,
  rag_chunk_count: 3,
};

const DEFAULT_SETTINGS: AppSettings = {
  vault_path: '',
  theme: DEFAULT_THEME,
  ai: DEFAULT_AI,
  graph: DEFAULT_GRAPH,
  sync: DEFAULT_SYNC_SETTINGS,
  research: DEFAULT_RESEARCH,
};

export const useSettingsStore = create<SettingsState>((set, get) => ({
  settings: null,
  loading: false,
  error: null,

  loadSettings: async () => {
    set({ loading: true, error: null });
    try {
      const result = await api.getSettings();
      set({
        settings: {
          ...DEFAULT_SETTINGS,
          ...result,
          theme: { ...DEFAULT_THEME, ...result.theme },
          ai: { ...DEFAULT_AI, ...result.ai },
          graph: { ...DEFAULT_GRAPH, ...result.graph },
          sync: { ...DEFAULT_SYNC_SETTINGS, ...result.sync },
          // research is only merged if it exists in the response
          research: (result as any).research
            ? { ...DEFAULT_RESEARCH, ...(result as any).research }
            : DEFAULT_RESEARCH,
        },
        loading: false,
      });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  saveSettings: async () => {
    const s = get().settings;
    if (!s) return;
    set({ error: null });
    try {
      await api.saveSettings(s);
    } catch (e) {
      set({ error: String(e) });
    }
  },

  updateTheme: (patch) => {
    const s = get().settings;
    if (!s) return;
    set({ settings: { ...s, theme: { ...s.theme, ...patch } } });
  },

  updateAi: (patch) => {
    const s = get().settings;
    if (!s) return;
    set({ settings: { ...s, ai: { ...s.ai, ...patch } } });
  },

  updateSync: (patch) => {
    const s = get().settings;
    if (!s) return;
    set({ settings: { ...s, sync: { ...s.sync, ...patch } } });
  },

  updateResearch: (patch) => {
    const s = get().settings;
    if (!s) return;
    set({ settings: { ...s, research: { ...s.research, ...patch } } });
  },

  setSettings: (settings) => set({ settings }),
}));
