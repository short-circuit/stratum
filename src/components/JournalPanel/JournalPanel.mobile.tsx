import { useState } from 'react';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import CircularProgress from '@mui/material/CircularProgress';
import Alert from '@mui/material/Alert';
import Button from '@mui/material/Button';
import IconButton from '@mui/material/IconButton';
import CalendarMonthIcon from '@mui/icons-material/CalendarMonth';
import CloseIcon from '@mui/icons-material/Close';
import ChevronLeftIcon from '@mui/icons-material/ChevronLeft';
import ChevronRightIcon from '@mui/icons-material/ChevronRight';
import Dialog from '@mui/material/Dialog';
import DialogTitle from '@mui/material/DialogTitle';
import DialogContent from '@mui/material/DialogContent';
import OutlinerEditor from '../OutlinerEditor';
import { useJournalPanel, formatDisplayDate } from './JournalPanel.shared';

const MONTHS = [
  'January', 'February', 'March', 'April', 'May', 'June',
  'July', 'August', 'September', 'October', 'November', 'December',
];

const DAY_HEADERS = ['Su', 'Mo', 'Tu', 'We', 'Th', 'Fr', 'Sa'];

function pad(n: number): string {
  return String(n).padStart(2, '0');
}

export default function JournalPanelMobile() {
  const {
    today,
    todayPagePath,
    todayExists,
    journalLoading,
    journalError,
    retryJournal,
    targetDate,
    allJournalDates,
    pastDates,
    visibleCount,
    visibleSections,
    sectionRef,
    sentinelRef,
    calendarOpen,
    setCalendarOpen,
    handleDateSelect,
  } = useJournalPanel();

  const now = new Date();
  const [viewMonth, setViewMonth] = useState(now.getMonth());
  const [viewYear, setViewYear] = useState(now.getFullYear());

  const daysInMonth = new Date(viewYear, viewMonth + 1, 0).getDate();
  const firstDay = new Date(viewYear, viewMonth, 1).getDay();

  const goPrev = () => {
    if (viewMonth === 0) {
      setViewMonth(11);
      setViewYear((y) => y - 1);
    } else {
      setViewMonth((m) => m - 1);
    }
  };

  const goNext = () => {
    if (viewMonth === 11) {
      setViewMonth(0);
      setViewYear((y) => y + 1);
    } else {
      setViewMonth((m) => m + 1);
    }
  };

  const handleCalendarDateSelect = (date: string) => {
    handleDateSelect(date);
    setCalendarOpen(false);
  };

  return (
    <Box sx={{ p: 2 }}>
      <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', mb: 0.5 }}>
        <Typography variant="subtitle2" sx={{ fontWeight: 600, color: 'text.secondary' }}>
          {formatDisplayDate(today)}
        </Typography>
        <IconButton
          size="small"
          onClick={() => setCalendarOpen(true)}
          aria-label="Calendar"
        >
          <CalendarMonthIcon fontSize="small" />
        </IconButton>
      </Box>

      {journalError ? (
        <Box sx={{ px: 1, my: 2 }}>
          <Alert severity="error" sx={{ mb: 1 }}>
            {journalError}
          </Alert>
          <Button variant="outlined" size="small" onClick={retryJournal}>
            Retry
          </Button>
        </Box>
      ) : journalLoading || !todayExists ? (
        <CircularProgress size={20} sx={{ display: 'block', mx: 'auto', my: 4 }} />
      ) : (
        <OutlinerEditor pagePath={todayPagePath} minHeight="0" />
      )}

      {pastDates.slice(0, visibleCount).map((date) => {
        const path = `journals/${date}.md`;
        const isVisible = visibleSections.has(date);

        return (
          <Box key={date} ref={sectionRef(date)}>
            <Typography
              variant="subtitle2"
              sx={{ pt: 1.5, pb: 0.5, fontWeight: 600, color: 'text.secondary' }}
            >
              {formatDisplayDate(date)}
            </Typography>
            {isVisible ? (
              <OutlinerEditor pagePath={path} autoFocus={date === targetDate} minHeight="0" />
            ) : (
              <CircularProgress size={14} sx={{ display: 'block', mx: 'auto', my: 2 }} />
            )}
          </Box>
        );
      })}

      {visibleCount < pastDates.length && <div ref={sentinelRef} />}

      <Dialog
        fullScreen
        open={calendarOpen}
        onClose={() => setCalendarOpen(false)}
      >
        <DialogTitle
          sx={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            px: 1,
          }}
        >
          <Typography variant="subtitle1" sx={{ fontWeight: 600 }}>
            Calendar
          </Typography>
          <IconButton size="small" onClick={() => setCalendarOpen(false)}>
            <CloseIcon />
          </IconButton>
        </DialogTitle>
        <DialogContent sx={{ pb: 2 }}>
          <Box
            sx={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              mb: 1.5,
            }}
          >
            <IconButton size="small" onClick={goPrev}>
              <ChevronLeftIcon />
            </IconButton>
            <Typography variant="subtitle2" sx={{ fontWeight: 600 }}>
              {MONTHS[viewMonth]} {viewYear}
            </Typography>
            <IconButton size="small" onClick={goNext}>
              <ChevronRightIcon />
            </IconButton>
          </Box>

          <Box
            sx={{
              display: 'grid',
              gridTemplateColumns: 'repeat(7, 1fr)',
              gap: 0.25,
              textAlign: 'center',
              mb: 0.5,
            }}
          >
            {DAY_HEADERS.map((d) => (
              <Typography key={d} variant="caption" color="text.disabled" sx={{ py: 0.5 }}>
                {d}
              </Typography>
            ))}
          </Box>

          <Box sx={{ display: 'grid', gridTemplateColumns: 'repeat(7, 1fr)', gap: 0.25 }}>
            {Array.from({ length: firstDay }).map((_, i) => (
              <Box key={`e-${i}`} />
            ))}
            {Array.from({ length: daysInMonth }).map((_, i) => {
              const day = i + 1;
              const date = `${viewYear}-${pad(viewMonth + 1)}-${pad(day)}`;
              const isTodayDate = date === today;
              const hasJournal = allJournalDates.has(date);
              return (
                <IconButton
                  key={day}
                  size="small"
                  onClick={() => handleCalendarDateSelect(date)}
                  sx={{
                    minWidth: 0,
                    p: 0.5,
                    fontSize: '0.75rem',
                    borderRadius: 1,
                    fontWeight: isTodayDate ? 700 : hasJournal ? 600 : 400,
                    opacity: hasJournal || isTodayDate ? 1 : 0.4,
                    bgcolor: isTodayDate
                      ? 'primary.light'
                      : hasJournal
                        ? 'action.selected'
                        : 'transparent',
                    color: isTodayDate ? 'primary.contrastText' : undefined,
                    '&:hover': {
                      bgcolor: isTodayDate ? 'primary.light' : 'action.hover',
                    },
                  }}
                >
                  {day}
                </IconButton>
              );
            })}
          </Box>
        </DialogContent>
      </Dialog>
    </Box>
  );
}
