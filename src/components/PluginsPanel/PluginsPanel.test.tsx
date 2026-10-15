import { render, screen, fireEvent, waitFor, within } from '@testing-library/react';
import { describe, it, expect, vi, afterEach } from 'vitest';
import PluginsPanelDesktop from './PluginsPanel.desktop';
import { clearCatalogCache } from '../../lib/pluginCatalog';

// E10.F7 fixture mirroring the community catalog (plugins/index.json). The
// install path verifies the sha256 of the downloaded bytes, so the fixture's
// digest MUST match the served bytes (an empty wasm module).
const EMPTY_WASM = new Uint8Array([0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00]);
const EMPTY_WASM_SHA256 = '93a44bbb96c751218e4c00d479e4c14358122a389acca16205b1e4d0dc5f9476';

const CATALOG_FIXTURE = {
  schema: 'stratum-plugins-catalog/v1',
  plugins: [
    {
      schema_version: 1,
      id: 'com.example.hello-world',
      name: 'Hello World',
      version: '0.1.0',
      description: 'Logs note metadata on save and demonstrates the pkm host API',
      author: 'Stratum Community',
      entry: 'plugin.wasm',
      permissions: ['file:read'],
      hooks: { onSave: true },
      repo: 'https://github.com/short-circuit/stratum-plugins',
      source: 'examples/hello-world',
      sha256: EMPTY_WASM_SHA256,
      download_url:
        'https://raw.githubusercontent.com/short-circuit/stratum-plugins/main/plugins/com.example.hello-world/plugin.wasm',
    },
  ],
};

const THEME_FIXTURE = {
  schema: 'stratum-theme-catalog/v1',
  themes: [
    {
      id: 'com.stratum.midnight-indigo',
      name: 'Midnight Indigo',
      version: '1.0.0',
      description: 'A dark indigo theme for late-night note-taking.',
      author: 'Stratum Community',
      dark_mode: true,
      primary_color: '#6366f1',
      secondary_color: '#312e81',
      font_size: 16,
      repo: 'https://github.com/short-circuit/stratum-plugins',
      source: 'themes/com.stratum.midnight-indigo',
      sha256: '0e76c0dce1ecf0fcfc7e7a4fd77ab8eb742b1c9c195d4289ed5d9386f034bca5',
      download_url:
        'https://raw.githubusercontent.com/short-circuit/stratum-plugins/main/themes/com.stratum.midnight-indigo/theme.json',
    },
  ],
};

function stubCatalogFetch(kind: 'plugin' | 'theme'): void {
  const body = kind === 'plugin' ? CATALOG_FIXTURE : THEME_FIXTURE;
  vi.stubGlobal(
    'fetch',
    vi.fn(async (input: RequestInfo | URL) => {
      const url = String(input);
      if (url.endsWith('.wasm')) {
        return {
          ok: true,
          status: 200,
          json: async () => ({ ok: true }),
          arrayBuffer: async () => EMPTY_WASM.buffer,
        } as unknown as Response;
      }
      if (url.endsWith('.json')) {
        return {
          ok: true,
          status: 200,
          json: async () => body,
        } as unknown as Response;
      }
      return {
        ok: true,
        status: 200,
        json: async () => body,
      } as unknown as Response;
    }),
  );
}

function cardOf(name: string): HTMLElement {
  return screen.getByText(name).closest('.MuiCard-root') as HTMLElement;
}

afterEach(() => {
  vi.unstubAllGlobals();
  // The catalog fetch caches payloads in localStorage. Without clearing,
  // a later test's "fetch fails" path serves the cached catalog + Offline
  // chip, and the empty-state Retry button never renders (CI-only flake
  // because jsdom provides working localStorage there).
  clearCatalogCache();
});

describe('PluginsPanelDesktop', () => {
  it('renders the seeded plugin list with statuses', async () => {
    render(<PluginsPanelDesktop />);

    expect(screen.getByText('Plugins')).toBeInTheDocument();

    // The in-memory mock backend seeds 3 plugins.
    await waitFor(() => {
      expect(screen.getByText('Developer Dashboard')).toBeInTheDocument();
    });
    expect(screen.getByText('Daily Summary')).toBeInTheDocument();
    expect(screen.getByText('Broken Example')).toBeInTheDocument();

    // Status labels rendered for each plugin.
    expect(screen.getAllByText('Ready').length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText('Disabled').length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText('Error').length).toBeGreaterThanOrEqual(1);
  });

  it('shows the mock-backend disclaimer when the Tauri backend is unavailable', async () => {
    render(<PluginsPanelDesktop />);
    await waitFor(() => {
      expect(screen.getByText(/mock data/)).toBeInTheDocument();
    });
  });

  it('renders manifest metadata (description, author, hooks) on plugin cards', async () => {
    render(<PluginsPanelDesktop />);
    await waitFor(() => {
      expect(screen.getByText('Developer Dashboard')).toBeInTheDocument();
    });

    const card = cardOf('Developer Dashboard');
    expect(within(card).getByText('Collects development telemetry and posts it to a local endpoint.')).toBeInTheDocument();
    expect(within(card).getByText('By stratum-team')).toBeInTheDocument();
    // Declared-enabled hooks (spec §8) surface next to permissions.
    expect(within(card).getByText('Hooks:')).toBeInTheDocument();
    expect(within(card).getByText('onSave')).toBeInTheDocument();
  });

  it('enables a disabled plugin after clicking Enable', async () => {
    render(<PluginsPanelDesktop />);
    await waitFor(() => {
      expect(screen.getByText('Daily Summary')).toBeInTheDocument();
    });

    const card = cardOf('Daily Summary');
    const enableButton = within(card).getByRole('button', { name: 'Enable' });
    fireEvent.click(enableButton);

    await waitFor(() => {
      expect(within(card).getByRole('button', { name: 'Disable' })).toBeInTheDocument();
    });
  });

  it('runs a host-function test and shows the result', async () => {
    render(<PluginsPanelDesktop />);
    await waitFor(() => {
      expect(screen.getByText('Developer Dashboard')).toBeInTheDocument();
    });

    const card = cardOf('Developer Dashboard');
    fireEvent.click(within(card).getByRole('button', { name: /Test note_read/ }));

    await waitFor(() => {
      expect(within(card).getByText(/note_read: ok/)).toBeInTheDocument();
    });
  });

  it('renders without an error banner in the healthy baseline', async () => {
    render(<PluginsPanelDesktop />);
    await waitFor(() => {
      expect(screen.getByText('Developer Dashboard')).toBeInTheDocument();
    });
    expect(screen.queryByText(/Failed on/)).not.toBeInTheDocument();
  });

  it('installs a plugin through the Install action', async () => {
    const promptSpy = vi.spyOn(window, 'prompt').mockReturnValue('extras.wasm');
    render(<PluginsPanelDesktop />);
    await waitFor(() => {
      expect(screen.getByText('Developer Dashboard')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole('button', { name: 'Install' }));

    await waitFor(() => {
      expect(promptSpy).toHaveBeenCalled();
      expect(screen.getAllByText('installed-extras').length).toBeGreaterThan(0);
    });
    promptSpy.mockRestore();
  });

  it('uninstalls a plugin from its card', async () => {
    render(<PluginsPanelDesktop />);
    await waitFor(() => {
      expect(screen.getByText('Daily Summary')).toBeInTheDocument();
    });

    const card = cardOf('Daily Summary');
    fireEvent.click(within(card).getByRole('button', { name: 'Uninstall' }));

    await waitFor(() => {
      expect(screen.queryByText('Daily Summary')).not.toBeInTheDocument();
    });
  });

  // ── E10.F7: Plugin Directory / Theme Hub tabs ──

  it('shows the three tabs (Installed / Directory / Themes)', async () => {
    render(<PluginsPanelDesktop />);
    await waitFor(() => {
      expect(screen.getByText('Developer Dashboard')).toBeInTheDocument();
    });
    expect(screen.getByRole('tab', { name: 'Installed' })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'Directory' })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'Themes' })).toBeInTheDocument();
  });

  it('lists catalog plugins in the Directory tab with sha256 + install', async () => {
    stubCatalogFetch('plugin');
    render(<PluginsPanelDesktop />);
    await waitFor(() => {
      expect(screen.getByText('Developer Dashboard')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole('tab', { name: 'Directory' }));

    await waitFor(() => {
      expect(screen.getByText('Hello World')).toBeInTheDocument();
    });
    const card = cardOf('Hello World');
    expect(within(card).getByText('com.example.hello-world')).toBeInTheDocument();
    expect(within(card).getByText('file:read')).toBeInTheDocument();
    expect(within(card).getByText(/sha256/i)).toBeInTheDocument();
    expect(within(card).getByRole('button', { name: 'Install' })).toBeInTheDocument();
  });

  it('installs a catalog plugin (digest-verified) and marks it installed', async () => {
    stubCatalogFetch('plugin');
    render(<PluginsPanelDesktop />);
    await waitFor(() => {
      expect(screen.getByText('Developer Dashboard')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole('tab', { name: 'Directory' }));
    await waitFor(() => {
      expect(screen.getByText('Hello World')).toBeInTheDocument();
    });

    const card = cardOf('Hello World');
    fireEvent.click(within(card).getByRole('button', { name: 'Install' }));

    await waitFor(() => {
      expect(within(cardOf('Hello World')).getByText('Installed')).toBeInTheDocument();
    });
  });

  it('lists catalog themes in the Themes tab', async () => {
    stubCatalogFetch('theme');
    render(<PluginsPanelDesktop />);
    await waitFor(() => {
      expect(screen.getByText('Developer Dashboard')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole('tab', { name: 'Themes' }));

    await waitFor(() => {
      expect(screen.getByText('Midnight Indigo')).toBeInTheDocument();
    });
    expect(screen.getByText('com.stratum.midnight-indigo')).toBeInTheDocument();
  });

  it('renders the directory error + retry when the catalog fetch fails', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => {
        throw new Error('network down');
      }),
    );
    render(<PluginsPanelDesktop />);
    await waitFor(() => {
      expect(screen.getByText('Developer Dashboard')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole('tab', { name: 'Directory' }));

    await waitFor(() => {
      expect(screen.getByRole('button', { name: 'Retry' })).toBeInTheDocument();
    });
  });
});
