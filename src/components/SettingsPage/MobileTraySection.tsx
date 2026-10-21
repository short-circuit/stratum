import { useState } from 'react';
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
import { SettingsSection, SettingsAccordion } from '../ui/settings';

// ---------------------------------------------------------------------------
// Mobile tray section — mirrors the desktop TrayTab controls on the mobile
// settings surface, exposed as a collapsible accordion (E11.F3).
//
// The tray icon / close-to-tray behaviour remains desktop-only, but the tray
// settings (shortcut, capture target, inbox path) are now reachable on mobile
// through this collapsible section. Default collapsed to keep the settings
// list compact.
// ---------------------------------------------------------------------------

export interface MobileTraySectionProps {
  tray: TraySettings;
  onTrayChange: (patch: Partial<TraySettings>) => void;
}

export default function MobileTraySection({ tray, onTrayChange }: MobileTraySectionProps) {
  const { t } = useTranslation();
  const [expanded, setExpanded] = useState(false);

  return (
    <SettingsSection title={t('settings.tray')} variant="mobile" divider>
      <SettingsAccordion
        summary={
          <Typography variant="body2" color="text.secondary">
            {expanded ? t('settings.hide_tray_config') : t('settings.configure_tray')}
          </Typography>
        }
        expanded={expanded}
        onChange={setExpanded}
      >
        <Box sx={{ display: 'flex', flexDirection: 'column', gap: 1.5 }}>
          <FormControlLabel
            control={<Switch checked={tray.enabled} onChange={e => onTrayChange({ enabled: e.target.checked })} />}
            label={t('settings.tray_enabled')}
            sx={{ mb: 0.5 }}
          />
          <FormControlLabel
            control={<Switch checked={tray.close_to_tray} onChange={e => onTrayChange({ close_to_tray: e.target.checked })} />}
            label={t('settings.tray_close_to_tray')}
            sx={{ mb: 0.5, display: 'flex' }}
          />
          <TextField
            label={t('settings.tray_shortcut')}
            value={tray.quick_capture_shortcut}
            onChange={e => onTrayChange({ quick_capture_shortcut: e.target.value })}
            fullWidth
            size="small"
          />
          <FormControl size="small" fullWidth>
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
      </SettingsAccordion>
    </SettingsSection>
  );
}
