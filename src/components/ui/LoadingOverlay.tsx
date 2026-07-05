import Box from '@mui/material/Box';
import CircularProgress from '@mui/material/CircularProgress';
import Typography from '@mui/material/Typography';

interface Props {
  message?: string;
  /**
   * When true, renders as absolute overlay. When false, renders as inline centered block.
   * @default true
   */
  overlay?: boolean;
}

export default function LoadingOverlay({ message = 'Loading...', overlay = true }: Props) {
  const content = (
    <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 1.5 }}>
      <CircularProgress size={20} />
      {message && <Typography variant="body2" color="text.secondary">{message}</Typography>}
    </Box>
  );

  if (!overlay) return content;

  return (
    <Box sx={{
      position: 'absolute', inset: 0, display: 'flex', alignItems: 'center', justifyContent: 'center',
      bgcolor: 'rgba(0,0,0,0.03)', zIndex: 10,
    }}>
      {content}
    </Box>
  );
}
