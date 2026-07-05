import { useState } from 'react';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';

interface PassphraseModalProps {
  onClose: () => void;
  onSubmit: (passphrase: string) => Promise<void>;
}

export default function PassphraseModal({ onClose, onSubmit }: PassphraseModalProps) {
  const [value, setValue] = useState('');
  const [submitting, setSubmitting] = useState(false);

  const handleSubmit = async () => {
    setSubmitting(true);
    await onSubmit(value);
    setSubmitting(false);
  };

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
          maxWidth: 400,
          width: '90%',
          boxShadow: 24,
        }}
      >
        <Typography variant="subtitle2" sx={{ mb: 1 }}>SSH Key Passphrase</Typography>
        <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 1.5 }}>
          Enter the passphrase for your SSH key to authenticate with the remote.
        </Typography>
        <TextField
          size="small"
          type="password"
          placeholder="Enter passphrase..."
          value={value}
          onChange={e => setValue(e.target.value)}
          onKeyDown={e => { if (e.key === 'Enter') handleSubmit(); }}
          autoFocus
          sx={{ width: '100%', mb: 2 }}
        />
        <Box sx={{ display: 'flex', gap: 1, justifyContent: 'flex-end' }}>
          <Button variant="outlined" size="small" onClick={onClose} sx={{ textTransform: 'none' }}>
            Cancel
          </Button>
          <Button
            variant="contained"
            size="small"
            onClick={handleSubmit}
            disabled={submitting || !value}
            sx={{ textTransform: 'none', bgcolor: 'var(--primary-500)' }}
          >
            {submitting ? 'Submitting...' : 'Submit'}
          </Button>
        </Box>
      </Box>
    </Box>
  );
}
