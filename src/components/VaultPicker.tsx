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
import { useStore } from '../stores/appStore';

export default function VaultPicker() {
  const { pickVaultDirectory, error, persistentError } = useStore(useShallow(
    s => ({ pickVaultDirectory: s.pickVaultDirectory, error: s.error, persistentError: s.persistentError }),
  ));

  const visibleError = persistentError ?? error;

  return (
    <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100vh', width: '100vw', bgcolor: 'background.default' }}>
      <Card sx={{ maxWidth: 480, mx: 2, width: '100%' }} elevation={8}>
        <CardContent sx={{ p: 4 }}>
          <Box sx={{ textAlign: 'center', mb: 4 }}>
            <Typography variant="h4" sx={{ fontWeight: 700, mb: 1 }}>
              Welcome to Stratum
            </Typography>
            <Typography variant="body2" color="text.secondary">
              Select or create a vault to get started.
              Your notes are stored as plain Markdown files.
            </Typography>
          </Box>

          {visibleError && (
            <Alert
              severity="error"
              sx={{ mb: 2 }}
              action={
                <IconButton
                  size="small"
                  aria-label="Dismiss error"
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
              Choose Vault Folder
            </Button>
            <Typography variant="caption" color="text.secondary" align="center">
              Opens a folder picker to select or create a vault directory.
              A{' '}
              <Box component="code" sx={{ px: 0.5, py: 0.25, bgcolor: 'action.hover', borderRadius: 0.5, fontSize: '0.7rem' }}>
                .pkm
              </Box>{' '}
              folder will be created inside.
            </Typography>
          </Box>
        </CardContent>
      </Card>
    </Box>
  );
}
