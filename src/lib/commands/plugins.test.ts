import { describe, it, expect } from 'vitest';
import {
  isPluginBackendReady,
  pluginsDisable,
  pluginsEnable,
  pluginsList,
  pluginsReload,
  pluginsStatus,
} from './plugins';

// These tests exercise the module without a Tauri runtime, so the in-memory
// mock backend is used. The mock store is module-level and mutable — each test
// targets a plugin it first locates to stay order-independent.

describe('plugin command wrappers', () => {
  it('does not report a Tauri backend in a plain browser', () => {
    expect(isPluginBackendReady()).toBe(false);
  });

  it('pluginsList returns seeded plugins with status + enabled', async () => {
    const { plugins } = await pluginsList();
    expect(plugins.length).toBeGreaterThan(0);
    for (const p of plugins) {
      expect(p).toHaveProperty('id');
      expect(p).toHaveProperty('name');
      expect(p).toHaveProperty('status');
      expect(typeof p.enabled).toBe('boolean');
    }
  });

  it('pluginsEnable flips a disabled plugin to enabled', async () => {
    const { plugins } = await pluginsList();
    const target = plugins.find(p => !p.enabled);
    expect(target).toBeDefined();
    const updated = await pluginsEnable(target!.id);
    expect(updated.id).toBe(target!.id);
    expect(updated.enabled).toBe(true);
  });

  it('pluginsDisable flips an enabled plugin to disabled', async () => {
    const { plugins } = await pluginsList();
    const target = plugins.find(p => p.enabled);
    expect(target).toBeDefined();
    const updated = await pluginsDisable(target!.id);
    expect(updated.id).toBe(target!.id);
    expect(updated.enabled).toBe(false);
  });

  it('pluginsReload returns the plugin with a non-error status when healthy', async () => {
    const { plugins } = await pluginsList();
    const target = plugins.find(p => p.status !== 'error') ?? plugins[0];
    const updated = await pluginsReload(target.id);
    expect(updated.id).toBe(target.id);
    expect(['ready', 'disabled']).toContain(updated.status);
  });

  it('pluginsStatus throws plugin_not_found for unknown ids', async () => {
    await expect(pluginsStatus('does-not-exist')).rejects.toThrow(/plugin_not_found/);
  });
});
