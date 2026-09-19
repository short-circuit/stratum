import { useState } from 'react';

// ---------------------------------------------------------------------------
// Global declarations for Tauri v2 injected globals.
// These are set by the Tauri runtime — not part of @tauri-apps/api.
// ---------------------------------------------------------------------------
declare global {
  interface Window {
    __TAURI_ENV__?: Record<string, string>;
    __TAURI_PLATFORM__?: string;
    __TAURI_INTERNALS__?: unknown;
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

function uaToOs(ua: string): string | null {
  if (/android/i.test(ua)) return 'android';
  if (/iphone|ipad|ipod/i.test(ua)) return 'ios';
  if (/macintosh|mac os x/i.test(ua)) return 'macos';
  if (/windows/i.test(ua)) return 'windows';
  if (/linux/i.test(ua)) return 'linux';
  return null;
}

function detect(): PlatformInfo {
  const platformVar = typeof window !== 'undefined' ? window.__TAURI_PLATFORM__ : undefined;
  const envVar = typeof window !== 'undefined' ? window.__TAURI_ENV__ : undefined;
  const hasInternals =
    typeof window !== 'undefined' && window.__TAURI_INTERNALS__ !== undefined;

  // Tauri 2.x does NOT inject `__TAURI_PLATFORM__`/`__TAURI_ENV__` in all
  // builds (confirmed absent in tauri 2.11.5 Android). The only reliable
  // Tauri marker in the WebView is `window.__TAURI_INTERNALS__`; OS detection
  // must fall back to the user agent so mobile routing (e.g. the SAF folder
  // picker) actually works on-device.
  const isTauri = platformVar !== undefined || envVar !== undefined || hasInternals;

  const ua = typeof navigator !== 'undefined' ? navigator.userAgent : '';
  const os: string | null = platformVar ?? uaToOs(ua);

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
 * Synchronous platform detection. Priority:
 *  1. Tauri `__TAURI_PLATFORM__` global when the runtime injects it;
 *  2. user-agent OS detection (works on Tauri 2.x WebViews, which do not
 *     inject the platform global, and in plain mobile browsers).
 *
 * Outside of any web context, falls back to desktop. `isTauri` is set when a
 * Tauri marker (`__TAURI_PLATFORM__`/`__TAURI_ENV__`/`__TAURI_INTERNALS__`)
 * is present.
 *
 * NOTE: Not cached – each call re-checks, allowing Tauri's async
 * initialization to complete before the value is read.
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
