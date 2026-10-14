import { useState } from 'react';
import Dialog from '@mui/material/Dialog';
import DialogTitle from '@mui/material/DialogTitle';
import DialogContent from '@mui/material/DialogContent';
import DialogActions from '@mui/material/DialogActions';
import Button from '@mui/material/Button';
import Box from '@mui/material/Box';
import { useTranslation } from 'react-i18next';
import { dueToLocalInput, localInputToUtcIso, formatDueLocal } from '../lib/reminders/reminderUtils';

interface Props {
  open: boolean;
  /** Existing due as an ISO string, or null when setting a new reminder. */
  initialDue: string | null;
  /** Called with the chosen UTC ISO string, or null to clear. */
  onSave: (isoUtc: string | null) => void;
  onClose: () => void;
}

function tomorrow0900(): Date {
  const d = new Date();
  d.setDate(d.getDate() + 1);
  d.setHours(9, 0, 0, 0);
  return d;
}

function evening1800(): Date {
  const d = new Date();
  d.setHours(18, 0, 0, 0);
  return d;
}

function inOneHour(): Date {
  return new Date(Date.now() + 60 * 60 * 1000);
}

/** Shared date-time dialog for setting / changing / clearing a block reminder. */
export default function ReminderDialog({ open, initialDue, onSave, onClose }: Props) {
  const { t } = useTranslation();
  const [value, setValue] = useState(() => dueToLocalInput(initialDue ?? ''));

  // Re-sync the input whenever the dialog (re)opens for a different block.
  const [lastInitial, setLastInitial] = useState(initialDue);
  if (lastInitial !== initialDue) {
    setLastInitial(initialDue);
    setValue(dueToLocalInput(initialDue ?? ''));
  }

  function applyQuick(d: Date) {
    setValue(formatDueLocal(d));
  }

  function handleSave() {
    let iso = localInputToUtcIso(value);
    if (!iso) {
      // Default an empty/invalid input to now rather than discarding the save.
      iso = new Date().toISOString();
      setValue(dueToLocalInput(iso));
    }
    onSave(iso);
  }

  return (
    <Dialog open={open} onClose={onClose} fullWidth maxWidth="xs">
      <DialogTitle>
        {initialDue ? t('reminders.dialog.change_reminder') : t('reminders.dialog.set_reminder')}
      </DialogTitle>
      <DialogContent>
        <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2, pt: 1 }}>
          <input
            type="datetime-local"
            value={value}
            onChange={(e) => setValue(e.target.value)}
            style={{ width: '100%', fontSize: '1rem', padding: '0.5rem' }}
          />
          <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 1 }}>
            <Button size="small" variant="outlined" onClick={() => applyQuick(inOneHour())}>
              {t('reminders.dialog.in_1_hour')}
            </Button>
            <Button size="small" variant="outlined" onClick={() => applyQuick(tomorrow0900())}>
              {t('reminders.dialog.tomorrow_0900')}
            </Button>
            <Button size="small" variant="outlined" onClick={() => applyQuick(evening1800())}>
              {t('reminders.dialog.evening_1800')}
            </Button>
          </Box>
          {value && !localInputToUtcIso(value) && (
            <Box component="span" sx={{ color: 'error.main', fontSize: '0.75rem' }}>
              {t('reminders.dialog.invalid')}
            </Box>
          )}
        </Box>
      </DialogContent>
      <DialogActions>
        {initialDue && (
          <Button size="small" onClick={() => onSave(null)}>
            {t('reminders.dialog.clear_reminder')}
          </Button>
        )}
        <Box sx={{ flex: 1 }} />
        <Button size="small" onClick={onClose}>
          {t('reminders.dialog.cancel')}
        </Button>
        <Button size="small" variant="contained" onClick={handleSave}>
          {t('reminders.dialog.save')}
        </Button>
      </DialogActions>
    </Dialog>
  );
}
