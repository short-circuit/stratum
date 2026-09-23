import { useState, useEffect, useMemo, useRef, useCallback } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { useShallow } from 'zustand/react/shallow';
import { useStore } from '../../stores/appStore';
import * as api from '../../lib/commands';

/** Format a Date as YYYY-MM-DD for journal paths. */
export function formatDate(d: Date): string {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, '0');
  const day = String(d.getDate()).padStart(2, '0');
  return `${y}-${m}-${day}`;
}

/** Derive the page path for a journal date string. */
export function journalPath(date: string): string {
  return `journals/${date}.md`;
}

/** Format a YYYY-MM-DD string into a human-readable display string. */
export function formatDisplayDate(dateStr: string): string {
  const d = new Date(dateStr + 'T12:00:00');
  return d.toLocaleDateString('en-US', {
    weekday: 'long',
    month: 'long',
    day: 'numeric',
    year: 'numeric',
  });
}

/** Shift a YYYY-MM-DD string by a signed number of days (for Prev/Next day arrows). */
export function addDays(dateStr: string, delta: number): string {
  const d = new Date(dateStr + 'T12:00:00');
  d.setDate(d.getDate() + delta);
  return formatDate(d);
}

/** True if a string is a valid journal date key (YYYY-MM-DD). */
export function isJournalDateKey(value: string | null): value is string {
  return !!value && /^\d{4}-\d{2}-\d{2}$/.test(value);
}

/** Shared hook for journal panel state and logic used by both desktop and mobile variants. */
export function useJournalPanel() {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const { pages, loadPages } = useStore(useShallow(
    s => ({ pages: s.pages, loadPages: s.loadPages }),
  ));

  const today = useMemo(() => formatDate(new Date()), []);
  const todayPagePath = journalPath(today);
  const targetDate = searchParams.get('date');

  const [visibleCount, setVisibleCount] = useState(25);
  const [visibleSections, setVisibleSections] = useState<Set<string>>(new Set());
  const [calendarOpen, setCalendarOpen] = useState(false);
  const [calendarAnchorEl, setCalendarAnchorEl] = useState<HTMLElement | null>(null);
  const [journalLoading, setJournalLoading] = useState(false);
  const [journalError, setJournalError] = useState<string | null>(null);

  const observerRef = useRef<IntersectionObserver | null>(null);
  const sentinelRef = useRef<HTMLDivElement | null>(null);
  // The panel's actual scrolling element. The app shell is overflow:hidden on
  // both platforms, so this is the journal panel's own scroll root — NOT
  // window. The desktop variant attaches it to the inner scroll Box, and the
  // mobile variant to its root Box. Used so "jump to today" scrolls the
  // correct container (window.scrollTo is a no-op when window doesn't scroll).
  const scrollRootRef = useRef<HTMLElement | null>(null);
  const processedTargetRef = useRef<string | null>(null);
  const targetDateRef = useRef(targetDate);
  useEffect(() => {
    targetDateRef.current = targetDate;
  }, [targetDate]);

  const allJournalDates = useMemo(() => {
    return new Set(
      pages
        .filter((p) => p.path.startsWith('journals/') && p.path.endsWith('.md'))
        .map((p) => p.path.slice(9, -3)),
    );
  }, [pages]);

  const pastDates = useMemo(() => {
    return pages
      .filter((p) => p.path.startsWith('journals/') && p.path.endsWith('.md'))
      .map((p) => p.path.slice(9, -3))
      .filter((d) => /^\d{4}-\d{2}-\d{2}$/.test(d) && d !== today)
      .sort()
      .reverse();
  }, [pages, today]);

  const todayExists = useMemo(
    () => pages.length > 0 && allJournalDates.has(today),
    [pages, allJournalDates, today],
  );

  // Atomic ensure — replaces createPage+loadPages which caused infinite spinner (#12).
  // Runs even when the page list is empty (fresh vault): the backend command is
  // idempotent and creates today's journal, otherwise the panel would show a
  // perpetual spinner on a brand-new vault that has no pages yet.
  const ensureJournal = useCallback(() => {
    if (todayExists) {
      setJournalLoading(false);
      setJournalError(null);
      return;
    }
    setJournalLoading(true);
    setJournalError(null);
    api
      .ensureTodayJournal()
      .then(() => {
        setJournalLoading(false);
        loadPages();
      })
      .catch((err) => {
        setJournalError(String(err));
        setJournalLoading(false);
      });
  }, [pages, todayExists, loadPages]);

  useEffect(() => {
    ensureJournal();
  }, [ensureJournal]);

  const retryJournal = useCallback(() => {
    setJournalError(null);
    setJournalLoading(true);
    api
      .ensureTodayJournal()
      .then(() => {
        setJournalLoading(false);
        loadPages();
      })
      .catch((err) => {
        setJournalError(String(err));
        setJournalLoading(false);
      });
  }, [loadPages]);

  const repairJournal = useCallback(() => {
    setJournalError(null);
    setJournalLoading(true);
    api
      .repairDbFromDisk()
      .then(() => loadPages())
      .then(() => api.ensureTodayJournal())
      .then(() => {
        setJournalLoading(false);
        loadPages();
      })
      .catch((err) => {
        setJournalError(String(err));
        setJournalLoading(false);
      });
  }, [loadPages]);

  const getObserver = useCallback(() => {
    if (!observerRef.current) {
      observerRef.current = new IntersectionObserver(
        (entries) => {
          for (const entry of entries) {
            if (entry.isIntersecting) {
              const date = (entry.target as HTMLElement).dataset.date;
              if (date) {
                setVisibleSections((prev) => {
                  if (prev.has(date)) return prev;
                  const next = new Set(prev);
                  next.add(date);
                  return next;
                });
                observerRef.current?.unobserve(entry.target);
              }
            }
          }
        },
        { rootMargin: '200px' },
      );
    }
    return observerRef.current;
  }, []);

  useEffect(() => {
    return () => observerRef.current?.disconnect();
  }, []);

  useEffect(() => {
    const el = sentinelRef.current;
    if (!el) return;
    const io = new IntersectionObserver(
      (entries) => {
        if (entries[0]?.isIntersecting) {
          setVisibleCount((prev) => prev + 25);
        }
      },
      { rootMargin: '400px' },
    );
    io.observe(el);
    return () => io.disconnect();
  }, [pastDates.length, visibleCount]);

  /** Callback ref for each date section — attaches intersection observer and handles scroll-to-target. */
  const sectionRef = useCallback(
    (date: string) =>
      (el: HTMLDivElement | null) => {
        if (el) {
          el.dataset.date = date;
          getObserver().observe(el);
          if (date === targetDateRef.current && date !== today) {
            requestAnimationFrame(() => {
              el.scrollIntoView({ behavior: 'smooth', block: 'center' });
            });
          }
        }
      },
    [getObserver, today],
  );

  // Scroll to target date when search param changes
  useEffect(() => {
    if (!targetDate) {
      processedTargetRef.current = null;
      return;
    }
    if (processedTargetRef.current === targetDate) return;

    if (targetDate === today) {
      scrollRootRef.current?.scrollTo({ top: 0, behavior: 'smooth' });
      processedTargetRef.current = targetDate;
      return;
    }

    const idx = pastDates.indexOf(targetDate);
    if (idx === -1) return;

    requestAnimationFrame(() => {
      if (idx >= visibleCount) {
        setVisibleCount(idx + 10);
      }
      setVisibleSections((prev) => {
        if (prev.has(targetDate)) return prev;
        const next = new Set(prev);
        next.add(targetDate);
        return next;
      });
    });

    requestAnimationFrame(() => {
      const el = document.querySelector(`[data-date="${targetDate}"]`);
      if (el) {
        // Prefer scrolling the panel container (works on both variants) and
        // fall back to element.scrollIntoView for any residual case.
        const root = scrollRootRef.current;
        if (root) {
          const top = (el as HTMLElement).offsetTop - root.offsetTop;
          root.scrollTo({ top, behavior: 'smooth' });
        } else {
          el.scrollIntoView({ behavior: 'smooth', block: 'center' });
        }
        processedTargetRef.current = targetDate;
      }
    });
  }, [targetDate, pastDates, today, visibleCount]);

  /** Create a journal page if needed, then navigate to it. */
  const handleDateSelect = useCallback(
    async (date: string) => {
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
    },
    [allJournalDates, loadPages, navigate],
  );

  /** Navigate to the previous (-1) or next (+1) day relative to the current target (or today).
   *  Reuses handleDateSelect so the target journal is created on demand if it
   *  doesn't exist yet — identical to selecting the date in the calendar. */
  const createNewDay = useCallback(
    (delta: -1 | 1) => {
      const base = isJournalDateKey(targetDate) ? targetDate : today;
      handleDateSelect(addDays(base, delta));
    },
    [handleDateSelect, targetDate, today],
  );

  return {
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
    calendarAnchorEl,
    setCalendarAnchorEl,
    handleDateSelect,
    createNewDay,
  };
}
