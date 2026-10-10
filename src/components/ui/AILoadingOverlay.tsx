import { createPortal } from 'react-dom';
import Box from '@mui/material/Box';
import CircularProgress from '@mui/material/CircularProgress';
import Typography from '@mui/material/Typography';
import { useTranslation } from 'react-i18next';

interface Props {
  loading: boolean;
  message?: string;
}

export default function AILoadingOverlay({ loading, message }: Props) {
  const { t } = useTranslation();
  if (!loading) return null;
  return createPortal(
    <Box sx={{ position: 'fixed', inset: 0, zIndex: 9999, display: 'flex', alignItems: 'center', justifyContent: 'center', bgcolor: 'rgba(0,0,0,0.5)' }}>
      <CircularProgress size={20} sx={{ mr: 1.5 }} />
      <Typography variant="body2">{message ?? t('common.ai_is_thinking')}</Typography>
    </Box>,
    document.body,
  );
}
