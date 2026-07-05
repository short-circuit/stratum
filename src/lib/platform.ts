import { useState } from 'react';

// ---------------------------------------------------------------------------
// Global declarations for Tauri v2 injected globals.
// These are set by the Tauri runtime — not part of @tauri-apps/api.
// ---------------------------------------------------------------------------
declare global {
  interface Window {
    __TAURI_ENV__?: Record<string, string>;
    __TAURI_PLATFORM__?: string;
  }
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type Platform = 'desktop' | 'mobile';

export interface PlatformInfo {
  /** 'desktop' or 'mobile' */
  platform: Platform;
  /** Convenience: true when platform === 'desktop' */
  isDesktop: boolean;
  /** Convenience: true when platform === 'mobile' */
  isMobile: boolean;
  /** True when running inside a Tauri webview (any OS) */
  isTauri: boolean;
  /**
   * Tauri OS string: 'windows' | 'macos' | 'linux' | 'android' | 'ios',
   * or null when not in Tauri.
   */
  os: string | null;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function detect(): PlatformInfo {
  const platformVar = typeof window !== 'undefined' ? window.__TAURI_PLATFORM__ : undefined;
  const envVar = typeof window !== 'undefined' ? window.__TAURI_ENV__ : undefined;

  const isTauri = platformVar !== undefined || envVar !== undefined;
  const os: string | null = platformVar ?? null;

  let platform: Platform;
  if (os === 'android' || os === 'ios') {
    platform = 'mobile';
  } else {
    platform = 'desktop';
  }

  return {
    platform,
    isDesktop: platform === 'desktop',
    isMobile: platform === 'mobile',
    isTauri,
    os,
  };
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Synchronous platform detection – uses Tauri v2 injected globals
 * (`window.__TAURI_PLATFORM__` / `__TAURI_ENV__`) so there is no async import
 * cost.
 *
 * Outside of a Tauri webview both globals are absent and the function
 * defaults to `{ platform: 'desktop', isTauri: false }`.
 *
 * NOTE: Not cached – each call re-checks the globals, allowing Tauri's
 * async initialization to complete before the value is read.
 */
export function getPlatform(): PlatformInfo {
  return detect();
}

/**
 * React convenience hook that wraps `getPlatform()` in a `useState` call.
 * Because the underlying value never changes after first invocation this
 * does not set up any listeners – it simply returns the singleton.
 */
export function usePlatform(): PlatformInfo {
  const [info] = useState<PlatformInfo>(getPlatform);
  return info;
}
