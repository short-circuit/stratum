import { useState } from 'react';
import Box from '@mui/material/Box';
import TextField from '@mui/material/TextField';
import Button from '@mui/material/Button';
import Typography from '@mui/material/Typography';
import UploadFileIcon from '@mui/icons-material/UploadFile';
import { useTranslation } from 'react-i18next';
import ImportDialog from '../ImportDialog';

interface VaultTabProps {
  vaultPath: string;
  onVaultPathChange: (path: string) => void;
  onBrowse: () => void;
}

export default function VaultTab({ vaultPath, onVaultPathChange, onBrowse }: VaultTabProps) {
  const { t } = useTranslation();
  const [importOpen, setImportOpen] = useState(false);

  return (
    <Box>
      <Typography variant="subtitle2" sx={{ mb: 1.5, color: 'text.secondary' }}>{t('settings.vault')}</Typography>
      <Box sx={{ maxWidth: 480 }}>
        <TextField
          label={t('settings.vault_path')}
          value={vaultPath}
          onChange={e => onVaultPathChange(e.target.value)}
          fullWidth
          size="small"
          sx={{ mb: 1, '& .MuiInputBase-input': { fontFamily: 'monospace' } }}
        />
        <Box sx={{ display: 'flex', gap: 1 }}>
          <Button variant="outlined" size="small" onClick={onBrowse}>
            {t('settings.browse')}
          </Button>
          <Button
            variant="outlined"
            size="small"
            startIcon={<UploadFileIcon />}
            onClick={() => setImportOpen(true)}
          >
            {t('importers.import_notes')}
          </Button>
        </Box>
      </Box>
      <ImportDialog open={importOpen} onClose={() => setImportOpen(false)} />
    </Box>
  );
}
