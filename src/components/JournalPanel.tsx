import { useState, useEffect, useMemo, useRef, useCallback } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import CircularProgress from '@mui/material/CircularProgress';
import IconButton from '@mui/material/IconButton';
import CalendarMonthIcon from '@mui/icons-material/CalendarMonth';
import { useStore } from '../stores/appStore';
import * as api from '../lib/commands';
import OutlinerEditor from './OutlinerEditor';
import JournalCalendar from './JournalCalendar';

function formatDate(d: Date): string {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, '0');
  const day = String(d.getDate()).padStart(2, '0');
  return `${y}-${m}-${day}`;
}

function journalPath(date: string): string {
  return `journals/${date}.md`;
}

function formatDisplayDate(dateStr: string): string {
  const d = new Date(dateStr + 'T12:00:00');
  return d.toLocaleDateString('en-US', {
    weekday: 'long', month: 'long', day: 'numeric', year: 'numeric',
  });
}

export default function JournalPanel() {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const { pages, loadPages } = useStore();
  const today = formatDate(new Date());
  const todayPagePath = journalPath(today);
  const targetDate = searchParams.get('date');

  const [visibleCount, setVisibleCount] = useState(25);
  const [visibleSections, setVisibleSections] = useState<Set<string>>(new Set());
  const [calendarOpen, setCalendarOpen] = useState(false);
  const [calendarAnchorEl, setCalendarAnchorEl] = useState<HTMLElement | null>(null);

  const observerRef = useRef<IntersectionObserver | null>(null);
  const sentinelRef = useRef<HTMLDivElement | null>(null);
  const processedTargetRef = useRef<string | null>(null);
  const targetDateRef = useRef(targetDate);
  useEffect(() => { targetDateRef.current = targetDate; }, [targetDate]);

  const allJournalDates = useMemo(() => {
    return new Set(
      pages
        .filter(p => p.path.startsWith('journals/') && p.path.endsWith('.md'))
        .map(p => p.path.slice(9, -3)),
    );
  }, [pages]);

  const pastDates = useMemo(() => {
    return pages
      .filter(p => p.path.startsWith('journals/') && p.path.endsWith('.md'))
      .map(p => p.path.slice(9, -3))
      .filter(d => /^\d{4}-\d{2}-\d{2}$/.test(d) && d !== today)
      .sort()
      .reverse();
  }, [pages, today]);

  const todayExists = useMemo(
    () => pages.length > 0 && allJournalDates.has(today),
    [pages, allJournalDates, today],
  );

  useEffect(() => {
    if (pages.length === 0 || todayExists) return;
    api.createPage(todayPagePath, today)
      .then(() => loadPages())
      .catch(err => {
        if (!String(err).includes('already exists')) {
          console.error('Failed to create journal:', err);
        }
      });
  }, [pages, todayExists, loadPages, today, todayPagePath]);

  const getObserver = useCallback(() => {
    if (!observerRef.current) {
      observerRef.current = new IntersectionObserver((entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) {
            const date = (entry.target as HTMLElement).dataset.date;
            if (date) {
              setVisibleSections(prev => {
                if (prev.has(date)) return prev;
                const next = new Set(prev);
                next.add(date);
                return next;
              });
              observerRef.current?.unobserve(entry.target);
            }
          }
        }
      }, { rootMargin: '200px' });
    }
    return observerRef.current;
  }, []);

  useEffect(() => {
    return () => observerRef.current?.disconnect();
  }, []);

  useEffect(() => {
    const el = sentinelRef.current;
    if (!el) return;
    const io = new IntersectionObserver((entries) => {
      if (entries[0]?.isIntersecting) {
        setVisibleCount(prev => prev + 25);
      }
    }, { rootMargin: '400px' });
    io.observe(el);
    return () => io.disconnect();
  }, [pastDates.length, visibleCount]);

  const sectionRef = useCallback((date: string) => (el: HTMLDivElement | null) => {
    if (el) {
      el.dataset.date = date;
      getObserver().observe(el);
      if (date === targetDateRef.current && date !== today) {
        requestAnimationFrame(() => {
          el.scrollIntoView({ behavior: 'smooth', block: 'center' });
        });
      }
    }
  }, [getObserver, today]);

  useEffect(() => {
    if (!targetDate) {
      processedTargetRef.current = null;
      return;
    }
    if (processedTargetRef.current === targetDate) return;

    if (targetDate === today) {
      window.scrollTo({ top: 0, behavior: 'smooth' });
      processedTargetRef.current = targetDate;
      return;
    }

    const idx = pastDates.indexOf(targetDate);
    if (idx === -1) return;

    requestAnimationFrame(() => {
      if (idx >= visibleCount) {
        setVisibleCount(idx + 10);
      }
      setVisibleSections(prev => {
        if (prev.has(targetDate)) return prev;
        const next = new Set(prev);
        next.add(targetDate);
        return next;
      });
    });

    requestAnimationFrame(() => {
      const el = document.querySelector(`[data-date="${targetDate}"]`);
      if (el) {
        el.scrollIntoView({ behavior: 'smooth', block: 'center' });
        processedTargetRef.current = targetDate;
      }
    });
  }, [targetDate, pastDates, today, visibleCount]);

  const handleDateSelect = async (date: string) => {
    if (!allJournalDates.has(date)) {
      try {
        await api.createPage(journalPath(date), date);
        await loadPages();
      } catch (err) {
        if (!String(err).includes('already exists')) {
          console.error('Failed to create journal:', err);
          return;
        }
      }
    }
    navigate(`/journal?date=${date}`);
  };

  return (
    <Box sx={{ p: 3 }}>
      <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', px: 1, mb: 0.5 }}>
        <Typography variant="subtitle2" sx={{ fontWeight: 600, color: 'text.secondary' }}>
          {formatDisplayDate(today)}
        </Typography>
        <IconButton
          size="small"
          onClick={(e) => { setCalendarAnchorEl(e.currentTarget); setCalendarOpen(o => !o); }}
          aria-label="Calendar"
        >
          <CalendarMonthIcon fontSize="small" />
        </IconButton>
      </Box>

      <JournalCalendar
        open={calendarOpen}
        onClose={() => { setCalendarOpen(false); setCalendarAnchorEl(null); }}
        onDateSelect={handleDateSelect}
        anchorEl={calendarAnchorEl}
        journalDates={allJournalDates}
      />

      {todayExists ? (
        <OutlinerEditor pagePath={todayPagePath} minHeight="0" />
      ) : (
        <CircularProgress size={20} sx={{ display: 'block', mx: 'auto', my: 4 }} />
      )}

      {pastDates.slice(0, visibleCount).map((date) => {
        const path = journalPath(date);
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
  );
}
