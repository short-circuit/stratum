import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import AttachmentPanel from './AttachmentPanel';

const mockShowError = vi.fn();
const mockOpenPage = vi.fn();
const mockGetPageAssetRefs = vi.fn();
const mockImportAssetBytes = vi.fn();
const mockInsertBlock = vi.fn();
const mockDeleteAssetRef = vi.fn();
const mockAssetFileToBase64 = vi.fn();

vi.mock('../lib/commands', () => ({
  getPageAssetRefs: (...a: unknown[]) => mockGetPageAssetRefs(...a),
  importAssetBytes: (...a: unknown[]) => mockImportAssetBytes(...a),
  insertBlock: (...a: unknown[]) => mockInsertBlock(...a),
  deleteAssetRef: (...a: unknown[]) => mockDeleteAssetRef(...a),
}));

vi.mock('../lib/assetUpload', () => ({
  ASSET_MAX_BYTES: 8 * 1024 * 1024,
  assetFileToBase64: (...a: unknown[]) => mockAssetFileToBase64(...a),
}));

vi.mock('../stores/appStore', () => ({
  useStore: {
    getState: () => ({ showError: mockShowError, openPage: mockOpenPage }),
  },
}));

const refs = [
  {
    vault_relative_path: 'assets/foo-abc123.png',
    mime: 'image/png',
    size: 2048,
    sha256: 'aa',
  },
  {
    vault_relative_path: 'assets/bar-de456.pdf',
    mime: 'application/pdf',
    size: 4096,
    sha256: 'bb',
  },
];

beforeEach(() => {
  vi.clearAllMocks();
  mockGetPageAssetRefs.mockResolvedValue(refs);
  mockImportAssetBytes.mockResolvedValue({
    vault_relative_path: 'assets/new-uploaded.png',
    mime: 'image/jpeg',
    size: 100,
    sha256: 'cc',
  });
  mockAssetFileToBase64.mockResolvedValue({ b64: 'aGVsbG8=', mime: 'image/jpeg' });
  mockInsertBlock.mockResolvedValue({});
  mockDeleteAssetRef.mockResolvedValue(undefined);
  mockOpenPage.mockResolvedValue(undefined);
});

describe('AttachmentPanel (E9.F2)', () => {
  it('renders the attachment header and lists page assets', async () => {
    render(<AttachmentPanel pagePath="pages/test.md" />);
    await waitFor(() =>
      expect(screen.getByText(/Attachments \(2\)/)).toBeInTheDocument(),
    );
    expect(screen.getByText('foo-abc123.png')).toBeInTheDocument();
    expect(screen.getByText('bar-de456.pdf')).toBeInTheDocument();
    expect(mockGetPageAssetRefs).toHaveBeenCalledWith('pages/test.md');
  });

  it('renders the empty state when the page has no assets', async () => {
    mockGetPageAssetRefs.mockResolvedValue([]);
    render(<AttachmentPanel pagePath="pages/test.md" />);
    await waitFor(() =>
      expect(screen.getByText('No attachments yet')).toBeInTheDocument(),
    );
  });

  it('calls deleteAssetRef + openPage when the trash button is clicked', async () => {
    const confirm = vi.spyOn(window, 'confirm').mockReturnValue(true);
    render(<AttachmentPanel pagePath="pages/test.md" />);
    await waitFor(() =>
      expect(screen.getByText('foo-abc123.png')).toBeInTheDocument(),
    );
    fireEvent.click(screen.getAllByTitle('Delete')[0]);
    await waitFor(() =>
      expect(mockDeleteAssetRef).toHaveBeenCalledWith('assets/foo-abc123.png'),
    );
    expect(mockOpenPage).toHaveBeenCalledWith('pages/test.md');
    confirm.mockRestore();
  });

  it('uploads a file and inserts an asset embed block', async () => {
    render(<AttachmentPanel pagePath="pages/test.md" />);
    await waitFor(() =>
      expect(screen.getByText('Attachments (2)')).toBeInTheDocument(),
    );
    const input = document.getElementById('asset-file-input') as HTMLInputElement;
    const file = new File(['hello'], 'photo.png', { type: 'image/png' });
    fireEvent.change(input, { target: { files: [file] } });
    await waitFor(() =>
      expect(mockImportAssetBytes).toHaveBeenCalled(),
    );
    expect(mockInsertBlock).toHaveBeenCalledWith(
      'pages/test.md',
      '![[assets/new-uploaded.png]]',
    );
    expect(mockOpenPage).toHaveBeenCalledWith('pages/test.md');
  });
});
