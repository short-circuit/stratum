import { useState } from 'react';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import IconButton from '@mui/material/IconButton';
import Popover from '@mui/material/Popover';
import ChevronLeftIcon from '@mui/icons-material/ChevronLeft';
import ChevronRightIcon from '@mui/icons-material/ChevronRight';

interface Props {
  open: boolean;
  onClose: () => void;
  onDateSelect: (date: string) => void;
  anchorEl: HTMLElement | null;
  journalDates?: Set<string>;
  today?: string;
}

function pad(n: number): string {
  return String(n).padStart(2, '0');
}

function formatDate(d: Date): string {
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

const MONTHS = [
  'January', 'February', 'March', 'April', 'May', 'June',
  'July', 'August', 'September', 'October', 'November', 'December',
];

const DAY_HEADERS = ['Su', 'Mo', 'Tu', 'We', 'Th', 'Fr', 'Sa'];

export default function JournalCalendar({
  open,
  onClose,
  onDateSelect,
  anchorEl,
  journalDates = new Set(),
  today: todayProp,
}: Props) {
  const now = new Date();
  const today = todayProp ?? formatDate(now);
  const [viewMonth, setViewMonth] = useState(now.getMonth());
  const [viewYear, setViewYear] = useState(now.getFullYear());

  const daysInMonth = new Date(viewYear, viewMonth + 1, 0).getDate();
  const firstDay = new Date(viewYear, viewMonth, 1).getDay();

  const goPrev = () => {
    if (viewMonth === 0) { setViewMonth(11); setViewYear(viewYear - 1); }
    else setViewMonth(viewMonth - 1);
  };

  const goNext = () => {
    if (viewMonth === 11) { setViewMonth(0); setViewYear(viewYear + 1); }
    else setViewMonth(viewMonth + 1);
  };

  return (
    <Popover
      open={open}
      onClose={onClose}
      anchorEl={anchorEl}
      anchorOrigin={{ vertical: 'bottom', horizontal: 'left' }}
      transformOrigin={{ vertical: 'top', horizontal: 'left' }}
      slotProps={{ paper: { sx: { p: 2 } } }}
    >
      <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', mb: 1 }}>
        <IconButton size="small" onClick={goPrev}><ChevronLeftIcon /></IconButton>
        <Typography variant="subtitle2" sx={{ fontWeight: 600 }}>
          {MONTHS[viewMonth]} {viewYear}
        </Typography>
        <IconButton size="small" onClick={goNext}><ChevronRightIcon /></IconButton>
      </Box>
      <Box sx={{ display: 'grid', gridTemplateColumns: 'repeat(7, 1fr)', gap: 0.25, textAlign: 'center', mb: 0.5 }}>
        {DAY_HEADERS.map(d => (
          <Typography key={d} variant="caption" color="text.disabled" sx={{ py: 0.5 }}>{d}</Typography>
        ))}
      </Box>
      <Box sx={{ display: 'grid', gridTemplateColumns: 'repeat(7, 1fr)', gap: 0.25 }}>
        {Array.from({ length: firstDay }).map((_, i) => <Box key={`e-${i}`} />)}
        {Array.from({ length: daysInMonth }).map((_, i) => {
          const day = i + 1;
          const date = `${viewYear}-${pad(viewMonth + 1)}-${pad(day)}`;
          const isTodayDate = date === today;
          const hasJournal = journalDates.has(date);
          return (
            <IconButton
              key={day}
              size="small"
              onClick={() => { onDateSelect(date); onClose(); }}
              sx={{
                minWidth: 0, p: 0.5, fontSize: '0.75rem', borderRadius: 1,
                fontWeight: isTodayDate ? 700 : hasJournal ? 600 : 400,
                opacity: hasJournal || isTodayDate ? 1 : 0.4,
                bgcolor: isTodayDate ? 'primary.light' : hasJournal ? 'action.selected' : 'transparent',
                color: isTodayDate ? 'primary.contrastText' : undefined,
                '&:hover': { bgcolor: isTodayDate ? 'primary.light' : 'action.hover' },
              }}
            >
              {day}
            </IconButton>
          );
        })}
      </Box>
    </Popover>
  );
}
