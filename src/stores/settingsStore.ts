import { create } from 'zustand';
import type { GraphSettings, SyncSettings } from '../lib/types';
import * as api from '../lib/commands';
import { setLocale } from '../i18n';

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
  embedding_dimensions: number;
  use_llm_gateway_and_auth: boolean;
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

export interface SttSettings {
  endpoint: string;
  api_key: string | null;
  model: string;
  diarize_model: string;
  language: string | null;
  diarize: boolean;
  auto_summarize: boolean;
  auto_identify: boolean;
  use_llm_gateway_and_auth: boolean;
}

export interface TtsSettings {
  endpoint: string;
  api_key: string | null;
  voice: string;
  format: string;
  speed: number;
  use_llm_gateway_and_auth: boolean;
}

export interface TraySettings {
  enabled: boolean;
  close_to_tray: boolean;
  quick_capture_shortcut: string;
  capture_target: string;
  inbox_path: string;
}

export interface VoiceSettings {
  enabled: boolean;
  target: string;
}

export interface AppSettings {
  vault_path: string;
  locale: string;
  theme: ThemeSettings;
  ai: AiConfig;
  graph: GraphSettings;
  sync: SyncSettings;
  research: ResearchSettings;
  stt: SttSettings;
  tts: TtsSettings;
  tray: TraySettings;
  voice: VoiceSettings;
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
  updateStt: (patch: Partial<SttSettings>) => void;
  updateTts: (patch: Partial<TtsSettings>) => void;
  updateTray: (patch: Partial<TraySettings>) => void;
  updateVoice: (patch: Partial<VoiceSettings>) => void;
  updateLocale: (locale: string) => void;
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
  use_3d: false,
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
  backup: {
    enabled: false,
    mode: 'folder',
    interval_minutes: 60,
    exclude: ['.git', '.pkm/blocks.db', '.pkm/search', '.pkm/backup_state.json', '.pkm/sync_state.json', '.DS_Store'],
    webdav_url: null,
    webdav_username: null,
    webdav_password: null,
    folder_path: null,
  },
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
  embedding_dimensions: 0,
  use_llm_gateway_and_auth: false,
};

const DEFAULT_STT: SttSettings = {
  endpoint: '',
  api_key: null,
  model: 'whisper-1',
  diarize_model: 'pyannote-diarization',
  language: null,
  diarize: true,
  auto_summarize: true,
  auto_identify: true,
  use_llm_gateway_and_auth: false,
};

const DEFAULT_TTS: TtsSettings = {
  endpoint: '',
  api_key: null,
  voice: 'alloy',
  format: 'mp3',
  speed: 1.0,
  use_llm_gateway_and_auth: false,
};

export const DEFAULT_TRAY: TraySettings = {
  enabled: true,
  close_to_tray: true,
  quick_capture_shortcut: 'CmdOrCtrl+Shift+Space',
  capture_target: 'journal',
  inbox_path: 'Inbox.md',
};

/** Default voice-capture settings. Keep in sync with `VoiceCaptureConfig::default()`. */
const DEFAULT_VOICE: VoiceSettings = {
  enabled: true,
  target: 'journal:today',
};

const DEFAULT_SETTINGS: AppSettings = {
  vault_path: '',
  locale: 'en',
  theme: DEFAULT_THEME,
  ai: DEFAULT_AI,
  graph: DEFAULT_GRAPH,
  sync: DEFAULT_SYNC_SETTINGS,
  research: DEFAULT_RESEARCH,
  stt: DEFAULT_STT,
  tts: DEFAULT_TTS,
  tray: DEFAULT_TRAY,
  voice: DEFAULT_VOICE,
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
          locale: result.locale ?? 'en',
          theme: { ...DEFAULT_THEME, ...result.theme },
          ai: { ...DEFAULT_AI, ...result.ai },
          graph: { ...DEFAULT_GRAPH, ...result.graph },
          sync: { ...DEFAULT_SYNC_SETTINGS, ...result.sync },
          // research is only merged if it exists in the response
          research: result.research ? { ...DEFAULT_RESEARCH, ...result.research } : DEFAULT_RESEARCH,
          stt: result.stt ? { ...DEFAULT_STT, ...result.stt } : DEFAULT_STT,
          tts: result.tts ? { ...DEFAULT_TTS, ...result.tts } : DEFAULT_TTS,
          tray: { ...DEFAULT_TRAY, ...result.tray },
          voice: result.voice ? { ...DEFAULT_VOICE, ...result.voice } : DEFAULT_VOICE,
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

  updateStt: (patch) => {
    const s = get().settings;
    if (!s) return;
    set({ settings: { ...s, stt: { ...s.stt, ...patch } } });
  },

  updateTts: (patch) => {
    const s = get().settings;
    if (!s) return;
    set({ settings: { ...s, tts: { ...s.tts, ...patch } } });
  },

  updateTray: (patch) => {
    const s = get().settings;
    if (!s) return;
    set({ settings: { ...s, tray: { ...s.tray, ...patch } } });
  },

  updateVoice: (patch) => {
    const s = get().settings;
    if (!s) return;
    set({ settings: { ...s, voice: { ...s.voice, ...patch } } });
  },

  updateLocale: (locale) => {
    const s = get().settings;
    if (!s) return;
    set({ settings: { ...s, locale } });
    void setLocale(locale);
  },

  setSettings: (settings) => set({ settings }),
}));
