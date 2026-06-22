import { useState, useEffect } from 'react';
import * as api from '../lib/commands';
import { applyAccentTheme } from '../lib/theme';

const PROVIDERS = [
  { value: 'ollama', label: 'Ollama (Local)' },
  { value: 'openai', label: 'OpenAI' },
  { value: 'anthropic', label: 'Anthropic' },
  { value: 'google', label: 'Google AI' },
  { value: 'zai', label: 'Z.AI' },
  { value: 'custom', label: 'Custom (OpenAI-compatible)' },
];

const PRESET_COLORS = [
  '#f97316', // orange
  '#ef4444', // red
  '#3b82f6', // blue
  '#8b5cf6', // violet
  '#10b981', // emerald
  '#f59e0b', // amber
  '#ec4899', // pink
  '#06b6d4', // cyan
];

export default function SettingsPage() {
  const [settings, setSettings] = useState<any>(null);
  const [saving, setSaving] = useState(false);
  const [fetching, setFetching] = useState(false);
  const [msg, setMsg] = useState('');
  const [availableModels, setAvailableModels] = useState<string[]>([]);

  useEffect(() => {
    api.getSettings().then(s => {
      setSettings(s);
      // Apply theme from saved settings
      if (s.theme) {
        applyAccentTheme(s.theme.accent_color || '#f97316', s.theme.dark_mode);
      }
    }).catch(err => setMsg(`Load failed: ${err}`));
  }, []);

  if (!settings) {
    return <div className="p-6 text-gray-400 text-sm">Loading settings...</div>;
  }

  const ai = settings.ai;
  const theme = settings.theme || { dark_mode: true, accent_color: '#f97316', font_size: 16 };

  const updateAi = (patch: any) => {
    setSettings({ ...settings, ai: { ...ai, ...patch } });
  };

  const updateTheme = (patch: any) => {
    const newTheme = { ...theme, ...patch };
    setSettings({ ...settings, theme: newTheme });
    // Apply immediately for live preview
    applyAccentTheme(newTheme.accent_color, newTheme.dark_mode);
  };

  const handleSave = async () => {
    setSaving(true);
    setMsg('');
    try {
      await api.saveSettings(settings);
      setMsg('Saved.');
    } catch (e) {
      setMsg(`Save failed: ${e}`);
    } finally {
      setSaving(false);
    }
  };

  const handleFetchModels = async () => {
    setFetching(true);
    setMsg('');
    try {
      const models = await api.fetchModels();
      setAvailableModels(models);
      setMsg(`Found ${models.length} models`);
    } catch (e) {
      setMsg(`Fetch failed: ${e}`);
    } finally {
      setFetching(false);
    }
  };

  const toggleModelCapability = (modelName: string, cap: string) => {
    const models = [...ai.models];
    const existing = models.find((m: any) => m.name === modelName);
    if (existing) {
      if (existing.capabilities.includes(cap)) {
        existing.capabilities = existing.capabilities.filter((c: string) => c !== cap);
      } else {
        existing.capabilities.push(cap);
      }
    } else {
      models.push({ name: modelName, capabilities: [cap] });
    }
    updateAi({ models: models.filter((m: any) => m.capabilities.length > 0) });
  };

  const modelCaps = (modelName: string): string[] => {
    return ai.models.find((m: any) => m.name === modelName)?.capabilities || [];
  };

  return (
    <div className="p-6 max-w-2xl mx-auto overflow-auto">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-lg font-semibold">Settings</h2>
        <button
          onClick={handleSave}
          disabled={saving}
          className="px-4 py-1.5 bg-[var(--accent-500)] text-white text-sm rounded hover:bg-[var(--accent-600)] disabled:opacity-50"
        >
          {saving ? 'Saving...' : 'Save'}
        </button>
      </div>

      {msg && (
        <div className={`text-sm p-2 rounded mb-3 ${
          msg.startsWith('Save failed') || msg.startsWith('Fetch failed')
            ? 'bg-red-50 dark:bg-red-900/20 text-red-600'
            : 'bg-green-50 dark:bg-green-900/20 text-green-600'
        }`}>
          {msg}
        </div>
      )}

      {/* AI Section */}
      <section className="mb-6">
        <h3 className="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-3">AI Configuration</h3>

        {/* Provider */}
        <div className="mb-3">
          <label className="text-xs text-gray-500 block mb-1">Provider</label>
          <select
            value={ai.provider}
            onChange={e => updateAi({ provider: e.target.value })}
            className="w-full text-sm px-2 py-1.5 rounded border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800"
          >
            {PROVIDERS.map(p => (
              <option key={p.value} value={p.value}>{p.label}</option>
            ))}
          </select>
        </div>

        {/* Endpoint */}
        {(ai.provider === 'ollama' || ai.provider === 'custom' || ai.provider === 'zai') && (
          <div className="mb-3">
            <label className="text-xs text-gray-500 block mb-1">API Endpoint URL</label>
            <input
              type="text"
              value={ai.endpoint || ''}
              onChange={e => updateAi({ endpoint: e.target.value || null })}
              placeholder="http://localhost:11434"
              className="w-full text-sm px-2 py-1.5 rounded border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800"
            />
          </div>
        )}

        {/* API Key */}
        <div className="mb-3">
          <label className="text-xs text-gray-500 block mb-1">API Key</label>
          <input
            type="password"
            value={ai.api_key || ''}
            onChange={e => updateAi({ api_key: e.target.value || null })}
            placeholder="sk-..."
            className="w-full text-sm px-2 py-1.5 rounded border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 font-mono"
          />
        </div>

        {/* Default Chat Model */}
        <div className="mb-3">
          <label className="text-xs text-gray-500 block mb-1">Default Chat Model</label>
          <input
            type="text"
            value={ai.model}
            onChange={e => updateAi({ model: e.target.value })}
            placeholder="gpt-4o"
            className="w-full text-sm px-2 py-1.5 rounded border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800"
          />
        </div>

        {/* Fetch Models */}
        <div className="mb-3">
          <button
            onClick={handleFetchModels}
            disabled={fetching}
            className="text-sm px-3 py-1.5 bg-gray-100 dark:bg-gray-700 rounded hover:bg-gray-200 dark:hover:bg-gray-600 disabled:opacity-50"
          >
            {fetching ? 'Fetching...' : 'Fetch Available Models'}
          </button>
        </div>

        {/* Model list with capabilities */}
        {availableModels.length > 0 && (
          <div className="mb-3">
            <label className="text-xs text-gray-500 block mb-1">Available Models (click to enable capabilities)</label>
            <div className="space-y-1 max-h-64 overflow-auto border border-gray-200 dark:border-gray-700 rounded">
              {availableModels.map(m => {
                const caps = modelCaps(m);
                return (
                  <div key={m} className="flex items-center px-3 py-1.5 text-sm hover:bg-gray-50 dark:hover:bg-gray-800/50">
                    <span className="flex-1 truncate font-mono text-xs">{m}</span>
                    <div className="flex gap-1">
                      {['chat', 'embedding', 'tts'].map(cap => (
                        <button
                          key={cap}
                          onClick={() => toggleModelCapability(m, cap)}
                          className={`text-xs px-1.5 py-0.5 rounded ${
                            caps.includes(cap)
                              ? 'bg-[var(--accent-500)] text-white'
                              : 'bg-gray-100 dark:bg-gray-700 text-gray-500'
                          }`}
                        >
                          {cap}
                        </button>
                      ))}
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        )}

        {/* RAG settings */}
        <div className="mb-3 flex items-center gap-4">
          <label className="flex items-center gap-2 text-sm">
            <input
              type="checkbox"
              checked={ai.rag_enabled}
              onChange={e => updateAi({ rag_enabled: e.target.checked })}
              className="rounded"
            />
            Enable RAG
          </label>
          {ai.rag_enabled && (
            <label className="flex items-center gap-1 text-sm">
              Chunks:
              <input
                type="number"
                value={ai.rag_chunk_count}
                onChange={e => updateAi({ rag_chunk_count: parseInt(e.target.value) || 5 })}
                className="w-16 text-sm px-2 py-0.5 rounded border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800"
                min={1}
                max={20}
              />
            </label>
          )}
        </div>
      </section>

      {/* Theme Section */}
      <section className="mb-6 pt-4 border-t border-gray-200 dark:border-gray-700">
        <h3 className="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-3">Theme</h3>

        {/* Dark mode */}
        <div className="mb-3">
          <label className="flex items-center gap-2 text-sm">
            <input
              type="checkbox"
              checked={theme.dark_mode}
              onChange={e => updateTheme({ dark_mode: e.target.checked })}
              className="rounded"
            />
            Dark mode
          </label>
        </div>

        {/* Accent color */}
        <div className="mb-3">
          <label className="text-xs text-gray-500 block mb-1">Accent color</label>
          <div className="flex gap-1.5 flex-wrap">
            {PRESET_COLORS.map(color => (
              <button
                key={color}
                onClick={() => updateTheme({ accent_color: color })}
                className={`w-7 h-7 rounded-full border-2 transition-transform ${
                  theme.accent_color === color
                    ? 'border-gray-800 dark:border-white scale-110'
                    : 'border-transparent hover:scale-105'
                }`}
                style={{ backgroundColor: color }}
                title={color}
              />
            ))}
          </div>
          <div className="flex items-center gap-2 mt-2">
            <input
              type="color"
              value={theme.accent_color}
              onChange={e => updateTheme({ accent_color: e.target.value })}
              className="w-8 h-8 rounded cursor-pointer border-0 p-0"
            />
            <input
              type="text"
              value={theme.accent_color}
              onChange={e => updateTheme({ accent_color: e.target.value })}
              className="flex-1 text-xs px-2 py-1 rounded border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 font-mono"
              placeholder="#f97316"
            />
          </div>
        </div>
      </section>

      {/* Vault Section */}
      <section className="mb-6 pt-4 border-t border-gray-200 dark:border-gray-700">
        <h3 className="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-3">Vault</h3>
        <div className="mb-3">
          <label className="text-xs text-gray-500 block mb-1">Vault Path</label>
          <input
            type="text"
            value={settings.vault_path}
            onChange={e => setSettings({ ...settings, vault_path: e.target.value })}
            className="w-full text-sm px-2 py-1.5 rounded border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 font-mono"
          />
        </div>
      </section>
    </div>
  );
}
