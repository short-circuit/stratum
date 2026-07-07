import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import CircularProgress from '@mui/material/CircularProgress';
import Alert from '@mui/material/Alert';
import Button from '@mui/material/Button';
import IconButton from '@mui/material/IconButton';
import CalendarMonthIcon from '@mui/icons-material/CalendarMonth';
import OutlinerEditor from '../OutlinerEditor';
import JournalCalendar from '../JournalCalendar';
import { useJournalPanel, formatDisplayDate } from './JournalPanel.shared';

export default function JournalPanelDesktop() {
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
    calendarAnchorEl,
    setCalendarAnchorEl,
    handleDateSelect,
  } = useJournalPanel();

  return (
    <Box sx={{ height: '100%', display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
      <Box sx={{ flex: 1, overflow: 'auto', p: 3 }}>
      <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', px: 1, mb: 0.5 }}>
        <Typography variant="subtitle2" sx={{ fontWeight: 600, color: 'text.secondary' }}>
          {formatDisplayDate(today)}
        </Typography>
        <IconButton
          size="small"
          onClick={(e) => {
            setCalendarAnchorEl(e.currentTarget);
            setCalendarOpen((o) => !o);
          }}
          aria-label="Calendar"
        >
          <CalendarMonthIcon fontSize="small" />
        </IconButton>
      </Box>

      <JournalCalendar
        open={calendarOpen}
        onClose={() => {
          setCalendarOpen(false);
          setCalendarAnchorEl(null);
        }}
        onDateSelect={handleDateSelect}
        anchorEl={calendarAnchorEl}
        journalDates={allJournalDates}
      />

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
              sx={{ pt: 1.5, pb: 0.5, px: 1, fontWeight: 600, color: 'text.secondary' }}
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
      </Box>
    </Box>
  );
}
