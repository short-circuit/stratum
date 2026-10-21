import Typography from '@mui/material/Typography';
import Switch from '@mui/material/Switch';
import FormControlLabel from '@mui/material/FormControlLabel';
import { useTranslation } from 'react-i18next';
import type { ReminderSettings } from '../../stores/settingsStore';
import { SettingsSection } from '../ui/settings';

interface Props {
  reminders: ReminderSettings;
  updateReminders: (patch: Partial<ReminderSettings>) => void;
}

/** Notifications settings section (mobile). */
export default function MobileNotificationsSection({ reminders, updateReminders }: Props) {
  const { t } = useTranslation();
  return (
    <SettingsSection title={t('settings.notifications.title')} variant="mobile" divider>
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
    </SettingsSection>
  );
}
