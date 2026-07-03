import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import CardContent from '@mui/material/CardContent';
import Typography from '@mui/material/Typography';
import Button from '@mui/material/Button';
import Alert from '@mui/material/Alert';
import FolderOpenIcon from '@mui/icons-material/FolderOpen';
import { useStore } from '../stores/appStore';

export default function VaultPicker() {
  const { pickVaultDirectory, error } = useStore();

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

          {error && (
            <Alert severity="error" sx={{ mb: 2 }}>
              {error}
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
