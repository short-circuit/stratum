import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import VaultTab from './VaultTab';

vi.mock('../../lib/commands/import', () => ({
  previewImport: vi.fn(),
  importVault: vi.fn(),
}));

vi.mock('../../lib/commands/features', () => ({
  exportHtml: vi.fn(),
  exportJson: vi.fn(),
  exportPdf: vi.fn(),
  exportZip: vi.fn(),
}));

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
  save: vi.fn(),
}));

describe('VaultTab pages view control', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders the pages view control with both options', () => {
    render(
      <VaultTab
        vaultPath="/v"
        onVaultPathChange={vi.fn()}
        onBrowse={vi.fn()}
        pagesView="flat"
        onPagesViewChange={vi.fn()}
      />,
    );

    expect(screen.getByRole('button', { name: 'Flat list' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Folder tree' })).toBeInTheDocument();
  });

  it('calls onPagesViewChange with tree when Folder tree is selected', () => {
    const onPagesViewChange = vi.fn();
    render(
      <VaultTab
        vaultPath="/v"
        onVaultPathChange={vi.fn()}
        onBrowse={vi.fn()}
        pagesView="flat"
        onPagesViewChange={onPagesViewChange}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Folder tree' }));
    expect(onPagesViewChange).toHaveBeenCalledWith('tree');
  });
});
