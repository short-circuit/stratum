import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import Switch from '@mui/material/Switch';
import FormControlLabel from '@mui/material/FormControlLabel';
import { useTranslation } from 'react-i18next';
import type { ReminderSettings } from '../../stores/settingsStore';

interface Props {
  reminders: ReminderSettings;
  onRemindersChange: (patch: Partial<ReminderSettings>) => void;
}

/** Notifications settings tab (desktop). */
export default function NotificationsTab({ reminders, onRemindersChange }: Props) {
  const { t } = useTranslation();
  return (
    <Box sx={{ maxWidth: 480 }}>
      <Typography variant="subtitle2" sx={{ mb: 1.5, color: 'text.secondary' }}>
        {t('settings.notifications.title')}
      </Typography>

      <FormControlLabel
        control={<Switch checked={reminders.enabled} onChange={e => onRemindersChange({ enabled: e.target.checked })} />}
        label={t('settings.notifications.enable')}
        sx={{ mb: 1 }}
      />
      <FormControlLabel
        control={<Switch checked={reminders.sound} onChange={e => onRemindersChange({ sound: e.target.checked })} />}
        label={t('settings.notifications.sound')}
        sx={{ mb: 1, display: 'flex' }}
      />
      <Typography variant="caption" color="text.secondary">
        {t('settings.notifications.sound_desc')}
      </Typography>
    </Box>
  );
}
