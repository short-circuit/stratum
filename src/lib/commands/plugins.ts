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
// contract, implemented in src-tauri/src/commands/plugins.rs). The backend
// integration lands with E3, so until `invoke` is available/wired we fall
// back to an in-memory mock so the UI is usable and testable standalone.

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
  },
  {
    id: 'daily-summary',
    name: 'Daily Summary',
    version: '0.2.1',
    status: 'disabled',
    enabled: false,
    permissions: ['file:read', 'file:write'],
  },
  {
    id: 'broken-example',
    name: 'Broken Example',
    version: '0.0.4',
    status: 'error',
    enabled: true,
    permissions: ['file:read'],
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
