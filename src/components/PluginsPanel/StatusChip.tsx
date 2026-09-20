import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import { useState } from 'react';
import type { PluginInfoDto } from '../../lib/types';

const STATUS_META: Record<string, { label: string; color: string; bg: string }> = {
  ready: { label: 'Ready', color: '#10b981', bg: 'rgba(16,185,129,0.12)' },
  disabled: { label: 'Disabled', color: '#6b7280', bg: 'rgba(107,114,128,0.15)' },
  error: { label: 'Error', color: '#ef4444', bg: 'rgba(239,68,68,0.12)' },
};

export function StatusChip({ plugin }: { plugin: PluginInfoDto }) {
  const meta = STATUS_META[plugin.status] ?? STATUS_META.error;
  const [full, setFull] = useState(false);

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', alignItems: 'flex-start', gap: 0.5 }}>
      <Box
        component="span"
        sx={{
          px: 1,
          py: 0.25,
          borderRadius: 1,
          fontSize: '0.65rem',
          fontWeight: 600,
          textTransform: 'uppercase',
          letterSpacing: '0.05em',
          color: meta.color,
          bgcolor: meta.bg,
          cursor: plugin.status === 'error' && plugin.error ? 'pointer' : 'default',
          title: plugin.status === 'error' && plugin.error ? 'Click for details' : undefined,
        }}
        onClick={() => {
          if (plugin.status === 'error' && plugin.error) setFull(v => !v);
        }}
      >
        {meta.label}
      </Box>
      {full && plugin.error && (
        <Typography
          variant="caption"
          component="pre"
          sx={{ m: 0, whiteSpace: 'pre-wrap', wordBreak: 'break-word', color: 'text.secondary', maxWidth: 420, fontSize: '0.68rem', fontFamily: 'monospace' }}
        >
          {plugin.error}
        </Typography>
      )}
    </Box>
  );
}
