import { describe, it, expect, vi, beforeEach } from 'vitest';
import { useRecentsStore } from '../stores/recentsStore';
import { subscribePagesChanged, PAGES_CHANGED_EVENT } from './recentsEvents';

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(),
}));

describe('recentsEvents — watcher event bridge', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('subscribes to the backend "pages-changed" event', async () => {
    const { listen } = await import('@tauri-apps/api/event');
    (listen as ReturnType<typeof vi.fn>).mockResolvedValue(() => {});

    const unlisten = await subscribePagesChanged(() => {});

    expect(listen).toHaveBeenCalledWith(
      PAGES_CHANGED_EVENT,
      expect.any(Function),
    );
    expect(typeof unlisten).toBe('function');
  });

  it('routes the event into the recents store refresh', async () => {
    const { listen } = await import('@tauri-apps/api/event');
    let handler: (() => void) | undefined;
    (listen as ReturnType<typeof vi.fn>).mockImplementation((_evt: string, cb: () => void) => {
      handler = cb;
      return Promise.resolve(() => {});
    });

    const refreshSpy = vi
      .spyOn(useRecentsStore.getState(), 'refresh')
      .mockImplementation(() => {});

    await subscribePagesChanged(() => {
      useRecentsStore.getState().refresh();
    });

    expect(handler).toBeDefined();
    handler!();
    expect(refreshSpy).toHaveBeenCalledTimes(1);
    refreshSpy.mockRestore();
  });

  it('returns an unsubscribe function (calls through to listen cleanup)', async () => {
    const { listen } = await import('@tauri-apps/api/event');
    const cleanup = vi.fn();
    (listen as ReturnType<typeof vi.fn>).mockResolvedValue(cleanup);

    const unlisten = await subscribePagesChanged(() => {});
    unlisten();

    expect(cleanup).toHaveBeenCalledTimes(1);
  });
});
