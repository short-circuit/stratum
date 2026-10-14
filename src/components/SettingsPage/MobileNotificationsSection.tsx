import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import Divider from '@mui/material/Divider';
import Switch from '@mui/material/Switch';
import FormControlLabel from '@mui/material/FormControlLabel';
import { useTranslation } from 'react-i18next';
import type { ReminderSettings } from '../../stores/settingsStore';

interface Props {
  reminders: ReminderSettings;
  updateReminders: (patch: Partial<ReminderSettings>) => void;
}

/** Notifications settings section (mobile). */
export default function MobileNotificationsSection({ reminders, updateReminders }: Props) {
  const { t } = useTranslation();
  return (
    <Box sx={{ px: 2, pt: 3 }}>
      <Typography variant="subtitle1" sx={{ fontWeight: 600, mb: 1 }}>
        {t('settings.notifications.title')}
      </Typography>
      <Divider sx={{ mb: 1.5 }} />
      <FormControlLabel
        control={<Switch checked={reminders.enabled} onChange={e => updateReminders({ enabled: e.target.checked })} />}
        label={t('settings.notifications.enable')}
        sx={{ mb: 0.5 }}
      />
      <FormControlLabel
        control={<Switch checked={reminders.sound} onChange={e => updateReminders({ sound: e.target.checked })} />}
        label={t('settings.notifications.sound')}
        sx={{ mb: 0.5, display: 'flex' }}
      />
      <Typography variant="caption" color="text.secondary">
        {t('settings.notifications.sound_desc')}
      </Typography>
    </Box>
  );
}
