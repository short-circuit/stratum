import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import ImportDialog from './index';
import { previewImport, importVault } from '../../lib/commands/import';
import { useStore } from '../../stores/appStore';
import type { ImportPreview, ImportReport } from '../../lib/types';

// Stub the Tauri command surface so the dialog renders in jsdom without a
// runtime backend.
vi.mock('../../lib/commands/import', () => ({
  previewImport: vi.fn(),
  importVault: vi.fn(),
}));

// The dialog picks the source folder via a dynamic native dialog plugin import;
// stub `open` to return a fixed path so the browse flow works in tests.
vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn().mockResolvedValue('/some/source'),
}));

const previewResult: ImportPreview = {
  source_path: '/some/source',
  source_name: 'source',
  format: 'obsidian',
  pages: [{ source_rel: 'a.md', rel_path: 'imported/source/a.md', title: 'A', status: 'new', size_bytes: 10 }],
  assets: [{ source_rel: 'img.png', rel_path: 'imported/source/img.png', status: 'new', size_bytes: 20 }],
  total_pages: 2,
  total_assets: 1,
  duplicates: [],
  conflicts: [],
};

const reportResult: ImportReport = {
  source_path: '/some/source',
  source_name: 'source',
  format: 'obsidian',
  pages_imported: 2,
  assets_imported: 1,
  duplicates_skipped: 0,
  conflicts_skipped: 0,
  dry_run: false,
  errors: [],
  imported_paths: [],
};

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(previewImport).mockResolvedValue(previewResult);
  vi.mocked(importVault).mockResolvedValue(reportResult);
  // Refreshing the page list after an import must not crash the test; no-op it.
  vi.spyOn(useStore.getState(), 'loadPages').mockResolvedValue(undefined);
});

describe('ImportDialog', () => {
  it('renders the title and the empty prompt when no source is selected', () => {
    render(<ImportDialog open onClose={() => {}} />);

    expect(screen.getByText('Import notes')).toBeInTheDocument();
    expect(screen.getByText('Pick a source folder to begin.')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Browse/ })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Close' })).toBeInTheDocument();
  });

  it('selecting a source and clicking Preview calls previewImport and renders the preview counts', async () => {
    render(<ImportDialog open onClose={() => {}} />);

    fireEvent.click(screen.getByRole('button', { name: /Browse/ }));

    // After choosing a folder the preview/import actions become available.
    const previewBtn = await screen.findByRole('button', { name: 'Preview' });
    fireEvent.click(previewBtn);

    await waitFor(() => {
      expect(previewImport).toHaveBeenCalledWith(
        '/some/source',
        expect.objectContaining({ format: 'auto', dry_run: true }),
      );
    });

    expect(await screen.findByText(/Obsidian/)).toBeInTheDocument();
    expect(screen.getByText(/Pages \(2\)/)).toBeInTheDocument();
    expect(screen.getByText(/Assets \(1\)/)).toBeInTheDocument();
  });

  it('clicking Import calls importVault and renders the report summary', async () => {
    render(<ImportDialog open onClose={() => {}} />);

    const browseBtn = await screen.findByRole('button', { name: /Browse/ });
    fireEvent.click(browseBtn);

    const importBtn = await screen.findByRole('button', { name: 'Import' });
    fireEvent.click(importBtn);

    await waitFor(() => {
      expect(importVault).toHaveBeenCalledWith(
        '/some/source',
        expect.objectContaining({ format: 'auto', dry_run: false }),
      );
    });

    expect(await screen.findByText(/2 pages/)).toBeInTheDocument();
    expect(screen.getByText('Import report')).toBeInTheDocument();
  });
});
