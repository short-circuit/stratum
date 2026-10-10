import Box from '@mui/material/Box';
import CircularProgress from '@mui/material/CircularProgress';
import Typography from '@mui/material/Typography';
import { useTranslation } from 'react-i18next';

interface Props {
  message?: string;
  /**
   * When true, renders as absolute overlay. When false, renders as inline centered block.
   * @default true
   */
  overlay?: boolean;
}

export default function LoadingOverlay({ message, overlay = true }: Props) {
  const { t } = useTranslation();
  const resolvedMessage = message ?? t('common.loading');
  const content = (
    <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 1.5 }}>
      <CircularProgress size={20} />
      {resolvedMessage && <Typography variant="body2" color="text.secondary">{resolvedMessage}</Typography>}
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
