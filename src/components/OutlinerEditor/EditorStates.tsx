/**
 * Shared loading and error placeholder states for the Outliner editor.
 *
 * Extracted verbatim from OutlinerEditor.desktop.tsx / OutlinerEditor.mobile.tsx
 * during the t_dc58c24c sizing-gate decomposition so the platform variants stay
 * under the 400-line component gate. These are pure presentational moves — no
 * behaviour change.
 *
 * @module OutlinerEditor/EditorStates
 */

import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import Button from '@mui/material/Button';
import Alert from '@mui/material/Alert';
import CircularProgress from '@mui/material/CircularProgress';
import { useTranslation } from 'react-i18next';

export function EditorLoadingState({ pagePath }: { pagePath: string }) {
  const { t } = useTranslation();
  return (
    <Box
      sx={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        height: 256,
      }}
    >
      <Box sx={{ textAlign: 'center' }}>
        <CircularProgress size={20} sx={{ mb: 1.5 }} />
        <Typography variant="body2" color="text.secondary">
          {t('editor.loading_editor')}
        </Typography>
        <Typography
          variant="caption"
          color="text.disabled"
          sx={{ display: 'block', mt: 0.25 }}
        >
          {pagePath}
        </Typography>
      </Box>
    </Box>
  );
}

export function EditorErrorState({
  error,
  onRetry,
  retryKey,
}: {
  error: string;
  onRetry: () => void;
  retryKey: string;
}) {
  const { t } = useTranslation();
  return (
    <Alert
      severity="error"
      sx={{ m: 2 }}
      action={
        <Button size="small" color="inherit" onClick={onRetry}>
          {t(retryKey)}
        </Button>
      }
    >
      {t('editor.editor_error')} {error}
    </Alert>
  );
}
