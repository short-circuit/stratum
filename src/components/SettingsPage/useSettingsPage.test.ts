import { renderHook, act, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import * as api from '../../lib/commands';
import { useStore } from '../../stores/appStore';
import { useSyncStore } from '../../stores/syncStore';
import { useSettingsPage } from './useSettingsPage';

// Mock the Tauri command surface so the hook can run in jsdom without a runtime.
vi.mock('../../lib/commands', () => ({
  getSettings: vi.fn(),
  saveSettings: vi.fn(),
  fetchModels: vi.fn(),
  reindexVault: vi.fn(),
  repairDbFromDisk: vi.fn(),
  normalizeAllFiles: vi.fn(),
  startSyncScheduler: vi.fn(),
  pickVaultDirectory: vi.fn(),
  listPages: vi.fn(),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

// Minimal VaultInfo-shaped object the real appStore action stores into `vault`.
const VAULT_INFO = {
  path: '/storage/emulated/0/Documents/MyVault',
  block_count: 0,
  page_count: 0,
};

const SETTINGS = {
  vault_path: '/data/data/com.stratum/files/StratumVault',
  theme: { dark_mode: true, primary_color: '#f97316', secondary_color: '#6b7280', font_size: 16 },
  ai: {
    provider: 'ollama',
    endpoint: null,
    api_key: null,
    api_key_from_env: false,
    model: 'qwen2.5:7b',
    models: [],
    rag_enabled: false,
    rag_chunk_count: 8,
  },
  graph: {
    show_connected: true,
    show_orphaned: false,
    show_tags: true,
    charge_strength: -600,
    link_distance: 120,
    alpha_decay: 0.12,
    velocity_decay: 0.6,
    link_curvature: 0,
  },
  sync: {
    mode: 'manual',
    remote_url: null,
    branch: 'master',
    auto_commit_interval_secs: 0,
    auto_sync_interval_secs: 0,
    ssh_key_path: null,
    commit_template: null,
  },
  research: { searxng_endpoint: '', max_results: 5, max_depth: 3 },
  stt: {
    endpoint: '',
    api_key: null,
    model: '',
    diarize_model: '',
    language: null,
    diarize: false,
    auto_summarize: false,
    auto_identify: false,
  },
  tts: {
    endpoint: '',
    api_key: null,
    voice: 'alloy',
    format: 'mp3',
    speed: 1.0,
  },
};

describe('useSettingsPage — vault path refresh after folder pick (SAF regression)', () => {
  beforeEach(() => {
    vi.clearAllMocks();

    // Seed the appStore as a healthy pre-existing vault and stub the pick to
    // resolve with the "newly selected" SAF folder.
    useStore.setState({
      vault: { path: SETTINGS.vault_path, block_count: 0, page_count: 0 },
      pages: [],
      currentPage: null,
      loading: false,
      error: null,
      themeConfig: { primaryHex: '#f97316', secondaryHex: '#6b7280', dark: true, fontSize: 16 },
    });

    useSyncStore.setState({
      syncStatus: null,
      commits: [],
      syncing: false,
      lastSyncTime: null,
    });

    (api.getSettings as ReturnType<typeof vi.fn>).mockResolvedValue(SETTINGS);
    (api.pickVaultDirectory as ReturnType<typeof vi.fn>).mockResolvedValue(VAULT_INFO);
    (api.fetchModels as ReturnType<typeof vi.fn>).mockResolvedValue([]);
    (api.reindexVault as ReturnType<typeof vi.fn>).mockResolvedValue({ succeeded: 0, failed: 0, processed: 0 });
    (api.repairDbFromDisk as ReturnType<typeof vi.fn>).mockResolvedValue({ succeeded: 0, failed: 0, processed: 0 });
    (api.normalizeAllFiles as ReturnType<typeof vi.fn>).mockResolvedValue(0);
    (api.startSyncScheduler as ReturnType<typeof vi.fn>).mockResolvedValue(undefined);
    (api.listPages as ReturnType<typeof vi.fn>).mockResolvedValue({ pages: [] });
  });

  afterEach(() => {
    useStore.setState({
      vault: null,
      pages: [],
      currentPage: null,
      loading: false,
      error: null,
      themeConfig: { primaryHex: '#f97316', secondaryHex: '#6b7280', dark: true, fontSize: 16 },
    });
  });

  it('reflects the newly selected vault path without an app reload', async () => {
    const { result } = renderHook(() => useSettingsPage());

    // Wait for the initial settings fetch to populate the form state.
    await waitFor(() => {
      expect(result.current.settings).toEqual(SETTINGS);
    });

    // Simulate the user tapping Browse and picking a new SAF folder. The real
    // appStore action resolves to the VaultInfo, which lands in `vault`, and the
    // wrapped handler must merge that path into the local settings state.
    await act(async () => {
      await result.current.handlePickVaultDirectory();
    });

    expect(result.current.settings.vault_path).toBe(VAULT_INFO.path);
    expect(api.pickVaultDirectory).toHaveBeenCalledTimes(1);
  });
});
