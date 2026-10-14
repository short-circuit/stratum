import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import {
  bytesToHex,
  clearCatalogCache,
  fetchPluginCatalog,
  fetchThemeCatalog,
  getActiveThemeId,
  listInstalledThemes,
  removeInstalledTheme,
  saveInstalledTheme,
  setActiveThemeId,
  sha256Hex,
  verifySha256,
} from './pluginCatalog';

// jsdom in this workspace does not expose `window.localStorage`; provide an
// in-memory store so the catalog cache + installed-theme persistence can be
// exercised (the module already degrades gracefully without storage).
const memStore = new Map<string, string>();
beforeEach(() => {
  memStore.clear();
  vi.stubGlobal('localStorage', {
    getItem: (k: string) => memStore.get(k) ?? null,
    setItem: (k: string, v: string) => void memStore.set(k, v),
    removeItem: (k: string) => void memStore.delete(k),
    clear: () => memStore.clear(),
  });
});

// Fixture mirroring the real community catalog schema (stratum-plugins).
const PLUGIN_FIXTURE = {
  schema: 'stratum-plugins-catalog/v1',
  plugins: [
    {
      schema_version: 1,
      id: 'com.example.hello-world',
      name: 'Hello World',
      version: '0.1.0',
      description: 'Logs note metadata on save and demonstrates the pkm host API',
      author: 'Stratum Community',
      entry: 'plugin.wasm',
      permissions: ['file:read'],
      hooks: { onSave: true },
      repo: 'https://github.com/short-circuit/stratum-plugins',
      source: 'examples/hello-world',
      sha256: '78d3d3d9784e089a7827a54b3c1f0cea2c3b75a14685edc16309cebad17c2d57',
      download_url:
        'https://raw.githubusercontent.com/short-circuit/stratum-plugins/main/plugins/com.example.hello-world/plugin.wasm',
    },
  ],
};

const THEME_FIXTURE = {
  schema: 'stratum-theme-catalog/v1',
  themes: [
    {
      id: 'com.stratum.midnight-indigo',
      name: 'Midnight Indigo',
      version: '1.0.0',
      description: 'A dark indigo theme for late-night note-taking.',
      author: 'Stratum Community',
      dark_mode: true,
      primary_color: '#6366f1',
      secondary_color: '#312e81',
      font_size: 16,
      repo: 'https://github.com/short-circuit/stratum-plugins',
      source: 'themes/com.stratum.midnight-indigo',
      sha256: '0e76c0dce1ecf0fcfc7e7a4fd77ab8eb742b1c9c195d4289ed5d9386f034bca5',
      download_url:
        'https://raw.githubusercontent.com/short-circuit/stratum-plugins/main/themes/com.stratum.midnight-indigo/theme.json',
    },
  ],
};

function stubOk(payload: unknown): void {
  vi.stubGlobal('fetch', vi.fn(async () => ({
    ok: true,
    status: 200,
    json: async () => payload,
  } as Response)));
}

function stubFail(): void {
  vi.stubGlobal('fetch', vi.fn(async () => {
    throw new Error('network down');
  }));
}

beforeEach(() => {
  clearCatalogCache();
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('pluginCatalog — catalog fetch + validation', () => {
  it('parses a valid plugin catalog and returns fresh entries', async () => {
    stubOk(PLUGIN_FIXTURE);
    const result = await fetchPluginCatalog();
    expect(result.offline).toBe(false);
    expect(result.error).toBeNull();
    expect(result.entries).toHaveLength(1);
    expect(result.entries[0].id).toBe('com.example.hello-world');
    expect(result.entries[0].name).toBe('Hello World');
    expect(result.entries[0].permissions).toEqual(['file:read']);
    expect(result.fetchedAt).toBeTruthy();
  });

  it('parses a valid theme catalog', async () => {
    stubOk(THEME_FIXTURE);
    const result = await fetchThemeCatalog();
    expect(result.offline).toBe(false);
    expect(result.entries).toHaveLength(1);
    expect(result.entries[0].id).toBe('com.stratum.midnight-indigo');
    expect(result.entries[0].dark_mode).toBe(true);
  });

  it('falls back to the cached catalog when the network fails (offline)', async () => {
    // Seed the cache with a successful fetch, then take the network down.
    stubOk(PLUGIN_FIXTURE);
    await fetchPluginCatalog();
    stubFail();
    const result = await fetchPluginCatalog();
    expect(result.offline).toBe(true);
    expect(result.error).toContain('network down');
    expect(result.entries).toHaveLength(1);
    expect(result.entries[0].id).toBe('com.example.hello-world');
  });

  it('returns an error result (no throw) when offline with no cache', async () => {
    stubFail();
    const result = await fetchPluginCatalog();
    expect(result.offline).toBe(false);
    expect(result.entries).toEqual([]);
    expect(result.error).toContain('network down');
  });

  it('rejects a schema-invalid catalog payload', async () => {
    stubOk({ schema: 'stratum-theme-catalog/v1', themes: [] });
    const result = await fetchPluginCatalog();
    expect(result.error).toContain('catalog_schema_invalid');
    expect(result.entries).toEqual([]);
  });

  it('rejects an HTTP error status', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => ({
      ok: false,
      status: 404,
      json: async () => ({}),
    } as Response)));
    const result = await fetchPluginCatalog();
    expect(result.error).toContain('catalog_http_404');
  });
});

describe('pluginCatalog — sha256 verification', () => {
  it('computes a known sha256 hex digest', async () => {
    // "abc" → standard test vector.
    const digest = await sha256Hex(new TextEncoder().encode('abc'));
    expect(digest).toBe('ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad');
  });

  it('bytesToHex encodes low/high bytes correctly', () => {
    expect(bytesToHex(new Uint8Array([0x00, 0x0f, 0xff]))).toBe('000fff');
  });

  it('verifies a matching digest', async () => {
    const data = new TextEncoder().encode('abc');
    const ok = await verifySha256(data, 'BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD');
    expect(ok).toBe(true);
  });

  it('fails verification on a digest mismatch', async () => {
    const data = new TextEncoder().encode('abc');
    const ok = await verifySha256(data, 'deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef');
    expect(ok).toBe(false);
  });

  it('throws on a malformed expected digest', async () => {
    await expect(verifySha256(new TextEncoder().encode('abc'), 'not-hex')).rejects.toThrow(
      /catalog_invalid_sha256/,
    );
  });
});

describe('pluginCatalog — installed theme persistence', () => {
  it('saves, lists, and removes installed themes (idempotent by id)', () => {
    expect(listInstalledThemes()).toEqual([]);
    saveInstalledTheme(THEME_FIXTURE.themes[0]);
    expect(listInstalledThemes()).toHaveLength(1);
    saveInstalledTheme(THEME_FIXTURE.themes[0]); // re-install = no duplicate
    expect(listInstalledThemes()).toHaveLength(1);
    const remaining = removeInstalledTheme('com.stratum.midnight-indigo');
    expect(remaining).toEqual([]);
    expect(listInstalledThemes()).toEqual([]);
  });

  it('tracks the active theme id', () => {
    expect(getActiveThemeId()).toBeNull();
    setActiveThemeId('com.stratum.midnight-indigo');
    expect(getActiveThemeId()).toBe('com.stratum.midnight-indigo');
    setActiveThemeId(null);
    expect(getActiveThemeId()).toBeNull();
  });
});
