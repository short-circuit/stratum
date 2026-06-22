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
  '#f97316', '#ef4444', '#3b82f6', '#8b5cf6',
  '#10b981', '#f59e0b', '#ec4899', '#06b6d4',
];

type Tab = 'vault' | 'theme' | 'ai';

export default function SettingsPage() {
  const [settings, setSettings] = useState<any>(null);
  const [saving, setSaving] = useState(false);
  const [fetching, setFetching] = useState(false);
  const [msg, setMsg] = useState('');
  const [availableModels, setAvailableModels] = useState<string[]>([]);
  const [tab, setTab] = useState<Tab>('vault');

  useEffect(() => {
    api.getSettings().then(s => {
      setSettings(s);
      if (s.theme) {
        applyAccentTheme(s.theme.accent_color || '#f97316', s.theme.dark_mode);
      }
    }).catch(err => setMsg(`Load failed: ${err}`));
  }, []);

  if (!settings) {
    return <div className="p-6 text-neutral-400 text-sm">Loading settings...</div>;
  }

  const ai = settings.ai;
  const theme = settings.theme || { dark_mode: true, accent_color: '#f97316', font_size: 16 };

  const updateAi = (patch: any) => setSettings({ ...settings, ai: { ...ai, ...patch } });
  const updateVault = (patch: any) => setSettings({ ...settings, ...patch });

  const updateTheme = (patch: any) => {
    const newTheme = { ...theme, ...patch };
    setSettings({ ...settings, theme: newTheme });
    applyAccentTheme(newTheme.accent_color, newTheme.dark_mode);
  };

  const handleSave = async () => {
    setSaving(true); setMsg('');
    try { await api.saveSettings(settings); setMsg('Saved.'); }
    catch (e) { setMsg(`Save failed: ${e}`); }
    finally { setSaving(false); }
  };

  const handleFetchModels = async () => {
    setFetching(true); setMsg('');
    try { const models = await api.fetchModels(); setAvailableModels(models); setMsg(`Found ${models.length} models`); }
    catch (e) { setMsg(`Fetch failed: ${e}`); }
    finally { setFetching(false); }
  };

  const toggleModelCapability = (modelName: string, cap: string) => {
    const models = [...ai.models];
    const existing = models.find((m: any) => m.name === modelName);
    if (existing) {
      existing.capabilities = existing.capabilities.includes(cap)
        ? existing.capabilities.filter((c: string) => c !== cap)
        : [...existing.capabilities, cap];
    } else {
      models.push({ name: modelName, capabilities: [cap] });
    }
    updateAi({ models: models.filter((m: any) => m.capabilities.length > 0) });
  };

  const modelCaps = (name: string) => ai.models.find((m: any) => m.name === name)?.capabilities || [];

  const tabs: { id: Tab; label: string }[] = [
    { id: 'vault', label: 'Vault' },
    { id: 'theme', label: 'Theme' },
    { id: 'ai', label: 'AI' },
  ];

  return (
    <div className="flex flex-col h-full">
      {/* Tab bar */}
      <div className="flex items-center border-b border-neutral-200 dark:border-neutral-700 shrink-0">
        <nav className="flex">
          {tabs.map(t => (
            <button
              key={t.id}
              onClick={() => setTab(t.id)}
              className={`px-4 py-2 text-sm font-medium transition-colors border-b-2 -mb-px ${
                tab === t.id
                  ? 'border-[var(--accent-500)] text-[var(--accent-600)] dark:text-[var(--accent-400)]'
                  : 'border-transparent text-neutral-500 hover:text-neutral-700 dark:hover:text-neutral-300'
              }`}
            >
              {t.label}
            </button>
          ))}
        </nav>
        <div className="flex-1" />
        <button
          onClick={handleSave}
          disabled={saving}
          className="mr-3 px-4 py-1.5 bg-[var(--accent-500)] text-white text-sm rounded hover:bg-[var(--accent-600)] disabled:opacity-50"
        >
          {saving ? 'Saving...' : 'Save'}
        </button>
      </div>

      {/* Message */}
      {msg && (
        <div className={`mx-6 mt-3 text-sm p-2 rounded ${
          msg.startsWith('Save failed') || msg.startsWith('Fetch failed')
            ? 'bg-red-50 dark:bg-red-900/20 text-red-600'
            : 'bg-green-50 dark:bg-green-900/20 text-green-600'
        }`}>
          {msg}
        </div>
      )}

      {/* Tab content */}
      <div className="flex-1 overflow-auto p-6">
        {tab === 'vault' && (
          <section>
            <h3 className="text-sm font-semibold text-neutral-700 dark:text-neutral-300 mb-3">Vault</h3>
            <div className="mb-3">
              <label className="text-xs text-neutral-500 block mb-1">Vault Path</label>
              <input
                type="text"
                value={settings.vault_path}
                onChange={e => updateVault({ vault_path: e.target.value })}
                className="w-full max-w-md text-sm px-2 py-1.5 rounded border border-neutral-300 dark:border-neutral-600 bg-white dark:bg-neutral-800 font-mono"
              />
            </div>
          </section>
        )}

        {tab === 'theme' && (
          <section>
            <h3 className="text-sm font-semibold text-neutral-700 dark:text-neutral-300 mb-3">Theme</h3>
            <div className="mb-3">
              <label className="flex items-center gap-2 text-sm">
                <input type="checkbox" checked={theme.dark_mode} onChange={e => updateTheme({ dark_mode: e.target.checked })} className="rounded" />
                Dark mode
              </label>
            </div>
            <div className="mb-3">
              <label className="text-xs text-neutral-500 block mb-1">Accent color</label>
              <div className="flex gap-1.5 flex-wrap mb-2">
                {PRESET_COLORS.map(color => (
                  <button
                    key={color}
                    onClick={() => updateTheme({ accent_color: color })}
                    className={`w-7 h-7 rounded-full border-2 transition-transform ${
                      theme.accent_color === color ? 'border-neutral-800 dark:border-white scale-110' : 'border-transparent hover:scale-105'
                    }`}
                    style={{ backgroundColor: color }}
                    title={color}
                  />
                ))}
              </div>
              <div className="flex items-center gap-2">
                <input type="color" value={theme.accent_color} onChange={e => updateTheme({ accent_color: e.target.value })} className="w-8 h-8 rounded cursor-pointer border-0 p-0" />
                <input type="text" value={theme.accent_color} onChange={e => updateTheme({ accent_color: e.target.value })} className="flex-1 max-w-[180px] text-xs px-2 py-1 rounded border border-neutral-300 dark:border-neutral-600 bg-white dark:bg-neutral-800 font-mono" placeholder="#f97316" />
              </div>
            </div>
          </section>
        )}

        {tab === 'ai' && (
          <section>
            <h3 className="text-sm font-semibold text-neutral-700 dark:text-neutral-300 mb-3">AI Configuration</h3>

            <div className="mb-3">
              <label className="text-xs text-neutral-500 block mb-1">Provider</label>
              <select
                value={ai.provider}
                onChange={e => updateAi({ provider: e.target.value })}
                className="w-full max-w-md text-sm px-2 py-1.5 rounded border border-neutral-300 dark:border-neutral-600 bg-white dark:bg-neutral-800"
              >
                {PROVIDERS.map(p => <option key={p.value} value={p.value}>{p.label}</option>)}
              </select>
            </div>

            {(ai.provider === 'ollama' || ai.provider === 'custom' || ai.provider === 'zai') && (
              <div className="mb-3">
                <label className="text-xs text-neutral-500 block mb-1">API Endpoint URL</label>
                <input type="text" value={ai.endpoint || ''} onChange={e => updateAi({ endpoint: e.target.value || null })} placeholder="http://localhost:11434" className="w-full max-w-md text-sm px-2 py-1.5 rounded border border-neutral-300 dark:border-neutral-600 bg-white dark:bg-neutral-800" />
              </div>
            )}

            <div className="mb-3">
              <label className="text-xs text-neutral-500 block mb-1">API Key</label>
              <input type="password" value={ai.api_key || ''} onChange={e => updateAi({ api_key: e.target.value || null })} placeholder="sk-..." className="w-full max-w-md text-sm px-2 py-1.5 rounded border border-neutral-300 dark:border-neutral-600 bg-white dark:bg-neutral-800 font-mono" />
            </div>

            <div className="mb-3">
              <label className="text-xs text-neutral-500 block mb-1">Default Chat Model</label>
              <input type="text" value={ai.model} onChange={e => updateAi({ model: e.target.value })} placeholder="gpt-4o" className="w-full max-w-md text-sm px-2 py-1.5 rounded border border-neutral-300 dark:border-neutral-600 bg-white dark:bg-neutral-800" />
            </div>

            <div className="mb-3">
              <button onClick={handleFetchModels} disabled={fetching} className="text-sm px-3 py-1.5 bg-neutral-100 dark:bg-neutral-700 rounded hover:bg-neutral-200 dark:hover:bg-neutral-600 disabled:opacity-50">
                {fetching ? 'Fetching...' : 'Fetch Available Models'}
              </button>
            </div>

            {availableModels.length > 0 && (
              <div className="mb-3">
                <label className="text-xs text-neutral-500 block mb-1">Models (click to enable capabilities)</label>
                <div className="space-y-1 max-h-64 overflow-auto border border-neutral-200 dark:border-neutral-700 rounded max-w-md">
                  {availableModels.map(m => {
                    const caps = modelCaps(m);
                    return (
                      <div key={m} className="flex items-center px-3 py-1.5 text-sm hover:bg-neutral-50 dark:hover:bg-neutral-800/50">
                        <span className="flex-1 truncate font-mono text-xs">{m}</span>
                        <div className="flex gap-1">
                          {['chat', 'embedding', 'tts'].map(cap => (
                            <button key={cap} onClick={() => toggleModelCapability(m, cap)}
                              className={`text-xs px-1.5 py-0.5 rounded ${
                                caps.includes(cap) ? 'bg-[var(--accent-500)] text-white' : 'bg-neutral-100 dark:bg-neutral-700 text-neutral-500'
                              }`}
                            >{cap}</button>
                          ))}
                        </div>
                      </div>
                    );
                  })}
                </div>
              </div>
            )}

            <div className="mb-3 flex items-center gap-4">
              <label className="flex items-center gap-2 text-sm">
                <input type="checkbox" checked={ai.rag_enabled} onChange={e => updateAi({ rag_enabled: e.target.checked })} className="rounded" />
                Enable RAG
              </label>
              {ai.rag_enabled && (
                <label className="flex items-center gap-1 text-sm">
                  Chunks: <input type="number" value={ai.rag_chunk_count} onChange={e => updateAi({ rag_chunk_count: parseInt(e.target.value) || 5 })} className="w-16 text-sm px-2 py-0.5 rounded border border-neutral-300 dark:border-neutral-600 bg-white dark:bg-neutral-800" min={1} max={20} />
                </label>
              )}
            </div>
          </section>
        )}
      </div>
    </div>
  );
}
