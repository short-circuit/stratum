import { renderHook, act } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { useAbortableInvoke } from './useAbortableInvoke';

// Cancellation-on-navigation acceptance coverage (E7.F7).
//
// `useAbortableInvoke` is the shared hook that guards every AI action (rewrite,
// summarize, format, mermaid, research) against navigating away mid-flight: when
// the owner component unmounts, the in-flight `invoke()` must be abandoned so no
// late promise can resolve into a dead component. This stubs the real WebView
// bridge (`window.__TAURI_INTERNALS__`) exactly like the audio tests do and
// verifies both the unmount-abort path and the replace-in-flight path.

function stubBridge() {
  let invokeImpl: (cmd: string, args: Record<string, unknown>) => Promise<unknown> =
    () => Promise.resolve('ok');
  const impl = vi.fn(async (cmd: string, args: Record<string, unknown>) =>
    invokeImpl(cmd, args),
  );
  Object.defineProperty(window, '__TAURI_INTERNALS__', {
    configurable: true,
    value: { invoke: impl },
  });
  return {
    impl,
    setImpl: (fn: (cmd: string, args: Record<string, unknown>) => Promise<unknown>) => {
      invokeImpl = fn;
    },
  };
}

describe('useAbortableInvoke', () => {
  beforeEach(() => {
    stubBridge();
  });

  it('aborts an in-flight invoke on unmount (navigation-away guard)', async () => {
    // A promise that never settles on its own; the hook must reject it with an
    // AbortError when the component unmounts (the navigation-away case).
    const bridge = stubBridge();
    let resolvePromise: (v: string) => void = () => {};
    bridge.setImpl(
      () =>
        new Promise((resolve) => {
          resolvePromise = resolve;
        }),
    );

    const { result, unmount } = renderHook(() => useAbortableInvoke());
    let rejection: unknown;
    const pending = act(async () => {
      try {
        await result.current.abortableInvoke('ai_transform_block', { text: 'x' });
      } catch (e) {
        rejection = e;
      }
    });

    // Navigating away unmounts the component.
    unmount();

    await pending;
    expect(rejection).toBeInstanceOf(DOMException);
    expect((rejection as DOMException).name).toBe('AbortError');
    expect(resolvePromise).toBeDefined();
  });

  it('completes normally when not aborted', async () => {
    const bridge = stubBridge();
    bridge.setImpl(async () => 'done');

    const { result } = renderHook(() => useAbortableInvoke());
    const out = await act(async () =>
      result.current.abortableInvoke('ai_transform_block', { text: 'hello' }),
    );
    expect(out).toBe('done');
  });

  it('aborts a previous in-flight call when a new one starts', async () => {
    const bridge = stubBridge();
    let released = false;
    bridge.setImpl(
      () =>
        new Promise<string>((resolve) => {
          // Only resolve once manually released; the hook must reject the first
          // call via AbortController before we release anything.
          const t = setInterval(() => {
            if (released) {
              clearInterval(t);
              resolve('done');
            }
          }, 5);
        }),
    );

    const { result } = renderHook(() => useAbortableInvoke());
    const first = result.current.abortableInvoke('ai_research', { query: 'a' });
    // Second invocation supersedes the first (e.g. user triggers AI again).
    const second = result.current.abortableInvoke('ai_research', { query: 'b' });

    await expect(first).rejects.toMatchObject({ name: 'AbortError' });
    released = true;
    await expect(second).resolves.toBe('done');
  });
});
