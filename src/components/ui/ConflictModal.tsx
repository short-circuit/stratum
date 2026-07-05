import { useState } from 'react';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Typography from '@mui/material/Typography';

interface ConflictModalProps {
  files: string[];
  onResolve: (file: string) => Promise<void>;
  onResolveAll: () => Promise<void>;
  onAbort: () => Promise<void>;
}

export default function ConflictModal({ files, onResolve, onResolveAll, onAbort }: ConflictModalProps) {
  const [resolving, setResolving] = useState<string | null>(null);

  return (
    <Box
      sx={{
        position: 'fixed',
        inset: 0,
        zIndex: 9999,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        bgcolor: 'rgba(0,0,0,0.5)',
      }}
    >
      <Box
        sx={{
          bgcolor: 'background.paper',
          borderRadius: 2,
          p: 3,
          maxWidth: 500,
          width: '90%',
          boxShadow: 24,
        }}
      >
        <Typography variant="subtitle2" sx={{ mb: 1.5, color: '#ef4444' }}>Sync Conflicts Detected</Typography>
        <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 1.5 }}>
          The following files have merge conflicts. Resolve each file or abort the merge.
        </Typography>
        <Box sx={{ maxHeight: 240, overflow: 'auto', mb: 2 }}>
          {files.map(file => (
            <Box key={file} sx={{ display: 'flex', alignItems: 'center', gap: 1, py: 0.5 }}>
              <Typography variant="caption" sx={{ flex: 1, fontFamily: 'monospace', fontSize: '0.7rem' }}>
                {file}
              </Typography>
              <Button
                variant="outlined"
                size="small"
                onClick={async () => {
                  setResolving(file);
                  await onResolve(file);
                  setResolving(null);
                }}
                disabled={resolving === file}
                sx={{ textTransform: 'none', fontSize: '0.65rem', minWidth: 60 }}
              >
                {resolving === file ? '...' : 'Accept'}
              </Button>
            </Box>
          ))}
        </Box>
        <Box sx={{ display: 'flex', gap: 1, justifyContent: 'flex-end' }}>
          <Button
            variant="outlined"
            size="small"
            color="error"
            onClick={onAbort}
            sx={{ textTransform: 'none' }}
          >
            Abort Merge
          </Button>
          <Button
            variant="contained"
            size="small"
            onClick={onResolveAll}
            sx={{ textTransform: 'none', bgcolor: 'var(--primary-500)' }}
          >
            Resolve All
          </Button>
        </Box>
      </Box>
    </Box>
  );
}
