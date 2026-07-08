import { useState, useEffect } from 'react';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import Alert from '@mui/material/Alert';
import Select from '@mui/material/Select';
import MenuItem from '@mui/material/MenuItem';
import TextField from '@mui/material/TextField';
import Button from '@mui/material/Button';
import Switch from '@mui/material/Switch';
import FormControlLabel from '@mui/material/FormControlLabel';
import ToggleButtonGroup from '@mui/material/ToggleButtonGroup';
import ToggleButton from '@mui/material/ToggleButton';

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

const CAPABILITIES = ['chat', 'embedding', 'tts'] as const;

interface AITabProps {
  ai: {
    provider: string;
    endpoint: string | null;
    api_key: string | null;
    api_key_from_env: boolean;
    model: string;
    models: { name: string; capabilities: string[] }[];
    rag_enabled: boolean;
    rag_chunk_count: number;
  };
  onAiChange: (patch: Partial<AITabProps['ai']>) => void;
  availableModels: string[];
  fetching: boolean;
  onFetchModels: () => void;
  onToggleModelCapability: (modelName: string, cap: string) => void;
}

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

export default function AITab({
  ai,
  onAiChange,
  availableModels,
  fetching,
  onFetchModels,
  onToggleModelCapability,
}: AITabProps) {
  const modelCaps = (name: string) =>
    (ai.models || []).find(m => m.name === name)?.capabilities || [];
  const envVarName = envVarForProvider(ai.provider);
  const [isKeyMasked, setIsKeyMasked] = useState(false);

  useEffect(() => {
    setIsKeyMasked(!!(ai.api_key && ai.api_key.includes('****')));
  }, [ai.api_key]);

  return (
    <Box>
      <Typography variant="subtitle2" sx={{ mb: 1.5, color: 'text.secondary' }}>AI Configuration</Typography>

      {ai.api_key_from_env && (
        <Alert severity="warning" sx={{ mb: 2, maxWidth: 480 }}>
          <Typography variant="caption">
            <strong>Security Notice:</strong> API key is set via the <strong>{envVarName}</strong> environment variable.
            {' '}Environment variables are more secure than storing keys in config files.
            {' '}To use a different key, unset the environment variable or update it.
          </Typography>
        </Alert>
      )}

      <Box sx={{ maxWidth: 480, display: 'flex', flexDirection: 'column', gap: 2 }}>
        <Select
          value={ai.provider || 'ollama'}
          onChange={e => onAiChange({ provider: e.target.value })}
          size="small"
          displayEmpty
        >
          {PROVIDERS.map(p => (
            <MenuItem key={p.value} value={p.value}>
              {p.label}
            </MenuItem>
          ))}
        </Select>

        {(ai.provider === 'ollama' ||
          ai.provider === 'custom' ||
          ai.provider === 'zai' ||
          ai.provider === 'custom-openai' ||
          ai.provider === 'custom-anthropic') && (
          <TextField
            label="API Endpoint URL"
            placeholder="http://localhost:11434"
            value={ai.endpoint || ''}
            onChange={e => onAiChange({ endpoint: e.target.value || null })}
            size="small"
          />
        )}

        <TextField
          label="API Key"
          type="password"
          placeholder={isKeyMasked ? 'Key saved - enter new value to change' : 'sk-...'}
          value={isKeyMasked ? '' : (ai.api_key || '')}
          onChange={e => onAiChange({ api_key: e.target.value || null })}
          size="small"
          sx={{ '& .MuiInputBase-input': { fontFamily: 'monospace' } }}
        />

        {ai.api_key_from_env && (
          <Alert severity="info" sx={{ py: 0, px: 1.5, '& .MuiAlert-message': { py: 0.75 } }}>
            <Typography variant="caption">
              API key loaded from <strong>{envVarName}</strong> environment variable.
              {ai.api_key ? ' Config file key is ignored while the env var is set.' : ''}
            </Typography>
          </Alert>
        )}

        <TextField
          label="Default Chat Model"
          placeholder="gpt-4o"
          value={ai.model || ''}
          onChange={e => onAiChange({ model: e.target.value })}
          size="small"
        />

        <Box>
          <Button variant="outlined" size="small" onClick={onFetchModels} disabled={fetching}>
            {fetching ? 'Fetching...' : 'Fetch Available Models'}
          </Button>
        </Box>

        {availableModels.length > 0 && (
          <Box>
            <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 0.5 }}>
              Models (click to enable capabilities)
            </Typography>
            <Box
              sx={{
                maxHeight: 200,
                overflow: 'auto',
                border: 1,
                borderColor: 'divider',
                borderRadius: 1,
              }}
            >
              {availableModels.map(m => {
                const caps = modelCaps(m);
                return (
                  <Box
                    key={m}
                    sx={{
                      display: 'flex',
                      alignItems: 'center',
                      px: 1.5,
                      py: 0.75,
                      '&:hover': { bgcolor: 'action.hover' },
                    }}
                  >
                    <Typography variant="caption" sx={{ flex: 1, fontFamily: 'monospace' }}>
                      {m}
                    </Typography>
                    <ToggleButtonGroup size="small" value={caps}>
                      {CAPABILITIES.map(cap => (
                        <ToggleButton
                          key={cap}
                          value={cap}
                          selected={caps.includes(cap)}
                          onChange={() => onToggleModelCapability(m, cap)}
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
            control={
              <Switch
                checked={ai.rag_enabled}
                onChange={e => onAiChange({ rag_enabled: e.target.checked })}
              />
            }
            label="Enable RAG"
          />
          {ai.rag_enabled && (
            <TextField
              label="Chunks"
              type="number"
              value={ai.rag_chunk_count || 5}
              onChange={e => onAiChange({ rag_chunk_count: parseInt(e.target.value) || 5 })}
              size="small"
              slotProps={{ htmlInput: { min: 1, max: 20 } }}
              sx={{ width: 100 }}
            />
          )}
        </Box>
      </Box>
    </Box>
  );
}
