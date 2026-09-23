import { useState, useEffect } from 'react';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import Divider from '@mui/material/Divider';
import Alert from '@mui/material/Alert';
import TextField from '@mui/material/TextField';
import Switch from '@mui/material/Switch';
import FormControlLabel from '@mui/material/FormControlLabel';
import Select from '@mui/material/Select';
import MenuItem from '@mui/material/MenuItem';
import Button from '@mui/material/Button';
import ToggleButtonGroup from '@mui/material/ToggleButtonGroup';
import ToggleButton from '@mui/material/ToggleButton';
import Accordion from '@mui/material/Accordion';
import AccordionSummary from '@mui/material/AccordionSummary';
import AccordionDetails from '@mui/material/AccordionDetails';
import ExpandMoreIcon from '@mui/icons-material/ExpandMore';

// ---------------------------------------------------------------------------
// Mobile AI provider configuration (collapsible accordion).
// Extracted from SettingsPage.mobile.tsx during the E6 sizing-gate refactor so
// the mobile settings screen stays under the 400-line component gate.
//
// Parity: mirrors the desktop AITab surfaces — provider/endpoint/key/model,
// Fetch Available Models + per-model capability editor, RAG toggle + chunk
// count, embedding dimensions, and masking of saved API keys. STT/TTS config
// lives in MobileSttTtsSection (separate module) to stay under the file gate.
// ---------------------------------------------------------------------------

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

export interface MobileAiAccordionProps {
  ai?: {
    provider: string;
    endpoint: string | null;
    api_key: string | null;
    api_key_from_env: boolean;
    model: string;
    models: { name: string; capabilities: string[] }[];
    rag_enabled: boolean;
    rag_chunk_count: number;
    embedding_dimensions: number;
  };
  updateAi: (patch: any) => void;
  availableModels?: string[];
  fetching?: boolean;
  onFetchModels?: () => void;
  onToggleModelCapability?: (modelName: string, cap: string) => void;
}

export default function MobileAiAccordion({
  ai,
  updateAi,
  availableModels = [],
  fetching = false,
  onFetchModels,
  onToggleModelCapability,
}: MobileAiAccordionProps) {
  const [aiExpanded, setAiExpanded] = useState(false);
  const [isKeyMasked, setIsKeyMasked] = useState(false);

  useEffect(() => {
    setIsKeyMasked(!!(ai?.api_key && ai.api_key.includes('****')));
  }, [ai?.api_key]);

  const modelCaps = (name: string) =>
    (ai?.models || []).find(m => m.name === name)?.capabilities || [];

  return (
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
              placeholder={isKeyMasked ? 'Key saved - enter new value to change' : 'sk-...'}
              value={isKeyMasked ? '' : (ai?.api_key || '')}
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

            {/* Fetch models + capability editor (desktop AITab parity) */}
            <Box>
              <Button
                variant="outlined"
                size="small"
                onClick={onFetchModels}
                disabled={fetching}
                sx={{ textTransform: 'none' }}
              >
                {fetching ? 'Fetching...' : 'Fetch Available Models'}
              </Button>
            </Box>
            {availableModels.length > 0 && (
              <Box>
                <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 0.5 }}>
                  Models (tap to enable capabilities)
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
                          flexWrap: 'wrap',
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
                              onChange={() => onToggleModelCapability?.(m, cap)}
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

            <FormControlLabel
              control={
                <Switch
                  checked={ai?.rag_enabled ?? false}
                  onChange={e => updateAi({ rag_enabled: e.target.checked })}
                />
              }
              label="Enable RAG"
            />
            {ai?.rag_enabled && (
              <TextField
                label="Chunks"
                type="number"
                value={ai.rag_chunk_count || 5}
                onChange={e => updateAi({ rag_chunk_count: parseInt(e.target.value) || 5 })}
                size="small"
                slotProps={{ htmlInput: { min: 1, max: 20 } }}
                helperText="Number of context chunks (1–20)"
              />
            )}
            <TextField
              label="Embedding Dimensions (0 = auto)"
              type="number"
              value={ai?.embedding_dimensions ?? 0}
              onChange={e => updateAi({ embedding_dimensions: parseInt(e.target.value) || 0 })}
              size="small"
              slotProps={{ htmlInput: { min: 0, max: 4096 } }}
            />
          </Box>
        </AccordionDetails>
      </Accordion>
    </Box>
  );
}
