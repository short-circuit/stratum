import { render, screen, fireEvent, waitFor, within } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import PluginsPanelDesktop from './PluginsPanel.desktop';

function cardOf(name: string): HTMLElement {
  return screen.getByText(name).closest('.MuiCard-root') as HTMLElement;
}

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
});
