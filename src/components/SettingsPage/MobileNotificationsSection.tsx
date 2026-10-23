import { useState } from 'react';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import Switch from '@mui/material/Switch';
import FormControlLabel from '@mui/material/FormControlLabel';
import { useTranslation } from 'react-i18next';
import type { ReminderSettings } from '../../stores/settingsStore';
import { SettingsSection, SettingsAccordion } from '../ui/settings';

interface Props {
  reminders: ReminderSettings;
  updateReminders: (patch: Partial<ReminderSettings>) => void;
}

/** Notifications settings section (mobile). */
export default function MobileNotificationsSection({ reminders, updateReminders }: Props) {
  const { t } = useTranslation();
  const [expanded, setExpanded] = useState(false);
  return (
    <SettingsSection title={t('settings.notifications.title')} variant="mobile" divider>
      <SettingsAccordion
        summary={
          <Typography variant="body2" color="text.secondary">
            {expanded ? t('settings.hide_notifications_config') : t('settings.configure_notifications')}
          </Typography>
        }
        expanded={expanded}
        onChange={setExpanded}
      >
        <Box sx={{ display: 'flex', flexDirection: 'column', gap: 1.5 }}>
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
      </SettingsAccordion>
    </SettingsSection>
  );
}
