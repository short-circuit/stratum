/**
 * WebGL / constrained-device detection for the 3D graph view.
 *
 * The 3D graph (react-force-graph-3d → three.js) requires a WebGL context and
 * is GPU/memory heavy. This module decides whether the current device can
 * render it and, when it cannot, reports a reason so the UI can fall back to
 * the 2D force layout with a user-visible explanation. Desktop always renders
 * 3D regardless of this module; the mobile variant is the only consumer.
 */

export type Graph3dSupportReason = 'ok' | 'no-webgl' | 'low-end' | 'device-unknown';

export interface Graph3dSupport {
  /** Whether 3D rendering is viable on this device. */
  supported: boolean;
  /** Why 3D was (or wasn't) deemed viable. `'ok'` when supported. */
  reason: Graph3dSupportReason;
}

/**
 * True when a WebGL context (2 or 1) can be created on the current page.
 * Returns false in environments that lack WebGL (e.g. jsdom, some headless
 * contexts, or hardware-accelerated rendering being disabled).
 */
function webglAvailable(): boolean {
  const canvas = document.createElement('canvas');
  for (const name of ['webgl2', 'webgl', 'experimental-webgl'] as const) {
    try {
      const ctx = canvas.getContext(name);
      if (ctx) return true;
    } catch {
      // Some engines throw when asked to create a context; treat as unavailable.
    }
  }
  return false;
}

/**
 * True on very constrained devices: ≤2 GB of RAM (exposed as `deviceMemory`
 * in Chrome/WebView in GiB) or ≤2 hardware cores. Conservative on purpose so
 * modern mid-range phones are NOT misclassified; `deviceMemory` is undefined
 * on iOS Safari/WebKit, in which case only the core count is considered.
 */
function constrainedHardware(): boolean {
  const nav = navigator as Navigator & { deviceMemory?: number };
  if (typeof nav.deviceMemory === 'number' && nav.deviceMemory <= 2) return true;
  if (typeof nav.hardwareConcurrency === 'number' && nav.hardwareConcurrency <= 2) return true;
  return false;
}

/**
 * Detect whether the current device can render the 3D graph. Never throws —
 * an unexpected failure is reported as an unsupported device so the caller can
 * fall back to 2D instead of crashing.
 */
export function detectGraph3dSupport(): Graph3dSupport {
  try {
    if (typeof document !== 'undefined' && !webglAvailable()) {
      return { supported: false, reason: 'no-webgl' };
    }
    if (constrainedHardware()) {
      return { supported: false, reason: 'low-end' };
    }
    return { supported: true, reason: 'ok' };
  } catch {
    return { supported: false, reason: 'device-unknown' };
  }
}
