import { useState } from 'react';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import Divider from '@mui/material/Divider';
import Alert from '@mui/material/Alert';
import TextField from '@mui/material/TextField';
import Switch from '@mui/material/Switch';
import Checkbox from '@mui/material/Checkbox';
import FormControlLabel from '@mui/material/FormControlLabel';
import Select from '@mui/material/Select';
import MenuItem from '@mui/material/MenuItem';
import Accordion from '@mui/material/Accordion';
import AccordionSummary from '@mui/material/AccordionSummary';
import AccordionDetails from '@mui/material/AccordionDetails';
import ExpandMoreIcon from '@mui/icons-material/ExpandMore';

// ---------------------------------------------------------------------------
// Mobile AI provider configuration (collapsible accordion).
// Extracted from SettingsPage.mobile.tsx during the E6 sizing-gate refactor so
// the mobile settings screen stays under the 400-line component gate.
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
    rag_enabled: boolean;
    embedding_dimensions: number;
    use_llm_gateway_and_auth: boolean;
  };
  updateAi: (patch: any) => void;
}

export default function MobileAiAccordion({ ai, updateAi }: MobileAiAccordionProps) {
  const [aiExpanded, setAiExpanded] = useState(false);

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
            <FormControlLabel
              control={
                <Checkbox
                  checked={ai?.use_llm_gateway_and_auth ?? false}
                  onChange={e => updateAi({ use_llm_gateway_and_auth: e.target.checked })}
                />
              }
              label="Use gateway and auth from LLM"
            />
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
