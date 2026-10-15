import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import Button from '@mui/material/Button';
import RefreshIcon from '@mui/icons-material/Refresh';
import { useTranslation } from 'react-i18next';
import ReminderSections from './ReminderSections';
import { useRemindersPanel } from './RemindersPanel.shared';

export default function RemindersPanelDesktop() {
  const { t } = useTranslation();
  const { grouped, loading, error, refresh, snooze, complete, open, humanize } = useRemindersPanel();

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', height: '100%', overflow: 'hidden' }}>
      <Box
        sx={{
          px: 2,
          py: 1.5,
          borderBottom: 1,
          borderColor: 'divider',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
        }}
      >
        <Typography variant="h6" sx={{ fontWeight: 700 }}>
          {t('reminders.panel.title')}
        </Typography>
        <Button size="small" startIcon={<RefreshIcon />} onClick={refresh} sx={{ textTransform: 'none' }}>
          {t('reminders.panel.due')}
        </Button>
      </Box>
      <Box sx={{ flex: 1, overflow: 'auto', p: 2 }}>
        <ReminderSections
          grouped={grouped}
          loading={loading}
          error={error}
          onRefresh={refresh}
          onSnooze={snooze}
          onComplete={complete}
          onOpen={open}
          humanize={humanize}
        />
      </Box>
    </Box>
  );
}
