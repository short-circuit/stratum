import { useState, useEffect } from 'react';
import * as api from '../lib/commands';
import { applyTheme } from '../lib/theme';

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

const SECONDARY_COLORS = [
  '#6b7280', '#78716c', '#a1a1aa', '#71717a',
  '#52525b', '#3f3f46', '#27272a',
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
        applyTheme(
          s.theme.primary_color || '#f97316',
          s.theme.secondary_color || '#6b7280',
          s.theme.dark_mode,
          s.theme.font_size,
        );
      }
    }).catch(err => setMsg(`Load failed: ${err}`));
  }, []);

  if (!settings) {
    return <div className="p-6 text-[var(--secondary-400)] text-sm">Loading settings...</div>;
  }

  const ai = settings.ai;
  const theme = settings.theme || { dark_mode: true, primary_color: '#f97316', secondary_color: '#6b7280', font_size: 16 };

  const updateAi = (patch: any) => setSettings({ ...settings, ai: { ...ai, ...patch } });
  const updateVault = (patch: any) => setSettings({ ...settings, ...patch });

  const updateTheme = (patch: any) => {
    const newTheme = { ...theme, ...patch };
    setSettings({ ...settings, theme: newTheme });
    applyTheme(newTheme.primary_color, newTheme.secondary_color, newTheme.dark_mode, newTheme.font_size);
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
      <div className="flex items-center border-b border-[var(--secondary-200)] dark:border-[var(--secondary-700)] shrink-0">
        <nav className="flex">
          {tabs.map(t => (
            <button
              key={t.id}
              onClick={() => setTab(t.id)}
              className={`px-4 py-2 text-sm font-medium transition-colors border-b-2 -mb-px ${
                tab === t.id
                  ? 'border-[var(--primary-500)] text-[var(--primary-600)] dark:text-[var(--primary-400)]'
                  : 'border-transparent text-[var(--secondary-500)] hover:text-[var(--secondary-700)] dark:hover:text-[var(--secondary-300)]'
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
          className="mr-3 px-4 py-1.5 bg-[var(--primary-500)] text-white text-sm rounded hover:bg-[var(--primary-600)] disabled:opacity-50"
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
            <h3 className="text-sm font-semibold text-[var(--secondary-700)] dark:text-[var(--secondary-300)] mb-3">Vault</h3>
            <div className="mb-3">
              <label className="text-xs text-[var(--secondary-500)] block mb-1">Vault Path</label>
              <input
                type="text"
                value={settings.vault_path}
                onChange={e => updateVault({ vault_path: e.target.value })}
                className="w-full max-w-md text-sm px-2 py-1.5 rounded border border-[var(--secondary-300)] dark:border-[var(--secondary-600)] bg-white dark:bg-[var(--secondary-800)] font-mono"
              />
            </div>
          </section>
        )}

        {tab === 'theme' && (
          <section>
            <h3 className="text-sm font-semibold text-[var(--secondary-700)] dark:text-[var(--secondary-300)] mb-3">Theme</h3>
            <div className="mb-3">
              <label className="flex items-center gap-2 text-sm">
                <input type="checkbox" checked={theme.dark_mode} onChange={e => updateTheme({ dark_mode: e.target.checked })} className="rounded" />
                Dark mode
              </label>
            </div>

            {/* Primary color */}
            <div className="mb-4">
              <label className="text-xs font-medium text-[var(--secondary-600)] dark:text-[var(--secondary-400)] block mb-1">Primary (buttons, accents)</label>
              <div className="flex gap-1.5 flex-wrap mb-2">
                {PRESET_COLORS.map(color => (
                  <button key={color} onClick={() => updateTheme({ primary_color: color })}
                    className={`w-7 h-7 rounded-full border-2 transition-transform ${
                      theme.primary_color === color ? 'border-[var(--secondary-800)] dark:border-white scale-110' : 'border-transparent hover:scale-105'
                    }`}
                    style={{ backgroundColor: color }} title={color} />
                ))}
              </div>
              <div className="flex items-center gap-2">
                <input type="color" value={theme.primary_color} onChange={e => updateTheme({ primary_color: e.target.value })} className="w-8 h-8 rounded cursor-pointer border-0 p-0" />
                <input type="text" value={theme.primary_color} onChange={e => updateTheme({ primary_color: e.target.value })} className="flex-1 max-w-[180px] text-xs px-2 py-1 rounded border border-[var(--secondary-300)] dark:border-[var(--secondary-600)] bg-white dark:bg-[var(--secondary-800)] font-mono" placeholder="#f97316" />
              </div>
            </div>

            {/* Secondary color */}
            <div className="mb-3">
              <label className="text-xs font-medium text-[var(--secondary-600)] dark:text-[var(--secondary-400)] block mb-1">Secondary (backgrounds, borders)</label>
              <div className="flex gap-1.5 flex-wrap mb-2">
                {SECONDARY_COLORS.map(color => (
                  <button key={color} onClick={() => updateTheme({ secondary_color: color })}
                    className={`w-7 h-7 rounded-full border-2 transition-transform ${
                      theme.secondary_color === color ? 'border-[var(--secondary-800)] dark:border-white scale-110' : 'border-transparent hover:scale-105'
                    }`}
                    style={{ backgroundColor: color }} title={color} />
                ))}
              </div>
              <div className="flex items-center gap-2">
                <input type="color" value={theme.secondary_color} onChange={e => updateTheme({ secondary_color: e.target.value })} className="w-8 h-8 rounded cursor-pointer border-0 p-0" />
                <input type="text" value={theme.secondary_color} onChange={e => updateTheme({ secondary_color: e.target.value })} className="flex-1 max-w-[180px] text-xs px-2 py-1 rounded border border-[var(--secondary-300)] dark:border-[var(--secondary-600)] bg-white dark:bg-[var(--secondary-800)] font-mono" placeholder="#6b7280" />
              </div>
            </div>

            {/* Font size */}
            <div className="mb-3">
              <label className="text-xs font-medium text-[var(--secondary-600)] dark:text-[var(--secondary-400)] block mb-1">
                Font Size: {theme.font_size || 16}px
              </label>
              <input
                type="range"
                min={12}
                max={28}
                step={1}
                value={theme.font_size || 16}
                onChange={e => updateTheme({ font_size: parseInt(e.target.value) })}
                className="w-full max-w-md accent-[var(--primary-500)]"
              />
              <div className="flex justify-between text-xs text-[var(--secondary-400)] mt-0.5 max-w-md">
                <span>12px</span>
                <span>16px</span>
                <span>20px</span>
                <span>28px</span>
              </div>
            </div>
          </section>
        )}

        {tab === 'ai' && (
          <section>
            <h3 className="text-sm font-semibold text-[var(--secondary-700)] dark:text-[var(--secondary-300)] mb-3">AI Configuration</h3>

            <div className="mb-3">
              <label className="text-xs text-[var(--secondary-500)] block mb-1">Provider</label>
              <select
                value={ai.provider}
                onChange={e => updateAi({ provider: e.target.value })}
                className="w-full max-w-md text-sm px-2 py-1.5 rounded border border-[var(--secondary-300)] dark:border-[var(--secondary-600)] bg-white dark:bg-[var(--secondary-800)]"
              >
                {PROVIDERS.map(p => <option key={p.value} value={p.value}>{p.label}</option>)}
              </select>
            </div>

            {(ai.provider === 'ollama' || ai.provider === 'custom' || ai.provider === 'zai') && (
              <div className="mb-3">
                <label className="text-xs text-[var(--secondary-500)] block mb-1">API Endpoint URL</label>
                <input type="text" value={ai.endpoint || ''} onChange={e => updateAi({ endpoint: e.target.value || null })} placeholder="http://localhost:11434" className="w-full max-w-md text-sm px-2 py-1.5 rounded border border-[var(--secondary-300)] dark:border-[var(--secondary-600)] bg-white dark:bg-[var(--secondary-800)]" />
              </div>
            )}

            <div className="mb-3">
              <label className="text-xs text-[var(--secondary-500)] block mb-1">API Key</label>
              <input type="password" value={ai.api_key || ''} onChange={e => updateAi({ api_key: e.target.value || null })} placeholder="sk-..." className="w-full max-w-md text-sm px-2 py-1.5 rounded border border-[var(--secondary-300)] dark:border-[var(--secondary-600)] bg-white dark:bg-[var(--secondary-800)] font-mono" />
            </div>

            <div className="mb-3">
              <label className="text-xs text-[var(--secondary-500)] block mb-1">Default Chat Model</label>
              <input type="text" value={ai.model} onChange={e => updateAi({ model: e.target.value })} placeholder="gpt-4o" className="w-full max-w-md text-sm px-2 py-1.5 rounded border border-[var(--secondary-300)] dark:border-[var(--secondary-600)] bg-white dark:bg-[var(--secondary-800)]" />
            </div>

            <div className="mb-3">
              <button onClick={handleFetchModels} disabled={fetching} className="text-sm px-3 py-1.5 bg-[var(--secondary-100)] dark:bg-[var(--secondary-700)] rounded hover:bg-[var(--secondary-200)] dark:hover:bg-[var(--secondary-600)] disabled:opacity-50">
                {fetching ? 'Fetching...' : 'Fetch Available Models'}
              </button>
            </div>

            {availableModels.length > 0 && (
              <div className="mb-3">
                <label className="text-xs text-[var(--secondary-500)] block mb-1">Models (click to enable capabilities)</label>
                <div className="space-y-1 max-h-64 overflow-auto border border-[var(--secondary-200)] dark:border-[var(--secondary-700)] rounded max-w-md">
                  {availableModels.map(m => {
                    const caps = modelCaps(m);
                    return (
                      <div key={m} className="flex items-center px-3 py-1.5 text-sm hover:bg-[var(--secondary-50)] dark:hover:bg-[var(--secondary-800)]/50">
                        <span className="flex-1 truncate font-mono text-xs">{m}</span>
                        <div className="flex gap-1">
                          {['chat', 'embedding', 'tts'].map(cap => (
                            <button key={cap} onClick={() => toggleModelCapability(m, cap)}
                              className={`text-xs px-1.5 py-0.5 rounded ${
                                caps.includes(cap) ? 'bg-[var(--primary-500)] text-white' : 'bg-[var(--secondary-100)] dark:bg-[var(--secondary-700)] text-[var(--secondary-500)]'
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
                  Chunks: <input type="number" value={ai.rag_chunk_count} onChange={e => updateAi({ rag_chunk_count: parseInt(e.target.value) || 5 })} className="w-16 text-sm px-2 py-0.5 rounded border border-[var(--secondary-300)] dark:border-[var(--secondary-600)] bg-white dark:bg-[var(--secondary-800)]" min={1} max={20} />
                </label>
              )}
            </div>
          </section>
        )}
      </div>
    </div>
  );
}
