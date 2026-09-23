import { render, screen, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import * as api from '../../lib/commands';
import { useStore } from '../../stores/appStore';
import { useSyncStore } from '../../stores/syncStore';
import SettingsPageMobile from './SettingsPage.mobile';

// The mobile page reuses the same `useSettingsPage` hook; mock the Tauri
// command surface so the component can mount in jsdom.
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

const SETTINGS = {
  vault_path: '/storage/emulated/0/Documents/MyVault',
  theme: { dark_mode: true, primary_color: '#f97316', secondary_color: '#6b7280', font_size: 16 },
  ai: {
    provider: 'ollama',
    endpoint: null,
    api_key: 'sk-****secret',
    api_key_from_env: false,
    model: 'qwen2.5:7b',
    models: [],
    rag_enabled: true,
    rag_chunk_count: 5,
    embedding_dimensions: 0,
  },
  research: { searxng_endpoint: 'http://localhost:8888', max_results: 5, max_depth: 3 },
  sync: {
    mode: 'auto_commit',
    remote_url: 'git@github.com:user/vault.git',
    branch: 'main',
    auto_commit_interval_secs: 300,
    auto_sync_interval_secs: 1800,
    ssh_key_path: null,
    commit_template: 'chore: update vault {datetime}',
  },
  stt: { endpoint: '', api_key: null, model: '', diarize_model: '', language: null, diarize: false, auto_summarize: false, auto_identify: false },
  tts: { endpoint: '', api_key: null, voice: 'alloy', format: 'mp3', speed: 1.0 },
};

describe('SettingsPageMobile — parity with desktop Settings', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useStore.setState({
      vault: { path: SETTINGS.vault_path, block_count: 0, page_count: 0 },
      pages: [],
      currentPage: null,
      loading: false,
      error: null,
      themeConfig: { primaryHex: '#f97316', secondaryHex: '#6b7280', dark: true, fontSize: 16 },
    });
    useSyncStore.setState({ syncStatus: null, commits: [], syncing: false, lastSyncTime: null });
    (api.getSettings as ReturnType<typeof vi.fn>).mockResolvedValue(SETTINGS);
    (api.fetchModels as ReturnType<typeof vi.fn>).mockResolvedValue([]);
    (api.listPages as ReturnType<typeof vi.fn>).mockResolvedValue({ pages: [] });
  });

  it('renders all desktop-parity settings sections on mobile', async () => {
    render(<SettingsPageMobile />);
    // Wait for the settings fetch + render
    await waitFor(() => {
      expect(screen.getByText('Save')).toBeInTheDocument();
    });

    // Core sections
    expect(screen.getByText('Vault')).toBeInTheDocument();
    expect(screen.getByText('Theme')).toBeInTheDocument();
    expect(screen.getByText('AI')).toBeInTheDocument();
    expect(screen.getByText('Speech & Audio')).toBeInTheDocument();
    expect(screen.getByText('Research')).toBeInTheDocument();
    expect(screen.getByText('Developer')).toBeInTheDocument();
    expect(screen.getByText('Sync')).toBeInTheDocument();
  });

  it('exposes the full sync configuration (mode, remote, intervals) on mobile', async () => {
    render(<SettingsPageMobile />);
    await waitFor(() => {
      expect(screen.getByText('Save')).toBeInTheDocument();
    });

    // Sync mode controls
    expect(screen.getByText('Auto-Commit')).toBeInTheDocument();
    expect(screen.getByText('Auto-Sync')).toBeInTheDocument();
    expect(screen.getByText('background')).toBeInTheDocument();

    // Remote & SSH fields
    expect(screen.getByDisplayValue('git@github.com:user/vault.git')).toBeInTheDocument();
    // Branch field values are text inputs
    expect(screen.getAllByPlaceholderText('main').length).toBeGreaterThan(0);

    // Auto-commit interval + template + preview
    expect(screen.getByDisplayValue(300)).toBeInTheDocument();
    expect(screen.getByDisplayValue('chore: update vault {datetime}')).toBeInTheDocument();
    expect(screen.getByText(/Preview:/)).toBeInTheDocument();
  });

  it('renders the AI model-fetch / capability surface and RAG chunk count', async () => {
    render(<SettingsPageMobile />);
    await waitFor(() => {
      expect(screen.getByText('Save')).toBeInTheDocument();
    });

    // Fetch models button is present (accordion collapsed)
    expect(screen.getByText('Fetch Available Models')).toBeInTheDocument();
    // RAG chunk count is shown because rag_enabled is true (max=20 input)
    const chunkInput = screen
      .getAllByDisplayValue(5)
      .find(el => (el as HTMLInputElement).max === '20') as HTMLInputElement | undefined;
    expect(chunkInput).toBeDefined();
  });
});
