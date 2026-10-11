import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import Switch from '@mui/material/Switch';
import TextField from '@mui/material/TextField';
import FormControlLabel from '@mui/material/FormControlLabel';
import FormControl from '@mui/material/FormControl';
import InputLabel from '@mui/material/InputLabel';
import Select from '@mui/material/Select';
import MenuItem from '@mui/material/MenuItem';
import { useTranslation } from 'react-i18next';
import type { TraySettings } from '../../stores/settingsStore';

interface TrayTabProps {
  tray: TraySettings;
  onTrayChange: (patch: Partial<TraySettings>) => void;
}

export default function TrayTab({ tray, onTrayChange }: TrayTabProps) {
  const { t } = useTranslation();
  return (
    <Box sx={{ maxWidth: 480 }}>
      <Typography variant="subtitle2" sx={{ mb: 1.5, color: 'text.secondary' }}>
        {t('settings.tray')}
      </Typography>

      <FormControlLabel
        control={<Switch checked={tray.enabled} onChange={e => onTrayChange({ enabled: e.target.checked })} />}
        label={t('settings.tray_enabled')}
        sx={{ mb: 1 }}
      />
      <FormControlLabel
        control={<Switch checked={tray.close_to_tray} onChange={e => onTrayChange({ close_to_tray: e.target.checked })} />}
        label={t('settings.tray_close_to_tray')}
        sx={{ mb: 2.5, display: 'flex' }}
      />

      <TextField
        label={t('settings.tray_shortcut')}
        value={tray.quick_capture_shortcut}
        onChange={e => onTrayChange({ quick_capture_shortcut: e.target.value })}
        fullWidth
        size="small"
        sx={{ mb: 2.5 }}
      />

      <FormControl size="small" sx={{ mb: 2.5, minWidth: 220 }}>
        <InputLabel>{t('settings.tray_capture_target')}</InputLabel>
        <Select
          label={t('settings.tray_capture_target')}
          value={tray.capture_target}
          onChange={e => onTrayChange({ capture_target: e.target.value as string })}
        >
          <MenuItem value="journal">{t('settings.tray_capture_target_journal')}</MenuItem>
          <MenuItem value="inbox">{t('settings.tray_capture_target_inbox')}</MenuItem>
        </Select>
      </FormControl>

      {tray.capture_target === 'inbox' && (
        <TextField
          label={t('settings.tray_inbox_path')}
          value={tray.inbox_path}
          onChange={e => onTrayChange({ inbox_path: e.target.value })}
          fullWidth
          size="small"
        />
      )}
    </Box>
  );
}
