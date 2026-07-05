import { createPortal } from 'react-dom';
import Box from '@mui/material/Box';
import CircularProgress from '@mui/material/CircularProgress';
import Typography from '@mui/material/Typography';

interface Props {
  loading: boolean;
  message?: string;
}

export default function AILoadingOverlay({ loading, message = 'AI is thinking...' }: Props) {
  if (!loading) return null;
  return createPortal(
    <Box sx={{ position: 'fixed', inset: 0, zIndex: 9999, display: 'flex', alignItems: 'center', justifyContent: 'center', bgcolor: 'rgba(0,0,0,0.3)' }}>
      <CircularProgress size={20} sx={{ mr: 1.5 }} />
      <Typography variant="body2">{message}</Typography>
    </Box>,
    document.body,
  );
}
