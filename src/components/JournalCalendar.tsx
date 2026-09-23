import { useState } from 'react';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import IconButton from '@mui/material/IconButton';
import Popover from '@mui/material/Popover';
import Dialog from '@mui/material/Dialog';
import DialogTitle from '@mui/material/DialogTitle';
import DialogContent from '@mui/material/DialogContent';
import ChevronLeftIcon from '@mui/icons-material/ChevronLeft';
import ChevronRightIcon from '@mui/icons-material/ChevronRight';
import CloseIcon from '@mui/icons-material/Close';
import { useResponsive } from '../lib/hooks/useResponsive';

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

// Shared calendar grid rendered inside either the desktop Popover or the
// mobile full-screen Dialog. Keeping the grid and month-state in ONE component
// prevents the desktop/mobile drift that previously existed when the mobile
// panel re-implemented an identical calendar inline (see audit 2.1).
interface GridProps {
  viewMonth: number;
  viewYear: number;
  today: string;
  journalDates: Set<string>;
  onSelect: (date: string) => void;
  onPrevMonth: () => void;
  onNextMonth: () => void;
  touch: boolean;
}

function CalendarGrid({
  viewMonth,
  viewYear,
  today,
  journalDates,
  onSelect,
  onPrevMonth,
  onNextMonth,
  touch,
}: GridProps) {
  const daysInMonth = new Date(viewYear, viewMonth + 1, 0).getDate();
  const firstDay = new Date(viewYear, viewMonth, 1).getDay();

  return (
    <>
      <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', mb: 1 }}>
        <IconButton size={touch ? 'medium' : 'small'} onClick={onPrevMonth} aria-label="Previous month">
          <ChevronLeftIcon />
        </IconButton>
        <Typography variant="subtitle2" sx={{ fontWeight: 600 }}>
          {MONTHS[viewMonth]} {viewYear}
        </Typography>
        <IconButton size={touch ? 'medium' : 'small'} onClick={onNextMonth} aria-label="Next month">
          <ChevronRightIcon />
        </IconButton>
      </Box>
      <Box sx={{ display: 'grid', gridTemplateColumns: 'repeat(7, 1fr)', gap: 0.25, textAlign: 'center', mb: 0.5 }}>
        {DAY_HEADERS.map((d) => (
          <Typography key={d} variant="caption" color="text.disabled" sx={{ py: 0.5 }}>{d}</Typography>
        ))}
      </Box>
      <Box sx={{ display: 'grid', gridTemplateColumns: 'repeat(7, 1fr)', gap: touch ? 0.5 : 0.25 }}>
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
              onClick={() => onSelect(date)}
              sx={{
                minWidth: 0,
                // Larger touch target on mobile for accessibility.
                p: touch ? 1 : 0.5,
                fontSize: '0.75rem',
                borderRadius: 1,
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
    </>
  );
}

/**
 * Journal day-picker shared by the desktop and mobile journal panels.
 *
 * Intentional platform deviation (documented): on desktop the calendar is an
 * anchored Popover; on mobile it renders as a full-screen Dialog (better touch
 * target + focus for a 7-column date grid). The grid, month-state, and day
 * selection logic are shared — only the chrome differs — so there is a single
 * source of truth and no fork to drift (audit 2.1/2.4).
 */
export default function JournalCalendar({
  open,
  onClose,
  onDateSelect,
  anchorEl,
  journalDates = new Set(),
  today: todayProp,
}: Props) {
  const { isMobile } = useResponsive();
  const now = new Date();
  const today = todayProp ?? formatDate(now);
  const [viewMonth, setViewMonth] = useState(now.getMonth());
  const [viewYear, setViewYear] = useState(now.getFullYear());

  const goPrev = () => {
    if (viewMonth === 0) { setViewMonth(11); setViewYear((y) => y - 1); }
    else setViewMonth((m) => m - 1);
  };

  const goNext = () => {
    if (viewMonth === 11) { setViewMonth(0); setViewYear((y) => y + 1); }
    else setViewMonth((m) => m + 1);
  };

  const selectDate = (date: string) => {
    onDateSelect(date);
    onClose();
  };

  const grid = (
    <CalendarGrid
      viewMonth={viewMonth}
      viewYear={viewYear}
      today={today}
      journalDates={journalDates}
      onSelect={selectDate}
      onPrevMonth={goPrev}
      onNextMonth={goNext}
      touch={isMobile}
    />
  );

  if (isMobile) {
    return (
      <Dialog fullScreen open={open} onClose={onClose} aria-label="Journal calendar">
        <DialogTitle sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', px: 1 }}>
          <Typography variant="subtitle1" sx={{ fontWeight: 600 }}>Calendar</Typography>
          <IconButton size="small" onClick={onClose} aria-label="Close calendar">
            <CloseIcon />
          </IconButton>
        </DialogTitle>
        <DialogContent sx={{ pb: 2 }}>{grid}</DialogContent>
      </Dialog>
    );
  }

  return (
    <Popover
      open={open}
      onClose={onClose}
      anchorEl={anchorEl}
      anchorOrigin={{ vertical: 'bottom', horizontal: 'left' }}
      transformOrigin={{ vertical: 'top', horizontal: 'left' }}
      slotProps={{ paper: { sx: { p: 2 } } }}
    >
      {grid}
    </Popover>
  );
}
