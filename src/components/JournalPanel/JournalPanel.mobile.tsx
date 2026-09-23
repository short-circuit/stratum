import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import CircularProgress from '@mui/material/CircularProgress';
import Alert from '@mui/material/Alert';
import Button from '@mui/material/Button';
import IconButton from '@mui/material/IconButton';
import CalendarMonthIcon from '@mui/icons-material/CalendarMonth';
import ChevronLeftIcon from '@mui/icons-material/ChevronLeft';
import ChevronRightIcon from '@mui/icons-material/ChevronRight';
import OutlinerEditor from '../OutlinerEditor';
import JournalCalendar from '../JournalCalendar';
import { useJournalPanel, formatDisplayDate } from './JournalPanel.shared';

/**
 * Mobile journal panel.
 *
 * Intentional platform deviations (documented):
 *  - The calendar renders as a full-screen Dialog (better touch targets and
 *    focus for the 7-column date grid) rather than the desktop anchored
 *    Popover. Both variants use the SAME shared JournalCalendar component whose
 *    chrome adapts to the breakpoint — no calendar logic is forked here
 *    (audit 2.1/2.4).
 *  - Prev/Next day arrows are rendered large enough for touch (audit 2.3).
 */
export default function JournalPanelMobile() {
  const {
    today,
    todayPagePath,
    todayExists,
    journalLoading,
    journalError,
    retryJournal,
    repairJournal,
    targetDate,
    allJournalDates,
    pastDates,
    visibleCount,
    visibleSections,
    sectionRef,
    sentinelRef,
    scrollRootRef,
    calendarOpen,
    setCalendarOpen,
    handleDateSelect,
    createNewDay,
  } = useJournalPanel();

  // Mobile scroll root: the mobile shell (MobileLayout) wraps the panel in an
  // overflow:hidden container, so this panel must be the scroll container
  // itself — every other mobile panel sets height:100% + overflow:auto on its
  // root for the same reason (audit 2.2). paddingBottom leaves room for the
  // fixed BottomNavigation + safe area.
  return (
    <Box ref={scrollRootRef as React.Ref<HTMLDivElement>} sx={{ height: '100%', overflow: 'auto', pb: 'var(--safe-area-bottom)' }}>
      <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', px: 1, pt: 0.5, pb: 1 }}>
        <IconButton size="small" onClick={() => createNewDay(-1)} aria-label="Previous day">
          <ChevronLeftIcon />
        </IconButton>
        <IconButton
          size="small"
          sx={{ borderRadius: 1 }}
          onClick={() => setCalendarOpen(true)}
          aria-label="Open calendar"
        >
          <Typography variant="subtitle2" sx={{ fontWeight: 600, color: 'text.secondary', mr: 0.5 }}>
            {formatDisplayDate(targetDate && targetDate !== today ? targetDate : today)}
          </Typography>
          <CalendarMonthIcon fontSize="small" />
        </IconButton>
        <IconButton size="small" onClick={() => createNewDay(1)} aria-label="Next day">
          <ChevronRightIcon />
        </IconButton>
      </Box>

      {journalError ? (
        <Box sx={{ px: 2, my: 2 }}>
          <Alert severity="error" sx={{ mb: 1 }}>
            {journalError}
          </Alert>
          <Button variant="outlined" size="small" onClick={retryJournal}>
            Retry
          </Button>
          <Button variant="outlined" size="small" onClick={repairJournal} sx={{ ml: 1 }}>
            Repair database
          </Button>
        </Box>
      ) : journalLoading || !todayExists ? (
        <CircularProgress size={20} sx={{ display: 'block', mx: 'auto', my: 4 }} />
      ) : (
        // Wrapped in an auto-height Box so today's editor sizes to its content
        // rather than stretching to the viewport height of the scroll
        // container (matches the past-entry sizing).
        <Box>
          <OutlinerEditor pagePath={todayPagePath} minHeight="0" />
        </Box>
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

      <JournalCalendar
        open={calendarOpen}
        onClose={() => setCalendarOpen(false)}
        onDateSelect={handleDateSelect}
        journalDates={allJournalDates}
        anchorEl={null}
      />
    </Box>
  );
}
