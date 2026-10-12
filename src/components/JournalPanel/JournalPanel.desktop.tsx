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
import VoiceCaptureButton from '../VoiceCaptureButton';
import { useJournalPanel, formatDisplayDate } from './JournalPanel.shared';
import { useTranslation } from 'react-i18next';

/**
 * Desktop journal panel.
 *
 * Provides the documented Prev/Next day arrows and a clickable date header
 * that opens the shared JournalCalendar (audit 2.3 — previously neither
 * variant offered these, contradicting docs/guide/journal.md).
 */
export default function JournalPanelDesktop() {
  const { t } = useTranslation();
  const {
    today,
    todayPagePath,
    todayExists,
    journalLoading,
    journalError,
    retryJournal,
    repairJournal,
    editorNonce,
    bumpEditorNonce,
    loadPages,
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
    calendarAnchorEl,
    setCalendarAnchorEl,
    handleDateSelect,
    createNewDay,
  } = useJournalPanel();

  return (
    <Box sx={{ height: '100%', display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
      <Box ref={scrollRootRef as React.Ref<HTMLDivElement>} sx={{ flex: 1, overflow: 'auto', p: 3 }}>
      <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', px: 1, mb: 0.5 }}>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
          <IconButton
            size="small"
            onClick={() => createNewDay(-1)}
            aria-label={t('journal.previous_day')}
          >
            <ChevronLeftIcon />
          </IconButton>
          <Typography
            variant="subtitle2"
            sx={{ fontWeight: 600, color: 'text.secondary', cursor: 'pointer', '&:hover': { textDecoration: 'underline' } }}
            onClick={(e) => {
              setCalendarAnchorEl(e.currentTarget);
              setCalendarOpen((o) => !o);
            }}
          >
            {formatDisplayDate(targetDate && targetDate !== today ? targetDate : today)}
          </Typography>
          <IconButton
            size="small"
            onClick={() => createNewDay(1)}
            aria-label={t('journal.next_day')}
          >
            <ChevronRightIcon />
          </IconButton>
        </Box>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
          <IconButton
            size="small"
            onClick={(e) => {
              setCalendarAnchorEl(e.currentTarget);
              setCalendarOpen((o) => !o);
            }}
            aria-label={t('journal_calendar.calendar')}
          >
            <CalendarMonthIcon fontSize="small" />
          </IconButton>
          <VoiceCaptureButton
            targetPath={todayPagePath}
            onInserted={() => {
              bumpEditorNonce();
              loadPages();
            }}
          />
        </Box>
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
            {t('common.retry')}
          </Button>
          <Button variant="outlined" size="small" onClick={repairJournal} sx={{ ml: 1 }}>            {t('journal.repair_database')}
          </Button>
        </Box>
      ) : journalLoading || !todayExists ? (
        <CircularProgress size={20} sx={{ display: 'block', mx: 'auto', my: 4 }} />
      ) : (
        // Wrapped in an auto-height Box so today's editor sizes to its
        // content rather than stretching to the viewport height of the
        // scroll container (see fill-viewport fix).
        <Box>
          <OutlinerEditor key={editorNonce} pagePath={todayPagePath} minHeight="0" />
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
      </Box>
    </Box>
  );
}
