import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import QueryPanelDesktop from './QueryPanel.desktop';
import type { SavedQuery } from '../../lib/types';

vi.mock('../../lib/commands', () => ({
  runQuery: vi.fn(),
  listSavedQueries: vi.fn(),
  saveSavedQuery: vi.fn(),
  renameSavedQuery: vi.fn(),
  deleteSavedQuery: vi.fn(),
}));

const QUERIES: SavedQuery[] = [
  { name: 'All TODO', query: '{:query [:find ?b :where [?b :block/marker "TODO"]]}', updated_at: '2026-09-22T20:00:00Z' },
];

import * as api from '../../lib/commands';

describe('QueryPanelDesktop — saved queries', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    (api.listSavedQueries as ReturnType<typeof vi.fn>).mockResolvedValue(QUERIES);
    (api.saveSavedQuery as ReturnType<typeof vi.fn>).mockResolvedValue(QUERIES[0]);
    (api.renameSavedQuery as ReturnType<typeof vi.fn>).mockResolvedValue({ ...QUERIES[0], name: 'Renamed' });
    (api.deleteSavedQuery as ReturnType<typeof vi.fn>).mockResolvedValue(undefined);
    (api.runQuery as ReturnType<typeof vi.fn>).mockResolvedValue({ columns: [], rows: [] });
  });

  it('renders the saved query list once loaded', async () => {
    render(<QueryPanelDesktop />);

    expect(screen.getByText('Datalog Query')).toBeInTheDocument();

    await waitFor(() => {
      expect(screen.getByText('All TODO')).toBeInTheDocument();
    });
  });

  it('shows an empty state when there are no saved queries', async () => {
    (api.listSavedQueries as ReturnType<typeof vi.fn>).mockResolvedValue([]);
    render(<QueryPanelDesktop />);

    await waitFor(() => {
      expect(screen.getByText(/No saved queries yet/)).toBeInTheDocument();
    });
  });

  it('loads a saved query into the editor on click', async () => {
    render(<QueryPanelDesktop />);

    await waitFor(() => {
      expect(screen.getByText('All TODO')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText('All TODO'));

    // The editable textarea (the Datalog input) is one of the multiline
    // textboxes; confirm it now contains the saved query text.
    const editor = document.querySelector('textarea') as HTMLTextAreaElement;
    expect(editor.value).toContain('{:query [:find ?b');
  });

  it('renames a saved query through the rename dialog', async () => {
    (api.listSavedQueries as ReturnType<typeof vi.fn>).mockResolvedValueOnce([...QUERIES])
      .mockResolvedValueOnce([{ ...QUERIES[0], name: 'Renamed' }]);
    render(<QueryPanelDesktop />);

    await waitFor(() => {
      expect(screen.getByText('All TODO')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole('button', { name: 'Rename saved query All TODO' }));

    await screen.findByLabelText('Name');
    fireEvent.change(screen.getByLabelText('Name'), { target: { value: 'Renamed' } });
    fireEvent.click(screen.getByRole('button', { name: 'Rename' }));

    await waitFor(() => {
      expect(screen.getByText('Renamed')).toBeInTheDocument();
    });
    expect(api.renameSavedQuery).toHaveBeenCalledWith('All TODO', 'Renamed');
  });

  it('deletes a saved query after confirming', async () => {
    (api.listSavedQueries as ReturnType<typeof vi.fn>).mockResolvedValueOnce([...QUERIES])
      .mockResolvedValueOnce([]);
    render(<QueryPanelDesktop />);

    await waitFor(() => {
      expect(screen.getByText('All TODO')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole('button', { name: 'Delete saved query All TODO' }));

    await waitFor(() => {
      expect(screen.getByText(/Delete “All TODO”/)).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole('button', { name: 'Delete' }));

    await waitFor(() => {
      expect(screen.getByText(/No saved queries yet/)).toBeInTheDocument();
    });
    expect(api.deleteSavedQuery).toHaveBeenCalledWith('All TODO');
  });

  it('opens the save dialog and saves the current query', async () => {
    (api.listSavedQueries as ReturnType<typeof vi.fn>).mockResolvedValueOnce([...QUERIES])
      .mockResolvedValueOnce([...QUERIES, { name: 'Saved Again', query: 'X', updated_at: '2026-09-22T22:00:00Z' }]);
    render(<QueryPanelDesktop />);

    await waitFor(() => {
      expect(screen.getByText('All TODO')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole('button', { name: /Save Query/ }));

    await screen.findByLabelText('Name');
    fireEvent.change(screen.getByLabelText('Name'), { target: { value: 'Saved Again' } });
    fireEvent.click(screen.getByRole('button', { name: 'Save' }));

    await waitFor(() => {
      expect(screen.getByText('Saved Again')).toBeInTheDocument();
    });
    expect(api.saveSavedQuery).toHaveBeenCalled();
  });

  it('disables saving when the name is empty', async () => {
    render(<QueryPanelDesktop />);

    await waitFor(() => {
      expect(screen.getByText('All TODO')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole('button', { name: /Save Query/ }));

    const saveButton = await screen.findByRole('button', { name: 'Save' });
    expect(saveButton).toBeDisabled();
    expect(api.saveSavedQuery).not.toHaveBeenCalled();
  });

  it('shows the load error alert when the saved-query list fails to load', async () => {
    (api.listSavedQueries as ReturnType<typeof vi.fn>).mockRejectedValue(new Error('disk error'));
    render(<QueryPanelDesktop />);

    await waitFor(() => {
      expect(screen.getByText(/Could not load saved queries: Error: disk error/)).toBeInTheDocument();
    });
  });

  it('shows the action error alert when a mutation fails, keeps the dialog open, and dismisses the alert', async () => {
    render(<QueryPanelDesktop />);

    await waitFor(() => {
      expect(screen.getByText('All TODO')).toBeInTheDocument();
    });

    (api.deleteSavedQuery as ReturnType<typeof vi.fn>).mockRejectedValueOnce(new Error('duplicate name'));
    fireEvent.click(screen.getByRole('button', { name: 'Delete saved query All TODO' }));

    await waitFor(() => {
      expect(screen.getByText(/Delete “All TODO”/)).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole('button', { name: 'Delete' }));

    // The failure is surfaced in a dismissible warning alert…
    await waitFor(() => {
      expect(screen.getByText(/Error: duplicate name/)).toBeInTheDocument();
    });

    // …and the confirm dialog stays open (state cleanup only runs after a
    // successful mutation), so the user can retry. Document this behavior.
    expect(screen.getByRole('button', { name: 'Cancel' })).toBeInTheDocument();

    // Dismiss the still-open confirm dialog, then the error alert.
    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    await waitFor(() => {
      expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    });

    // The MUI Alert close affordance is an icon button (data-testid CloseIcon).
    const closeBtn = screen.getByTestId('CloseIcon').closest('button');
    expect(closeBtn).not.toBeNull();
    fireEvent.click(closeBtn as HTMLButtonElement);
    await waitFor(() => {
      expect(screen.queryByText(/Error: duplicate name/)).not.toBeInTheDocument();
    });
  });
});
