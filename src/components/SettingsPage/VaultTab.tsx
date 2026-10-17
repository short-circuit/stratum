import { useState } from 'react';
import Box from '@mui/material/Box';
import TextField from '@mui/material/TextField';
import Button from '@mui/material/Button';
import Typography from '@mui/material/Typography';
import ToggleButtonGroup from '@mui/material/ToggleButtonGroup';
import ToggleButton from '@mui/material/ToggleButton';
import UploadFileIcon from '@mui/icons-material/UploadFile';
import { useTranslation } from 'react-i18next';
import ImportDialog from '../ImportDialog';
import ExportSection from './ExportSection';

interface VaultTabProps {
  vaultPath: string;
  onVaultPathChange: (path: string) => void;
  onBrowse: () => void;
  pagesView: string;
  onPagesViewChange: (v: string) => void;
}

export default function VaultTab({ vaultPath, onVaultPathChange, onBrowse, pagesView, onPagesViewChange }: VaultTabProps) {
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
      <Box sx={{ mb: 2 }}>
        <Typography
          variant="caption"
          sx={{ display: 'block', color: 'text.secondary', fontWeight: 500, mb: 0.75 }}
        >
          {t('settings.pages_view')}
        </Typography>
        <ToggleButtonGroup
          exclusive
          size="small"
          value={pagesView}
          onChange={(_, v) => v && onPagesViewChange(v)}
          sx={{ mb: 1 }}
        >
          <ToggleButton value="flat">{t('settings.pages_view_flat')}</ToggleButton>
          <ToggleButton value="tree">{t('settings.pages_view_tree')}</ToggleButton>
        </ToggleButtonGroup>
      </Box>
      <ImportDialog open={importOpen} onClose={() => setImportOpen(false)} />
      <Box sx={{ mt: 2 }}>
        <ExportSection vaultPath={vaultPath} />
      </Box>
    </Box>
  );
}
