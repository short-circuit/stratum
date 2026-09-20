import { invoke } from '@tauri-apps/api/core';
import type {
  PluginHttpRequestDto,
  PluginHttpRequestParams,
  PluginInfoDto,
  PluginListResultDto,
  PluginNoteReadDto,
} from '../types';

// WASM plugin Tauri command wrappers.
//
// These call the commands defined by docs/advanced/plugins.md §9 (normative
// contract, implemented in src-tauri/src/commands/plugins.rs). When the Tauri
// backend is unavailable (e.g. running the web UI outside the desktop shell),
// `callOrMock` falls back to an in-memory mock so the UI remains usable and
// testable standalone.

const BACKEND_READY: boolean = (() => {
  try {
    return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
  } catch {
    return false;
  }
})();

export function isPluginBackendReady(): boolean {
  return BACKEND_READY;
}

// ─────────────────────────────────────────────────────────────────────────────
// Mock dataset. Mirrors PluginInfoDto; the `enabled` bit and `status` field are
// kept consistent (enabled=false <=> status 'disabled'). Errors surface as
// status 'error' with an `error` message, matching the runtime failure envelope.
// ─────────────────────────────────────────────────────────────────────────────

const MOCK_PLUGINS: PluginInfoDto[] = [
  {
    id: 'dev-dashboard',
    name: 'Developer Dashboard',
    version: '0.1.0',
    status: 'ready',
    enabled: true,
    permissions: ['file:read', 'network'],
    hooks: ['onSave'],
    author: 'stratum-team',
    description: 'Collects development telemetry and posts it to a local endpoint.',
  },
  {
    id: 'daily-summary',
    name: 'Daily Summary',
    version: '0.2.1',
    status: 'disabled',
    enabled: false,
    permissions: ['file:read', 'file:write'],
    hooks: ['onOpen', 'onSave'],
    author: 'acme-lab',
    description: 'Summarizes the day\u2019s notes into a single digest page.',
  },
  {
    id: 'broken-example',
    name: 'Broken Example',
    version: '0.0.4',
    status: 'error',
    enabled: true,
    permissions: ['file:read'],
    hooks: [],
    author: '',
    description: '',
    error: "runtime_error: import 'pkm.note_write' not found (plugin compiled with a different ABI)",
  },
];

let mockStore: Record<string, PluginInfoDto> = Object.fromEntries(
  MOCK_PLUGINS.map(p => [p.id, { ...p }]),
);

/** Synchronize the `enabled` ↔ `status` invariant for the mock store. */
function rebuildMockStatus(list: PluginInfoDto[]): PluginInfoDto[] {
  for (const p of list) {
    if (!p.enabled && p.status !== 'error') p.status = 'disabled';
    if (p.enabled && p.status === 'disabled') p.status = 'ready';
  }
  return list;
}

function applyPluginMutation(id: string, enabled: boolean): PluginInfoDto {
  const p = mockStore[id];
  if (!p) throw new Error(`plugin_not_found: no plugin with id '${id}'`);
  p.enabled = enabled;
  mockStore = rebuildMockStatus(Object.values(mockStore)).reduce<Record<string, PluginInfoDto>>(
    (acc, v) => { acc[v.id] = v; return acc; },
    {},
  );
  return mockStore[p.id];
}

const IS_TAURI = BACKEND_READY;

/** Attempt the real Tauri command; fall back to the mock when unavailable. */
async function callOrMock<T>(command: string, args: Record<string, unknown>, mock: () => T): Promise<T> {
  if (IS_TAURI) {
    try {
      return await invoke<T>(command, args);
    } catch (e) {
      // Command not implemented on the Rust side yet — fall through to mock.
      const msg = String(e);
      if (/not (found|implemented)|unknown command/i.test(msg)) return mock();
      throw e;
    }
  }
  return mock();
}

export async function pluginsList(): Promise<PluginListResultDto> {
  return callOrMock('plugins_list', {}, () => {
    // Return a copy so callers can't mutate the module-level mock.
    return { plugins: rebuildMockStatus(Object.values(mockStore)).map(p => ({ ...p })) };
  });
}

export async function pluginsEnable(id: string): Promise<PluginInfoDto> {
  return callOrMock('plugins_enable', { id }, () => applyPluginMutation(id, true));
}

export async function pluginsDisable(id: string): Promise<PluginInfoDto> {
  return callOrMock('plugins_disable', { id }, () => applyPluginMutation(id, false));
}

export async function pluginsReload(id: string): Promise<PluginInfoDto> {
  return callOrMock('plugins_reload', { id }, () => {
    const p = mockStore[id];
    if (!p) throw new Error(`plugin_not_found: no plugin with id '${id}'`);
    return { ...p, status: p.enabled ? 'ready' : 'disabled', error: null };
  });
}

export async function pluginsStatus(id: string): Promise<PluginInfoDto> {
  return callOrMock('plugins_status', { id }, () => {
    const p = mockStore[id];
    if (!p) throw new Error(`plugin_not_found: no plugin with id '${id}'`);
    return { ...p, status: p.enabled ? 'ready' : 'disabled' };
  });
}

export async function pluginsInstall(path: string): Promise<PluginInfoDto> {
  return callOrMock(
    'plugins_install',
    { path },
    () => {
      // The mock treats install as adding a synthetic plugin keyed by the
      // path's basename so the list round-trips inside the UI.
      const id = `installed-${path.split(/[\\/]/).pop() ?? 'plugin'}`.replace(/\.wasm$/i, '');
      const existing = mockStore[id];
      const added: PluginInfoDto = existing ?? {
        id,
        name: id,
        version: '0.1.0',
        status: 'disabled',
        enabled: false,
        permissions: [],
      };
      mockStore = { ...mockStore, [id]: { ...added, status: 'disabled', enabled: false } };
      return mockStore[id];
    },
  );
}

export async function pluginsUninstall(id: string): Promise<PluginListResultDto> {
  return callOrMock('plugins_uninstall', { id }, () => {
    const next = Object.values(mockStore).filter(p => p.id !== id);
    mockStore = Object.fromEntries(next.map(p => [p.id, { ...p }]));
    return { plugins: rebuildMockStatus(next.map(p => ({ ...p }))) };
  });
}

/**
 * Open the OS file picker for a `.wasm` plugin, then install it. Desktop-only;
 * outside the Tauri shell (tests / plain browser) it falls back to a mock
 * install keyed by a prompt-provided name so the UI remains testable.
 */
export async function pluginsInstallFromFile(): Promise<PluginInfoDto | null> {
  if (!BACKEND_READY) {
    const name = window.prompt?.('Plugin .wasm file name (mock install)') ?? null;
    if (!name) return null;
    return pluginsInstall(name);
  }
  const { open } = await import('@tauri-apps/plugin-dialog');
  const selection = await open({
    multiple: false,
    filters: [{ name: 'WASM plugin', extensions: ['wasm'] }],
  });
  if (!selection) return null;
  const path = Array.isArray(selection) ? selection[0] : selection;
  return pluginsInstall(path);
}

export async function pluginNoteRead(path: string): Promise<PluginNoteReadDto> {
  return callOrMock('plugin_note_read', { path }, async () => {
    const mtime = new Date().toISOString();
    return { path, content: `# ${path}\n\n(mocked plugin note read)`, mtime };
  });
}

export async function pluginHttpRequest(params: PluginHttpRequestParams): Promise<PluginHttpRequestDto> {
  return callOrMock('plugin_http_request', { ...params }, async () => {
    return {
      status: 200,
      headers: [['content-type', 'application/json']],
      body: JSON.stringify({ mocked: true, url: params.url }),
    };
  });
}
