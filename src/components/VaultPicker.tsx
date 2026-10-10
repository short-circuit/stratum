import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import CardContent from '@mui/material/CardContent';
import Typography from '@mui/material/Typography';
import Button from '@mui/material/Button';
import Alert from '@mui/material/Alert';
import IconButton from '@mui/material/IconButton';
import CloseIcon from '@mui/icons-material/Close';
import FolderOpenIcon from '@mui/icons-material/FolderOpen';
import { useShallow } from 'zustand/react/shallow';
import { useTranslation } from 'react-i18next';
import { useStore } from '../stores/appStore';

export default function VaultPicker() {
  const { t } = useTranslation();
  const { pickVaultDirectory, error, persistentError } = useStore(useShallow(
    s => ({ pickVaultDirectory: s.pickVaultDirectory, error: s.error, persistentError: s.persistentError }),
  ));

  const visibleError = persistentError ?? error;

  return (
    <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100vh', width: '100vw', bgcolor: 'background.default', paddingTop: 'var(--safe-area-top, 0px)', paddingBottom: 'var(--safe-area-bottom, 0px)' }}>
      <Card sx={{ maxWidth: 480, mx: 2, width: '100%' }} elevation={8}>
        <CardContent sx={{ p: 4 }}>
          <Box sx={{ textAlign: 'center', mb: 4 }}>
            <Typography variant="h4" sx={{ fontWeight: 700, mb: 1 }}>
              {t('vault_picker.welcome_to_stratum')}
            </Typography>
            <Typography variant="body2" color="text.secondary">
              {t('vault_picker.select_or_create_a_vault_to_get_started')}
              {t('vault_picker.your_notes_are_stored_as_plain_markdown_files')}
            </Typography>
          </Box>

          {visibleError && (
            <Alert
              severity="error"
              sx={{ mb: 2 }}
              action={
                <IconButton
                  size="small"
                  aria-label={t('vault_picker.dismiss_error')}
                  onClick={() => useStore.getState().dismissError(visibleError.id)}
                >
                  <CloseIcon fontSize="small" />
                </IconButton>
              }
            >
              {visibleError.message}
            </Alert>
          )}

          <Box sx={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 2 }}>
            <Button
              variant="contained"
              size="large"
              fullWidth
              startIcon={<FolderOpenIcon />}
              onClick={pickVaultDirectory}
              sx={{ py: 1.5, borderRadius: 2 }}
            >
              {t('vault_picker.choose_vault_folder')}
            </Button>
            <Typography variant="caption" color="text.secondary" align="center">
              {t('vault_picker.opens_a_folder_picker_to_select_or_create_a_vault_directory_')}{' '}
              <Box component="code" sx={{ px: 0.5, py: 0.25, bgcolor: 'action.hover', borderRadius: 0.5, fontSize: '0.7rem' }}>
                .pkm
              </Box>{' '}
              {t('vault_picker.folder_will_be_created_inside')}
            </Typography>
          </Box>
        </CardContent>
      </Card>
    </Box>
  );
}
