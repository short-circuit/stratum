import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import List from '@mui/material/List';
import ListItem from '@mui/material/ListItem';
import ListItemText from '@mui/material/ListItemText';
import Chip from '@mui/material/Chip';
import Alert from '@mui/material/Alert';
import { useTranslation } from 'react-i18next';
import EmptyState from '../ui/EmptyState';
import type { ReminderGroup } from './RemindersPanel.shared';
import type { ReminderDto } from '../../lib/types';

interface Props {
  grouped: ReminderGroup;
  loading: boolean;
  error: string | null;
  onRefresh: () => void;
  onSnooze: (pagePath: string, blockId: string, minutes: number) => void;
  onComplete: (pagePath: string, blockId: string) => void;
  onOpen: (pagePath: string, blockId: string) => void;
  humanize: (r: ReminderDto) => string;
}

function Section({
  title,
  color,
  items,
  ...actions
}: {
  title: string;
  color: 'error' | 'warning' | 'default';
  items: ReminderDto[];
  onSnooze: Props['onSnooze'];
  onComplete: Props['onComplete'];
  onOpen: Props['onOpen'];
  humanize: Props['humanize'];
}) {
  if (items.length === 0) return null;
  return (
    <Box sx={{ mb: 2 }}>
      <Typography variant="subtitle2" sx={{ color: `${color}.main`, mb: 0.5 }}>
        {title} ({items.length})
      </Typography>
      <List dense disablePadding>
        {items.map((r) => (
          <ListItem
            key={r.block_id}
            sx={{ borderBottom: 1, borderColor: 'divider', px: 0.5 }}
          >
            <ListItemText
              primary={<Typography variant="body2">{r.content}</Typography>}
              secondary={`${r.page_title ? r.page_title + ' · ' : ''}${actions.humanize(r)}`}
            />
            <Box sx={{ display: 'flex', gap: 0.5, flexWrap: 'wrap', justifyContent: 'flex-end' }}>
              <Chip label="10m" size="small" onClick={() => actions.onSnooze(r.page_path, r.block_id, 10)} />
              <Chip label="1h" size="small" onClick={() => actions.onSnooze(r.page_path, r.block_id, 60)} />
              <Chip label="Done" size="small" color="success" variant="outlined" onClick={() => actions.onComplete(r.page_path, r.block_id)} />
              <Chip label="Open" size="small" color="primary" variant="outlined" onClick={() => actions.onOpen(r.page_path, r.block_id)} />
            </Box>
          </ListItem>
        ))}
      </List>
    </Box>
  );
}

/** Shared list rendering for the reminders panel, used by desktop + mobile. */
export default function ReminderSections({ grouped, loading, error, onRefresh, onSnooze, onComplete, onOpen, humanize }: Props) {
  const { t } = useTranslation();
  const total = grouped.overdue.length + grouped.today.length + grouped.upcoming.length;

  if (loading && total === 0) {
    return (
      <Box sx={{ display: 'flex', justifyContent: 'center', py: 6 }}>
        <Box component="span" sx={{ width: 24, height: 24, borderRadius: '50%', border: 2, borderColor: 'primary.main', borderTopColor: 'transparent', animation: 'spin 0.6s linear infinite' }} />
      </Box>
    );
  }

  if (error && total === 0) {
    return <Alert severity="error" onClose={onRefresh as unknown as () => void}>{error}</Alert>;
  }

  if (total === 0) {
    return (
      <EmptyState
        icon={null}
        message={t('reminders.panel.empty')}
        actionLabel={t('reminders.panel.due')}
        onAction={onRefresh}
      />
    );
  }

  const actions = { onSnooze, onComplete, onOpen, humanize };
  return (
    <Box>
      <Section title={t('reminders.panel.overdue')} color="error" items={grouped.overdue} {...actions} />
      <Section title={t('reminders.panel.today')} color="warning" items={grouped.today} {...actions} />
      <Section title={t('reminders.panel.upcoming')} color="default" items={grouped.upcoming} {...actions} />
    </Box>
  );
}
