import Box from '@mui/material/Box';
import { useTranslation } from 'react-i18next';
import { SettingsSection, SettingsRow } from '../ui/settings';
import type { ReminderSettings } from '../../stores/settingsStore';

interface Props {
  reminders: ReminderSettings;
  onRemindersChange: (patch: Partial<ReminderSettings>) => void;
}

/** Notifications settings tab (desktop). */
export default function NotificationsTab({ reminders, onRemindersChange }: Props) {
  const { t } = useTranslation();
  return (
    <SettingsSection title={t('settings.notifications.title')}>
      <Box sx={{ maxWidth: 480 }}>
        <SettingsRow
          label={t('settings.notifications.enable')}
          checked={reminders.enabled}
          onCheckedChange={v => onRemindersChange({ enabled: v })}
          sx={{ mb: 2 }}
        />
        <SettingsRow
          label={t('settings.notifications.sound')}
          checked={reminders.sound}
          onCheckedChange={v => onRemindersChange({ sound: v })}
          description={t('settings.notifications.sound_desc')}
        />
      </Box>
    </SettingsSection>
  );
}
