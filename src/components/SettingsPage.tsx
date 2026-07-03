import { useState, useEffect, useRef, useCallback } from 'react';
import Box from '@mui/material/Box';
import Tabs from '@mui/material/Tabs';
import Tab from '@mui/material/Tab';
import TextField from '@mui/material/TextField';
import Button from '@mui/material/Button';
import Select from '@mui/material/Select';
import MenuItem from '@mui/material/MenuItem';
import Switch from '@mui/material/Switch';
import Slider from '@mui/material/Slider';
import Typography from '@mui/material/Typography';
import Alert from '@mui/material/Alert';
import FormControlLabel from '@mui/material/FormControlLabel';
import ToggleButtonGroup from '@mui/material/ToggleButtonGroup';
import ToggleButton from '@mui/material/ToggleButton';
import { useStore } from '../stores/appStore';
import * as api from '../lib/commands';
import { applyTheme } from '../lib/theme';

const PRESET_COLORS = [
  '#f97316', '#ef4444', '#3b82f6', '#8b5cf6',
  '#10b981', '#f59e0b', '#ec4899', '#06b6d4',
];

const SECONDARY_COLORS = [
  '#6b7280', '#78716c', '#a1a1aa', '#71717a',
  '#52525b', '#3f3f46', '#27272a',
];

const PROVIDERS = [
  { value: 'ollama', label: 'Ollama (Local)' },
  { value: 'openai', label: 'OpenAI' },
  { value: 'anthropic', label: 'Anthropic' },
  { value: 'google', label: 'Google AI' },
  { value: 'zai', label: 'Z.AI' },
  { value: 'custom', label: 'Custom (OpenAI-compatible)' },
];

const CAPABILITIES = ['chat', 'embedding', 'tts'] as const;

type Tab = 'vault' | 'theme' | 'ai' | 'research' | 'developer';

export default function SettingsPage() {
  const { pickVaultDirectory, setThemeConfig } = useStore();
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const [settings, setSettings] = useState<any>(null);
  const [saving, setSaving] = useState(false);
  const [fetching, setFetching] = useState(false);
  const [msg, setMsg] = useState('');
  const [msgSeverity, setMsgSeverity] = useState<'success' | 'error'>('success');
  const [availableModels, setAvailableModels] = useState<string[]>([]);
  const [tab, setTab] = useState<Tab>('vault');
  const savedThemeRef = useRef<{ primary: string; secondary: string; dark: boolean; fontSize: number } | null>(null);
  const muiSyncTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const syncMuiTheme = useCallback((primary: string, secondary: string, dark: boolean, fontSize: number) => {
    if (muiSyncTimer.current) clearTimeout(muiSyncTimer.current);
    muiSyncTimer.current = setTimeout(() => {
      setThemeConfig({ primaryHex: primary, secondaryHex: secondary, dark, fontSize });
    }, 150);
  }, [setThemeConfig]);

  useEffect(() => {
    api.getSettings().then(s => {
      setSettings(s);
      if (s.theme) {
        const p = s.theme.primary_color || '#f97316';
        const sec = s.theme.secondary_color || '#6b7280';
        const d = s.theme.dark_mode ?? true;
        const f = s.theme.font_size || 16;
        savedThemeRef.current = { primary: p, secondary: sec, dark: d, fontSize: f };
        applyTheme(p, sec, d, f);
        setThemeConfig({ primaryHex: p, secondaryHex: sec, dark: d, fontSize: f });
      }
    }).catch(err => { setMsg(`Load failed: ${err}`); setMsgSeverity('error'); });

    return () => {
      // Restore saved theme on unmount without save
      const saved = savedThemeRef.current;
      if (saved) {
        applyTheme(saved.primary, saved.secondary, saved.dark, saved.fontSize);
        setThemeConfig({ primaryHex: saved.primary, secondaryHex: saved.secondary, dark: saved.dark, fontSize: saved.fontSize });
      }
      if (muiSyncTimer.current) clearTimeout(muiSyncTimer.current);
    };
  }, [setThemeConfig]);

  if (!settings) {
    return (
      <Box sx={{ p: 3 }}>
        <Typography variant="body2" color="text.secondary">Loading settings...</Typography>
      </Box>
    );
  }

  const ai = settings.ai;
  const research = settings.research || { searxng_endpoint: 'http://localhost:8888', max_results: 3, max_depth: 2 };
  const theme = settings.theme || { dark_mode: true, primary_color: '#f97316', secondary_color: '#6b7280', font_size: 16 };

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const updateAi = (patch: any) => setSettings({ ...settings, ai: { ...ai, ...patch } });
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const updateVault = (patch: any) => setSettings({ ...settings, ...patch });
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const updateResearch = (patch: any) => setSettings({ ...settings, research: { ...research, ...patch } });
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const updateTheme = (patch: any) => {
    const newTheme = { ...theme, ...patch };
    setSettings({ ...settings, theme: newTheme });
    applyTheme(newTheme.primary_color, newTheme.secondary_color, newTheme.dark_mode, newTheme.font_size);
    syncMuiTheme(newTheme.primary_color, newTheme.secondary_color, newTheme.dark_mode, newTheme.font_size || 16);
  };

  const handleSave = async () => {
    setSaving(true); setMsg('');
    try {
      await api.saveSettings(settings);
      const p = theme.primary_color;
      const sec = theme.secondary_color;
      const d = theme.dark_mode;
      const f = theme.font_size || 16;
      savedThemeRef.current = { primary: p, secondary: sec, dark: d, fontSize: f };
      if (muiSyncTimer.current) clearTimeout(muiSyncTimer.current);
      setThemeConfig({ primaryHex: p, secondaryHex: sec, dark: d, fontSize: f });
      setMsg('Saved.'); setMsgSeverity('success');
    }
    catch (e) { setMsg(`Save failed: ${e}`); setMsgSeverity('error'); }
    finally { setSaving(false); }
  };

  const handleFetchModels = async () => {
    setFetching(true); setMsg('');
    try { const models = await api.fetchModels(); setAvailableModels(models); setMsg(`Found ${models.length} models`); setMsgSeverity('success'); }
    catch (e) { setMsg(`Fetch failed: ${e}`); setMsgSeverity('error'); }
    finally { setFetching(false); }
  };

  const toggleModelCapability = (modelName: string, cap: string) => {
    const models = [...(ai.models || [])];
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const existing = models.find((m: any) => m.name === modelName);
    if (existing) {
      existing.capabilities = existing.capabilities.includes(cap)
        ? existing.capabilities.filter((c: string) => c !== cap)
        : [...existing.capabilities, cap];
    } else {
      models.push({ name: modelName, capabilities: [cap] });
    }
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    updateAi({ models: models.filter((m: any) => m.capabilities.length > 0) });
  };

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const modelCaps = (name: string) => (ai.models || []).find((m: any) => m.name === name)?.capabilities || [];

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      {/* Tab bar */}
      <Box sx={{ borderBottom: 1, borderColor: 'divider', display: 'flex', alignItems: 'center', flexShrink: 0 }}>
        <Tabs value={tab} onChange={(_, v: Tab) => setTab(v)} sx={{ minHeight: 40 }}>
          <Tab label="Vault" value="vault" sx={{ minHeight: 40, textTransform: 'none' }} />
          <Tab label="Theme" value="theme" sx={{ minHeight: 40, textTransform: 'none' }} />
          <Tab label="AI" value="ai" sx={{ minHeight: 40, textTransform: 'none' }} />
          <Tab label="Research" value="research" sx={{ minHeight: 40, textTransform: 'none' }} />
          <Tab label="Developer" value="developer" sx={{ minHeight: 40, textTransform: 'none' }} />
        </Tabs>
        <Box sx={{ flex: 1 }} />
        <Button
          variant="contained"
          onClick={handleSave}
          disabled={saving}
          sx={{ mr: 2, textTransform: 'none' }}
        >
          {saving ? 'Saving...' : 'Save'}
        </Button>
      </Box>

      {/* Message */}
      {msg && (
        <Alert severity={msgSeverity} sx={{ mx: 3, mt: 1.5 }} onClose={() => setMsg('')}>
          {msg}
        </Alert>
      )}

      {/* Tab content */}
      <Box sx={{ flex: 1, overflow: 'auto', p: 3 }}>
        {tab === 'vault' && (
          <Box>
            <Typography variant="subtitle2" sx={{ mb: 1.5, color: 'text.secondary' }}>Vault</Typography>
            <Box sx={{ maxWidth: 480 }}>
              <TextField
                label="Vault Path"
                value={settings.vault_path || ''}
                onChange={e => updateVault({ vault_path: e.target.value })}
                fullWidth
                size="small"
                sx={{ mb: 1, '& .MuiInputBase-input': { fontFamily: 'monospace' } }}
              />
              <Button variant="outlined" size="small" onClick={pickVaultDirectory}>
                Browse
              </Button>
            </Box>
          </Box>
        )}

        {tab === 'theme' && (
          <Box>
            <Typography variant="subtitle2" sx={{ mb: 1.5, color: 'text.secondary' }}>Theme</Typography>

            <FormControlLabel
              control={<Switch checked={theme.dark_mode} onChange={e => updateTheme({ dark_mode: e.target.checked })} />}
              label="Dark mode"
              sx={{ mb: 2 }}
            />

            {/* Primary color */}
            <Box sx={{ mb: 2.5 }}>
              <Typography variant="caption" sx={{ fontWeight: 500, color: 'text.secondary', display: 'block', mb: 0.75 }}>
                Primary (buttons, accents)
              </Typography>
              <ToggleButtonGroup
                value={theme.primary_color}
                exclusive
                onChange={(_, v) => v && updateTheme({ primary_color: v })}
                sx={{ flexWrap: 'wrap', gap: 0.5, mb: 1 }}
              >
                {PRESET_COLORS.map(color => (
                  <ToggleButton
                    key={color}
                    value={color}
                    size="small"
                    sx={{
                      width: 28, height: 28, minWidth: 28, p: 0, borderRadius: '50%!important',
                      border: 2, borderColor: theme.primary_color === color ? 'text.primary' : 'transparent',
                      bgcolor: color, '&:hover': { bgcolor: color },
                      '&.Mui-selected': { bgcolor: color, '&:hover': { bgcolor: color } },
                    }}
                  />
                ))}
              </ToggleButtonGroup>
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                <TextField
                  type="color"
                  value={theme.primary_color}
                  onChange={e => updateTheme({ primary_color: e.target.value })}
                  sx={{ width: 40, '& .MuiInputBase-root': { p: 0.25 }, '& input': { cursor: 'pointer', height: 32, p: 0 } }}
                />
                <TextField
                  size="small"
                  value={theme.primary_color}
                  onChange={e => updateTheme({ primary_color: e.target.value })}
                  placeholder="#f97316"
                  sx={{ width: 160, '& .MuiInputBase-input': { fontFamily: 'monospace', fontSize: '0.75rem' } }}
                />
              </Box>
            </Box>

            {/* Secondary color */}
            <Box sx={{ mb: 2.5 }}>
              <Typography variant="caption" sx={{ fontWeight: 500, color: 'text.secondary', display: 'block', mb: 0.75 }}>
                Secondary (backgrounds, borders)
              </Typography>
              <ToggleButtonGroup
                value={theme.secondary_color}
                exclusive
                onChange={(_, v) => v && updateTheme({ secondary_color: v })}
                sx={{ flexWrap: 'wrap', gap: 0.5, mb: 1 }}
              >
                {SECONDARY_COLORS.map(color => (
                  <ToggleButton
                    key={color}
                    value={color}
                    size="small"
                    sx={{
                      width: 28, height: 28, minWidth: 28, p: 0, borderRadius: '50%!important',
                      border: 2, borderColor: theme.secondary_color === color ? 'text.primary' : 'transparent',
                      bgcolor: color, '&:hover': { bgcolor: color },
                      '&.Mui-selected': { bgcolor: color, '&:hover': { bgcolor: color } },
                    }}
                  />
                ))}
              </ToggleButtonGroup>
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                <TextField
                  type="color"
                  value={theme.secondary_color}
                  onChange={e => updateTheme({ secondary_color: e.target.value })}
                  sx={{ width: 40, '& .MuiInputBase-root': { p: 0.25 }, '& input': { cursor: 'pointer', height: 32, p: 0 } }}
                />
                <TextField
                  size="small"
                  value={theme.secondary_color}
                  onChange={e => updateTheme({ secondary_color: e.target.value })}
                  placeholder="#6b7280"
                  sx={{ width: 160, '& .MuiInputBase-input': { fontFamily: 'monospace', fontSize: '0.75rem' } }}
                />
              </Box>
            </Box>

            {/* Font size */}
            <Box sx={{ mb: 2, maxWidth: 400 }}>
              <Typography variant="caption" sx={{ fontWeight: 500, color: 'text.secondary', display: 'block', mb: 1 }}>
                Font Size: {theme.font_size || 16}px
              </Typography>
              <Slider
                value={theme.font_size || 16}
                min={12}
                max={28}
                step={1}
                onChange={(_, v) => updateTheme({ font_size: v as number })}
                valueLabelDisplay="auto"
              />
              <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
                <Typography variant="caption" color="text.disabled">12px</Typography>
                <Typography variant="caption" color="text.disabled">20px</Typography>
                <Typography variant="caption" color="text.disabled">28px</Typography>
              </Box>
            </Box>
          </Box>
        )}

        {tab === 'ai' && (
          <Box>
            <Typography variant="subtitle2" sx={{ mb: 1.5, color: 'text.secondary' }}>AI Configuration</Typography>

            <Box sx={{ maxWidth: 480, display: 'flex', flexDirection: 'column', gap: 2 }}>
              <Select
                value={ai.provider || 'ollama'}
                onChange={e => updateAi({ provider: e.target.value })}
                size="small"
                displayEmpty
              >
                {PROVIDERS.map(p => <MenuItem key={p.value} value={p.value}>{p.label}</MenuItem>)}
              </Select>

              {(ai.provider === 'ollama' || ai.provider === 'custom' || ai.provider === 'zai') && (
                <TextField
                  label="API Endpoint URL"
                  placeholder="http://localhost:11434"
                  value={ai.endpoint || ''}
                  onChange={e => updateAi({ endpoint: e.target.value || null })}
                  size="small"
                />
              )}

              <TextField
                label="API Key"
                type="password"
                placeholder="sk-..."
                value={ai.api_key || ''}
                onChange={e => updateAi({ api_key: e.target.value || null })}
                size="small"
                sx={{ '& .MuiInputBase-input': { fontFamily: 'monospace' } }}
              />

              <TextField
                label="Default Chat Model"
                placeholder="gpt-4o"
                value={ai.model || ''}
                onChange={e => updateAi({ model: e.target.value })}
                size="small"
              />

              <Box>
                <Button variant="outlined" size="small" onClick={handleFetchModels} disabled={fetching}>
                  {fetching ? 'Fetching...' : 'Fetch Available Models'}
                </Button>
              </Box>

              {availableModels.length > 0 && (
                <Box>
                  <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 0.5 }}>
                    Models (click to enable capabilities)
                  </Typography>
                  <Box sx={{ maxHeight: 200, overflow: 'auto', border: 1, borderColor: 'divider', borderRadius: 1 }}>
                    {availableModels.map(m => {
                      const caps = modelCaps(m);
                      return (
                        <Box key={m} sx={{ display: 'flex', alignItems: 'center', px: 1.5, py: 0.75, '&:hover': { bgcolor: 'action.hover' } }}>
                          <Typography variant="caption" sx={{ flex: 1, fontFamily: 'monospace' }}>{m}</Typography>
                          <ToggleButtonGroup size="small" value={caps}>
                            {CAPABILITIES.map(cap => (
                              <ToggleButton
                                key={cap}
                                value={cap}
                                selected={caps.includes(cap)}
                                onChange={() => toggleModelCapability(m, cap)}
                                sx={{ textTransform: 'none', fontSize: '0.7rem', px: 1, py: 0.25 }}
                              >
                                {cap}
                              </ToggleButton>
                            ))}
                          </ToggleButtonGroup>
                        </Box>
                      );
                    })}
                  </Box>
                </Box>
              )}

              <Box sx={{ display: 'flex', alignItems: 'center', gap: 2 }}>
                <FormControlLabel
                  control={<Switch checked={ai.rag_enabled} onChange={e => updateAi({ rag_enabled: e.target.checked })} />}
                  label="Enable RAG"
                />
                {ai.rag_enabled && (
                  <TextField
                    label="Chunks"
                    type="number"
                    value={ai.rag_chunk_count || 5}
                    onChange={e => updateAi({ rag_chunk_count: parseInt(e.target.value) || 5 })}
                    size="small"
                    slotProps={{ htmlInput: { min: 1, max: 20 } }}
                    sx={{ width: 100 }}
                  />
                )}
              </Box>
            </Box>
          </Box>
        )}

        {tab === 'research' && (
          <Box>
            <Typography variant="subtitle2" sx={{ mb: 1.5, color: 'text.secondary' }}>Web Research (SearXNG)</Typography>

            <Box sx={{ maxWidth: 480, display: 'flex', flexDirection: 'column', gap: 2 }}>
              <TextField
                label="SearXNG Endpoint URL"
                placeholder="http://localhost:8888"
                value={research.searxng_endpoint}
                onChange={e => updateResearch({ searxng_endpoint: e.target.value })}
                size="small"
                helperText="The URL of your SearXNG instance (e.g. http://localhost:8888)"
              />

              <TextField
                label="Max Results Per Search"
                type="number"
                value={research.max_results}
                onChange={e => updateResearch({ max_results: parseInt(e.target.value) || 3 })}
                size="small"
                slotProps={{ htmlInput: { min: 1, max: 10 } }}
                sx={{ width: 200 }}
              />

              <TextField
                label="Research Depth (search-read cycles)"
                type="number"
                value={research.max_depth}
                onChange={e => updateResearch({ max_depth: parseInt(e.target.value) || 2 })}
                size="small"
                slotProps={{ htmlInput: { min: 1, max: 5 } }}
                sx={{ width: 200 }}
              />
            </Box>
          </Box>
        )}

        {tab === 'developer' && (
          <Box>
            <Typography variant="subtitle2" sx={{ mb: 1.5, color: 'text.secondary' }}>Developer Tools</Typography>

            <Box sx={{ maxWidth: 480 }}>
              <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 1.5 }}>
                Re-sync all pages from disk into the database. Useful after importing new notes
                or recovering from a corrupted database. This operation is idempotent — running it
                multiple times produces the same result.
              </Typography>
              <Button
                variant="contained"
                color="error"
                onClick={async () => {
                  setFetching(true);
                  setMsg(''); setMsgSeverity('success');
                  try {
                    const count = await api.reindexVault();
                    setMsg(`Reindexed ${count} pages.`);
                  } catch (e) {
                    setMsg(`Reindex failed: ${e}`); setMsgSeverity('error');
                  } finally {
                    setFetching(false);
                  }
                }}
                disabled={fetching}
              >
                {fetching ? 'Reindexing...' : 'Reindex All'}
              </Button>
            </Box>
          </Box>
        )}
      </Box>
    </Box>
  );
}
