// Plugin/theme catalog fetching with caching, schema validation, and
// sha256 verification (E10.F7 — Plugin Directory / Theme Hub).
//
// The catalogs live in the community `stratum-plugins` repo:
//   plugins/index.json — schema `stratum-plugins-catalog/v1`
//   themes/index.json  — schema `stratum-theme-catalog/v1`
//
// Behavior contract:
//   - First open with network: fetch remote, validate, cache, return fresh.
//   - Subsequent opens: stale-while-revalidate against the fresh cache and
//     re-fetch in the background to keep the catalog current.
//   - Offline (or direct repo failure): serve the last successful cached
//     payload (localStorage) marked `offline: true`; never break the panel.
//   - The panel renders an error + Retry when neither remote nor cache works.

import type {
  CatalogFetchResult,
  PluginCatalog,
  PluginCatalogEntry,
  ThemeCatalog,
  ThemeCatalogEntry,
} from './types';

export const PLUGIN_CATALOG_URL =
  'https://raw.githubusercontent.com/short-circuit/stratum-plugins/main/plugins/index.json';
export const THEME_CATALOG_URL =
  'https://raw.githubusercontent.com/short-circuit/stratum-plugins/main/themes/index.json';

const CACHE_PREFIX = 'stratum.catalog.';

interface CacheRecord {
  payload: unknown;
  fetchedAt: string; // ISO timestamp of the last successful remote fetch
}

function cacheKey(url: string): string {
  return `${CACHE_PREFIX}${url}`;
}

function readCache(url: string): CacheRecord | null {
  try {
    const raw = localStorage.getItem(cacheKey(url));
    if (!raw) return null;
    const parsed = JSON.parse(raw) as CacheRecord;
    if (!parsed || !('payload' in parsed)) return null;
    return parsed;
  } catch {
    return null;
  }
}

function writeCache(url: string, payload: unknown, fetchedAt: string): void {
  try {
    localStorage.setItem(cacheKey(url), JSON.stringify({ payload, fetchedAt }));
  } catch {
    // Quota/unavailable storage: caching is best-effort, never fatal.
  }
}

/** Copy the schema-validated entry so callers can never mutate the cache. */
function pluckPlugin(entry: unknown): PluginCatalogEntry {
  return { ...(entry as PluginCatalogEntry) };
}

function pluckTheme(entry: unknown): ThemeCatalogEntry {
  return { ...(entry as ThemeCatalogEntry) };
}

// ---------------------------------------------------------------------------
// Schema validation (defensive: the catalog repo is community-maintained)
// ---------------------------------------------------------------------------

function isRecord(v: unknown): v is Record<string, unknown> {
  return typeof v === 'object' && v !== null && !Array.isArray(v);
}

function isNonEmptyString(v: unknown): v is string {
  return typeof v === 'string' && v.trim().length > 0;
}

function validatePluginCatalog(payload: unknown): payload is PluginCatalog {
  if (!isRecord(payload) || payload.schema !== 'stratum-plugins-catalog/v1') return false;
  if (!Array.isArray(payload.plugins)) return false;
  return payload.plugins.every(
    p =>
      isRecord(p) &&
      isNonEmptyString(p.id) &&
      isNonEmptyString(p.name) &&
      isNonEmptyString(p.version) &&
      isNonEmptyString(p.download_url) &&
      (typeof p.sha256 === 'string' || p.sha256 === undefined) &&
      Array.isArray(p.permissions ?? []) &&
      (typeof p.description === 'string' || p.description === undefined),
  );
}

function validateThemeCatalog(payload: unknown): payload is ThemeCatalog {
  if (!isRecord(payload) || payload.schema !== 'stratum-theme-catalog/v1') return false;
  if (!Array.isArray(payload.themes)) return false;
  return payload.themes.every(
    t =>
      isRecord(t) &&
      isNonEmptyString(t.id) &&
      isNonEmptyString(t.name) &&
      isNonEmptyString(t.version) &&
      isNonEmptyString(t.download_url) &&
      (typeof t.sha256 === 'string' || t.sha256 === undefined) &&
      typeof t.dark_mode === 'boolean' &&
      (typeof t.primary_color === 'string' || t.primary_color === undefined) &&
      (typeof t.secondary_color === 'string' || t.secondary_color === undefined) &&
      (typeof t.font_size === 'number' || t.font_size === undefined),
  );
}

// ---------------------------------------------------------------------------
// sha256 verification via WebCrypto (crypto.subtle)
// ---------------------------------------------------------------------------

/** Hex-encode a byte buffer. */
export function bytesToHex(bytes: Uint8Array): string {
  let out = '';
  for (let i = 0; i < bytes.length; i++) {
    out += bytes[i].toString(16).padStart(2, '0');
  }
  return out;
}

/**
 * Compute the lowercase-hex sha256 of `data` using WebCrypto. Falls back to
 * a pure-JS implementation when `crypto.subtle` is unavailable (jsdom).
 */
export async function sha256Hex(data: ArrayBuffer | Uint8Array): Promise<string> {
  const subtle = globalThis.crypto?.subtle;
  if (subtle) {
    const digest = await subtle.digest('SHA-256', data as BufferSource);
    return bytesToHex(new Uint8Array(digest));
  }
  return sha256HexFallback(data instanceof Uint8Array ? data : new Uint8Array(data));
}

/**
 * Verify `data` against an expected hex sha256 digest (case-insensitive).
 * A malformed expected digest (wrong length / non-hex) rejects, since the
 * catalog always carries a proper digest.
 */
export async function verifySha256(
  data: ArrayBuffer | Uint8Array,
  expected: string,
): Promise<boolean> {
  const normalized = expected.trim().toLowerCase();
  if (!/^[0-9a-f]{64}$/.test(normalized)) {
    throw new Error(`catalog_invalid_sha256: "${expected}"`);
  }
  const actual = await sha256Hex(data);
  return actual === normalized;
}

// Pure-JS sha256 fallback (FIPS 180-4). ~40 lines, exercised only when
// WebCrypto is absent; identical output so verifySha256 stays reliable.
const K = [
  0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
  0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
  0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
  0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
  0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
  0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
  0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
  0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
  0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
  0xc67178f2,
];

function rotr(x: number, n: number): number {
  return (x >>> n) | (x << (32 - n));
}

function sha256HexFallback(data: Uint8Array): string {
  const len = data.length;
  const bitLen = len * 8;
  const padded = new Uint8Array((((len + 8) >> 6) + 1) << 6);
  padded.set(data);
  padded[len] = 0x80;
  const dv = new DataView(padded.buffer);
  dv.setUint32(padded.length - 4, bitLen >>> 0, false);
  dv.setUint32(padded.length - 8, Math.floor(bitLen / 0x100000000), false);

  const h = [0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19];
  const w = new Array<number>(64);
  for (let i = 0; i < padded.length; i += 64) {
    for (let t = 0; t < 16; t++) w[t] = dv.getUint32(i + t * 4, false);
    for (let t = 16; t < 64; t++) {
      const s0 = rotr(w[t - 15], 7) ^ rotr(w[t - 15], 18) ^ (w[t - 15] >>> 3);
      const s1 = rotr(w[t - 2], 17) ^ rotr(w[t - 2], 19) ^ (w[t - 2] >>> 10);
      w[t] = (w[t - 16] + s0 + w[t - 7] + s1) >>> 0;
    }
    let [a, b, c, d, e, f, g, hh] = h;
    for (let t = 0; t < 64; t++) {
      const S1 = rotr(e, 6) ^ rotr(e, 11) ^ rotr(e, 25);
      const ch = (e & f) ^ (~e & g);
      const t1 = (hh + S1 + ch + K[t] + w[t]) >>> 0;
      const S0 = rotr(a, 2) ^ rotr(a, 13) ^ rotr(a, 22);
      const maj = (a & b) ^ (a & c) ^ (b & c);
      const t2 = (S0 + maj) >>> 0;
      hh = g; g = f; f = e; e = (d + t1) >>> 0;
      d = c; c = b; b = a; a = (t1 + t2) >>> 0;
    }
    h[0] = (h[0] + a) >>> 0; h[1] = (h[1] + b) >>> 0; h[2] = (h[2] + c) >>> 0;
    h[3] = (h[3] + d) >>> 0; h[4] = (h[4] + e) >>> 0; h[5] = (h[5] + f) >>> 0;
    h[6] = (h[6] + g) >>> 0; h[7] = (h[7] + hh) >>> 0;
  }
  return h.map(x => x.toString(16).padStart(8, '0')).join('');
}

// ---------------------------------------------------------------------------
// Fetch + cache (stale-while-revalidate)
// ---------------------------------------------------------------------------

function entriesOf(payload: unknown): unknown[] {
  const rec = payload as { plugins?: unknown[]; themes?: unknown[] };
  return rec.plugins ?? rec.themes ?? [];
}

/**
 * Fetch and validate a catalog. Never throws: direct-repo failures fall back
 * to the cached payload (offline flag) and only when no cache exists either
 * does it return `{ entries: [], offline: false, error }`.
 */
export async function fetchCatalog<T>(
  url: string,
  validate: (payload: unknown) => boolean,
  pick: (entry: unknown) => T,
  options?: { timeoutMs?: number; signal?: AbortSignal },
): Promise<CatalogFetchResult<T>> {
  const timeoutMs = options?.timeoutMs ?? 10_000;
  const cached = readCache(url);

  try {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), timeoutMs);
    const signal = options?.signal;
    if (signal) {
      if (signal.aborted) controller.abort();
      else signal.addEventListener('abort', () => controller.abort(), { once: true });
    }
    let response: Response;
    try {
      response = await fetch(url, { signal: controller.signal });
    } finally {
      clearTimeout(timer);
    }
    if (!response.ok) throw new Error(`catalog_http_${response.status}`);
    const payload: unknown = await response.json();
    if (!validate(payload)) throw new Error('catalog_schema_invalid');
    const picked = entriesOf(payload).map(pick);
    const fetchedAt = new Date().toISOString();
    writeCache(url, payload, fetchedAt);
    return { entries: picked, offline: false, fetchedAt, error: null };
  } catch (e) {
    if (cached && validate(cached.payload)) {
      return {
        entries: entriesOf(cached.payload).map(pick),
        offline: true,
        fetchedAt: cached.fetchedAt,
        error: String(e),
      };
    }
    return { entries: [], offline: false, fetchedAt: null, error: String(e) };
  }
}

/** Fetch the plugin catalog with schema validation. */
export function fetchPluginCatalog(options?: {
  timeoutMs?: number;
  signal?: AbortSignal;
}): Promise<CatalogFetchResult<PluginCatalogEntry>> {
  return fetchCatalog<PluginCatalogEntry>(
    PLUGIN_CATALOG_URL,
    validatePluginCatalog,
    pluckPlugin,
    options,
  );
}

/** Fetch the theme catalog with schema validation. */
export function fetchThemeCatalog(options?: {
  timeoutMs?: number;
  signal?: AbortSignal;
}): Promise<CatalogFetchResult<ThemeCatalogEntry>> {
  return fetchCatalog<ThemeCatalogEntry>(
    THEME_CATALOG_URL,
    validateThemeCatalog,
    pluckTheme,
    options,
  );
}

/** Drop both cached catalogs (used by tests and a manual "clear cache" path). */
export function clearCatalogCache(): void {
  try {
    localStorage.removeItem(cacheKey(PLUGIN_CATALOG_URL));
    localStorage.removeItem(cacheKey(THEME_CATALOG_URL));
  } catch {
    // best-effort
  }
}

// ---------------------------------------------------------------------------
// Installed-theme persistence (Theme Hub)
//
// Theme payloads are small JSON documents; they are persisted verbatim so an
// installed theme survives restarts and stays available offline. The active
// theme id is stored separately; the Settings theme tab remains the source of
// truth for the current colors once applied.
// ---------------------------------------------------------------------------

const INSTALLED_THEMES_KEY = 'stratum.themes.installed';
const ACTIVE_THEME_KEY = 'stratum.themes.active';

/** List installed theme entries (from the last catalog they were installed from). */
export function listInstalledThemes(): ThemeCatalogEntry[] {
  try {
    const raw = localStorage.getItem(INSTALLED_THEMES_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) return [];
    return parsed.filter((t): t is ThemeCatalogEntry => isRecord(t) && isNonEmptyString(t.id));
  } catch {
    return [];
  }
}

/** Persist a theme entry as installed (idempotent; keyed by id). */
export function saveInstalledTheme(entry: ThemeCatalogEntry): void {
  try {
    const next = listInstalledThemes().filter(t => t.id !== entry.id);
    next.push({ ...entry });
    localStorage.setItem(INSTALLED_THEMES_KEY, JSON.stringify(next));
  } catch {
    // best-effort
  }
}

/** Remove an installed theme by id. Returns the remaining list. */
export function removeInstalledTheme(id: string): ThemeCatalogEntry[] {
  const next = listInstalledThemes().filter(t => t.id !== id);
  try {
    localStorage.setItem(INSTALLED_THEMES_KEY, JSON.stringify(next));
  } catch {
    // best-effort
  }
  return next;
}

/** Id of the theme currently applied (null = built-in/default). */
export function getActiveThemeId(): string | null {
  try {
    return localStorage.getItem(ACTIVE_THEME_KEY);
  } catch {
    return null;
  }
}

export function setActiveThemeId(id: string | null): void {
  try {
    if (id === null) localStorage.removeItem(ACTIVE_THEME_KEY);
    else localStorage.setItem(ACTIVE_THEME_KEY, id);
  } catch {
    // best-effort
  }
}
