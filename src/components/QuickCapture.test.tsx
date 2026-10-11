import { describe, it, expect, vi, beforeEach, type Mock } from 'vitest';
import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import QuickCapture from './QuickCapture';
import { onQuickCaptureOpen, quickCaptureSave } from '../lib/commands';

const unlisten = () => {};
let openCb: ((voice: boolean) => void) | null = null;

vi.mock('../lib/commands', () => ({
  onQuickCaptureOpen: vi.fn((cb: (voice: boolean) => void) => {
    openCb = cb;
    return Promise.resolve(unlisten);
  }),
  onAppQuit: vi.fn(() => Promise.resolve(unlisten)),
  quickCaptureSave: vi.fn(),
  dictationStart: vi.fn(() => Promise.resolve({ recording_path: 'rec', device_name: 'dev', sample_rate: 16000 })),
  dictationStop: vi.fn(() => Promise.resolve({ recording_path: 'rec', duration_secs: 1 })),
  dictationTranscribe: vi.fn(() => Promise.resolve({ markdown: '' })),
  saveLibrary: vi.fn(() => Promise.resolve()),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(() => Promise.resolve(unlisten)),
}));

async function openDialog(voice = false) {
  render(<QuickCapture />);
  await waitFor(() => expect(onQuickCaptureOpen).toHaveBeenCalled());
  act(() => openCb?.(voice));
}

describe('QuickCapture', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    openCb = null;
  });

  it('opens on quick-capture:open and shows the input + title', async () => {
    await openDialog();
    expect(await screen.findByText('Quick Capture')).toBeInTheDocument();
    const input = await screen.findByPlaceholderText('Capture a thought…');
    expect(input.tagName).toBe('TEXTAREA');
  });

  it('typing + Enter saves text, closes the dialog, and shows the saved snackbar', async () => {
    (quickCaptureSave as Mock).mockResolvedValue({ page_path: 'journals/2026-10-11.md', block_id: 'b1' });
    await openDialog();
    const input = await screen.findByPlaceholderText('Capture a thought…');
    fireEvent.change(input, { target: { value: 'text' } });
    fireEvent.keyDown(input, { key: 'Enter', code: 'Enter', keyCode: 13 });

    await waitFor(() => expect(quickCaptureSave).toHaveBeenCalledWith('text'));
    expect(await screen.findByText('Captured to journals/2026-10-11.md')).toBeInTheDocument();
    await waitFor(() => expect(screen.queryByPlaceholderText('Capture a thought…')).not.toBeInTheDocument());
  });

  it('Escape closes without saving', async () => {
    await openDialog();
    const input = await screen.findByPlaceholderText('Capture a thought…');
    fireEvent.keyDown(input, { key: 'Escape', code: 'Escape', keyCode: 27 });
    await waitFor(() => expect(screen.queryByPlaceholderText('Capture a thought…')).not.toBeInTheDocument());
    expect(quickCaptureSave).not.toHaveBeenCalled();
  });

  it('a save error keeps the dialog open and shows the error snackbar', async () => {
    (quickCaptureSave as Mock).mockRejectedValue(new Error('save boom'));
    await openDialog();
    const input = await screen.findByPlaceholderText('Capture a thought…');
    fireEvent.change(input, { target: { value: 'text' } });
    fireEvent.keyDown(input, { key: 'Enter', code: 'Enter', keyCode: 13 });

    expect(await screen.findByText(/save boom/)).toBeInTheDocument();
    expect(screen.getByPlaceholderText('Capture a thought…')).toBeInTheDocument();
  });
});
