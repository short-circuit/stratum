import { useCallback, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';

/**
 * Hook that wraps Tauri invoke() with automatic abort on unmount.
 * Prevents setState calls on unmounted components when users navigate away
 * while in-flight operations are still pending.
 *
 * Usage:
 *   const { abortableInvoke } = useAbortableInvoke();
 *   const data = await abortableInvoke<ReturnType>('command_name', { arg1: val1 });
 */
export function useAbortableInvoke() {
  const controllerRef = useRef<AbortController | null>(null);

  // Cleanup: abort any in-flight calls on unmount
  useEffect(() => {
    return () => {
      controllerRef.current?.abort();
      controllerRef.current = null;
    };
  }, []);

  const abortableInvoke = useCallback(
    async <T>(
      cmd: string,
      args?: Record<string, unknown>,
    ): Promise<T> => {
      // Abort any previous in-flight call for this hook instance
      controllerRef.current?.abort();

      const controller = new AbortController();
      controllerRef.current = controller;

      try {
        return await Promise.race([
          invoke<T>(cmd, args),
          new Promise<T>((_, reject) => {
            const onAbort = () => {
              reject(
                new DOMException('The operation was aborted', 'AbortError'),
              );
            };
            controller.signal.addEventListener('abort', onAbort, { once: true });
          }),
        ]);
      } catch (e) {
        // Propagate AbortError so callers can distinguish from real errors
        if (e instanceof DOMException && e.name === 'AbortError') {
          throw e;
        }
        throw e;
      }
    },
    [],
  );

  return { abortableInvoke };
}
