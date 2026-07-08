import { useState } from 'react';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import Divider from '@mui/material/Divider';
import Button from '@mui/material/Button';
import Alert from '@mui/material/Alert';
import TextField from '@mui/material/TextField';
import Switch from '@mui/material/Switch';
import FormControlLabel from '@mui/material/FormControlLabel';
import Slider from '@mui/material/Slider';
import Select from '@mui/material/Select';
import MenuItem from '@mui/material/MenuItem';
import ToggleButtonGroup from '@mui/material/ToggleButtonGroup';
import ToggleButton from '@mui/material/ToggleButton';
import Accordion from '@mui/material/Accordion';
import AccordionSummary from '@mui/material/AccordionSummary';
import AccordionDetails from '@mui/material/AccordionDetails';
import ExpandMoreIcon from '@mui/icons-material/ExpandMore';
import { useSettingsPage } from './SettingsPage.shared';

const PRIMARY_SWATCHES = [
  '#f97316', '#ef4444', '#3b82f6', '#8b5cf6',
  '#10b981', '#f59e0b', '#ec4899', '#06b6d4',
];

const SECONDARY_SWATCHES = [
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
  { value: 'custom-openai', label: 'Custom OpenAI API' },
  { value: 'custom-anthropic', label: 'Custom Anthropic API' },
];

function envVarForProvider(provider: string): string {
  switch (provider) {
    case 'openai':
    case 'custom-openai':
      return 'OPENAI_API_KEY';
    case 'anthropic':
    case 'custom-anthropic':
      return 'ANTHROPIC_API_KEY';
    case 'google':
      return 'GOOGLE_API_KEY';
    default:
      return '';
  }
}

export default function SettingsPageMobile() {
  const {
    settings,
    saving,
    fetching,
    msg,
    msgSeverity,
    syncing,
    syncStatus,
    ai,
    research,
    theme,
    setMsg,
    updateAi,
    updateVault,
    updateResearch,
    updateTheme,
    handleSave,
    handleReindex,
    handleSyncNow,
    pickVaultDirectory,
  } = useSettingsPage();

  const [aiExpanded, setAiExpanded] = useState(false);

  if (!settings) {
    return (
      <Box sx={{ p: 2 }}>
        <Typography variant="body2" color="text.secondary">
          Loading settings...
        </Typography>
      </Box>
    );
  }

  return (
    <Box sx={{ height: '100%', overflow: 'auto' }}>
      {/* Save button + message */}
      <Box sx={{ p: 2, pb: 0 }}>
        <Button
          variant="contained"
          onClick={handleSave}
          disabled={saving}
          fullWidth
          sx={{ textTransform: 'none', mb: 1 }}
        >
          {saving ? 'Saving...' : 'Save'}
        </Button>
        {msg && (
          <Alert severity={msgSeverity} onClose={() => setMsg('')} sx={{ mb: 1 }}>
            {msg}
          </Alert>
        )}
      </Box>

      {/* ─── Vault Section ─── */}
      <Box sx={{ px: 2, pt: 2 }}>
        <Typography variant="subtitle1" sx={{ fontWeight: 600, mb: 1 }}>
          Vault
        </Typography>
        <Divider sx={{ mb: 1.5 }} />
        <TextField
          label="Vault Path"
          value={settings.vault_path || ''}
          onChange={e => updateVault({ vault_path: e.target.value })}
          fullWidth
          size="small"
          sx={{ mb: 1, '& .MuiInputBase-input': { fontFamily: 'monospace', fontSize: '0.8rem' } }}
        />
        <Button variant="outlined" size="small" onClick={pickVaultDirectory}>
          Browse
        </Button>
      </Box>

      {/* ─── Theme Section ─── */}
      <Box sx={{ px: 2, pt: 3 }}>
        <Typography variant="subtitle1" sx={{ fontWeight: 600, mb: 1 }}>
          Theme
        </Typography>
        <Divider sx={{ mb: 1.5 }} />
        <FormControlLabel
          control={
            <Switch
              checked={theme.dark_mode}
              onChange={e => updateTheme({ dark_mode: e.target.checked })}
            />
          }
          label="Dark mode"
          sx={{ mb: 1.5 }}
        />
        <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 0.5 }}>
          Primary color
        </Typography>
        <ToggleButtonGroup
          value={theme.primary_color}
          exclusive
          onChange={(_, v) => v && updateTheme({ primary_color: v })}
          sx={{ flexWrap: 'wrap', gap: 0.5, mb: 1.5 }}
        >
          {PRIMARY_SWATCHES.map(color => (
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
        <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 0.5 }}>
          Secondary color
        </Typography>
        <ToggleButtonGroup
          value={theme.secondary_color}
          exclusive
          onChange={(_, v) => v && updateTheme({ secondary_color: v })}
          sx={{ flexWrap: 'wrap', gap: 0.5, mb: 1.5 }}
        >
          {SECONDARY_SWATCHES.map(color => (
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
        <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 0.5 }}>
          Font Size: {theme.font_size || 16}px
        </Typography>
        <Slider
          value={theme.font_size || 16}
          min={12}
          max={28}
          step={1}
          onChange={(_, v) => updateTheme({ font_size: v as number })}
          valueLabelDisplay="auto"
          sx={{ mb: 1, maxWidth: 300 }}
        />
      </Box>

      {/* ─── AI Section (collapsible) ─── */}
      <Box sx={{ px: 2, pt: 3 }}>
        <Typography variant="subtitle1" sx={{ fontWeight: 600, mb: 1 }}>
          AI
        </Typography>
        <Divider sx={{ mb: 1 }} />
        <Accordion
          disableGutters
          square
          expanded={aiExpanded}
          onChange={() => setAiExpanded(!aiExpanded)}
          sx={{ boxShadow: 0, '&:before': { display: 'none' }, bgcolor: 'transparent' }}
        >
          <AccordionSummary expandIcon={<ExpandMoreIcon />} sx={{ px: 0, minHeight: 36 }}>
            <Typography variant="body2" color="text.secondary">
              {aiExpanded ? 'Hide provider config' : 'Configure AI provider'}
            </Typography>
          </AccordionSummary>
          <AccordionDetails sx={{ px: 0, pb: 1 }}>
            <Box sx={{ display: 'flex', flexDirection: 'column', gap: 1.5 }}>
              {ai?.api_key_from_env && (
                <Alert severity="warning" sx={{ py: 0.5, px: 1.5, '& .MuiAlert-message': { py: 0.5 } }}>
                  <Typography variant="caption">
                    <strong>Security Notice:</strong> API key set via{' '}
                    <strong>{envVarForProvider(ai?.provider)}</strong> environment variable.
                  </Typography>
                </Alert>
              )}
              <Select
                value={ai?.provider || 'ollama'}
                onChange={e => updateAi({ provider: e.target.value })}
                size="small"
                displayEmpty
              >
                {PROVIDERS.map(p => (
                  <MenuItem key={p.value} value={p.value}>
                    {p.label}
                  </MenuItem>
                ))}
              </Select>
              <TextField
                label="API Endpoint"
                placeholder="http://localhost:11434"
                value={ai?.endpoint || ''}
                onChange={e => updateAi({ endpoint: e.target.value || null })}
                size="small"
              />
              <TextField
                label="API Key"
                type="password"
                placeholder="sk-..."
                value={ai?.api_key || ''}
                onChange={e => updateAi({ api_key: e.target.value || null })}
                size="small"
                sx={{ '& .MuiInputBase-input': { fontFamily: 'monospace', fontSize: '0.8rem' } }}
              />
              {ai?.api_key_from_env && (
                <Alert severity="info" sx={{ py: 0, px: 1.5, '& .MuiAlert-message': { py: 0.75 } }}>
                  <Typography variant="caption">
                    API key loaded from <strong>{envVarForProvider(ai?.provider)}</strong> environment variable.
                    {ai?.api_key ? ' Config file key is ignored while the env var is set.' : ''}
                  </Typography>
                </Alert>
              )}
              <TextField
                label="Default Model"
                placeholder="gpt-4o"
                value={ai?.model || ''}
                onChange={e => updateAi({ model: e.target.value })}
                size="small"
              />
              <FormControlLabel
                control={
                  <Switch
                    checked={ai?.rag_enabled ?? false}
                    onChange={e => updateAi({ rag_enabled: e.target.checked })}
                  />
                }
                label="Enable RAG"
              />
            </Box>
          </AccordionDetails>
        </Accordion>
      </Box>

      {/* ─── Research Section ─── */}
      <Box sx={{ px: 2, pt: 2 }}>
        <Typography variant="subtitle1" sx={{ fontWeight: 600, mb: 1 }}>
          Research
        </Typography>
        <Divider sx={{ mb: 1.5 }} />
        <TextField
          label="SearXNG Endpoint"
          placeholder="http://localhost:8888"
          value={research.searxng_endpoint}
          onChange={e => updateResearch({ searxng_endpoint: e.target.value })}
          fullWidth
          size="small"
          helperText="URL of your SearXNG instance"
        />
      </Box>

      {/* ─── Developer Section ─── */}
      <Box sx={{ px: 2, pt: 3 }}>
        <Typography variant="subtitle1" sx={{ fontWeight: 600, mb: 1 }}>
          Developer
        </Typography>
        <Divider sx={{ mb: 1.5 }} />
        <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 1 }}>
          Re-sync all pages from disk into the database. Idempotent.
        </Typography>
        <Button
          variant="contained"
          color="error"
          onClick={handleReindex}
          disabled={fetching}
          size="small"
          sx={{ textTransform: 'none' }}
        >
          {fetching ? 'Reindexing...' : 'Rebuild Index'}
        </Button>
      </Box>

      {/* ─── Sync Section ─── */}
      <Box sx={{ px: 2, pt: 3, pb: 3 }}>
        <Typography variant="subtitle1" sx={{ fontWeight: 600, mb: 1 }}>
          Sync
        </Typography>
        <Divider sx={{ mb: 1.5 }} />
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1.5, mb: 1 }}>
          <Button
            variant="contained"
            onClick={handleSyncNow}
            disabled={syncing}
            size="small"
            sx={{ textTransform: 'none' }}
          >
            {syncing ? 'Syncing...' : 'Sync Now'}
          </Button>
          {syncStatus && (
            <Box
              sx={{
                px: 1.5,
                py: 0.25,
                borderRadius: 1,
                fontSize: '0.65rem',
                fontWeight: 600,
                textTransform: 'uppercase',
                letterSpacing: '0.05em',
                color: '#fff',
                bgcolor:
                  syncStatus.status === 'ok'
                    ? '#10b981'
                    : syncStatus.status === 'conflicts'
                      ? '#ef4444'
                      : syncStatus.status === 'no_repo'
                        ? '#eab308'
                        : '#6b7280',
              }}
            >
              {syncStatus.status === 'ok' && 'OK'}
              {syncStatus.status === 'conflicts' &&
                `Conflicts (${syncStatus.conflicts.length})`}
              {syncStatus.status === 'no_repo' && 'No Repo'}
              {syncStatus.status !== 'ok' &&
                syncStatus.status !== 'conflicts' &&
                syncStatus.status !== 'no_repo' &&
                syncStatus.status}
              {(syncStatus.ahead > 0 || syncStatus.behind > 0) && (
                <Box component="span" sx={{ ml: 0.5, fontWeight: 400 }}>
                  +{syncStatus.ahead}/-{syncStatus.behind}
                </Box>
              )}
            </Box>
          )}
        </Box>
        {syncStatus?.branch && (
          <Typography variant="caption" color="text.disabled" sx={{ fontFamily: 'monospace', display: 'block' }}>
            {syncStatus.branch}
          </Typography>
        )}
        {syncStatus?.last_sync_time && (
          <Typography variant="caption" color="text.disabled" sx={{ display: 'block' }}>
            Last sync: {new Date(syncStatus.last_sync_time).toLocaleString()}
          </Typography>
        )}
      </Box>

    </Box>
  );
}
