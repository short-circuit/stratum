import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useStore } from '../stores/appStore';
import * as api from '../lib/commands';

function formatDate(d: Date): string {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, '0');
  const day = String(d.getDate()).padStart(2, '0');
  return `${y}-${m}-${day}`;
}

function journalPath(date: string): string {
  return `journals/${date}.md`;
}

export default function JournalPanel() {
  const navigate = useNavigate();
  const { vault } = useStore();
  const today = new Date();
  const [selectedDate, setSelectedDate] = useState(formatDate(today));
  const [viewMonth, setViewMonth] = useState(today.getMonth());
  const [viewYear, setViewYear] = useState(today.getFullYear());
  const [creating, setCreating] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!vault) return;
    const path = journalPath(formatDate(today));
    api.createPage(path, formatDate(today))
      .then(() => console.log('Created journal:', path))
      .catch(err => {
        if (!String(err).includes('already exists')) {
          setError(`Failed to create journal: ${err}`);
        }
      });
  }, [vault]);

  const goToJournal = async (date: string) => {
    setSelectedDate(date);
    setError(null);
    if (!vault) {
      setError('Vault not loaded yet. Please wait...');
      return;
    }

    const path = journalPath(date);
    setCreating(true);
    try {
      await api.createPage(path, date);
    } catch (err) {
      if (!String(err).includes('already exists')) {
        setError(`Failed to create page: ${err}`);
        setCreating(false);
        return;
      }
    }
    setCreating(false);
    navigate(`/page/${encodeURIComponent(path)}`);
  };

  const daysInMonth = new Date(viewYear, viewMonth + 1, 0).getDate();
  const firstDay = new Date(viewYear, viewMonth, 1).getDay();

  const prevMonth = () => {
    if (viewMonth === 0) {
      setViewMonth(11);
      setViewYear(viewYear - 1);
    } else {
      setViewMonth(viewMonth - 1);
    }
  };

  const nextMonth = () => {
    if (viewMonth === 11) {
      setViewMonth(0);
      setViewYear(viewYear + 1);
    } else {
      setViewMonth(viewMonth + 1);
    }
  };

  const months = [
    'January', 'February', 'March', 'April', 'May', 'June',
    'July', 'August', 'September', 'October', 'November', 'December',
  ];

  return (
    <div className="p-4">
      {/* Error display */}
      {error && (
        <div className="mb-3 p-2 bg-red-50 dark:bg-red-900/30 text-red-600 dark:text-red-300 text-xs rounded">
          {error}
          <button onClick={() => setError(null)} className="ml-2 underline">Dismiss</button>
        </div>
      )}

      {/* Month navigation */}
      <div className="flex items-center justify-between mb-4">
        <button
          onClick={prevMonth}
          className="px-2 py-1 text-sm rounded hover:bg-gray-200 dark:hover:bg-gray-700"
        >
          ←
        </button>
        <h3 className="text-sm font-semibold">
          {months[viewMonth]} {viewYear}
        </h3>
        <button
          onClick={nextMonth}
          className="px-2 py-1 text-sm rounded hover:bg-gray-200 dark:hover:bg-gray-700"
        >
          →
        </button>
      </div>

      {/* Day headers */}
      <div className="grid grid-cols-7 gap-0.5 text-center mb-1">
        {['Su', 'Mo', 'Tu', 'We', 'Th', 'Fr', 'Sa'].map(d => (
          <div key={d} className="text-xs text-gray-400 py-1">{d}</div>
        ))}
      </div>

      {/* Calendar grid */}
      <div className="grid grid-cols-7 gap-0.5">
        {Array.from({ length: firstDay }).map((_, i) => (
          <div key={`empty-${i}`} />
        ))}
        {Array.from({ length: daysInMonth }).map((_, i) => {
          const day = i + 1;
          const date = `${viewYear}-${String(viewMonth + 1).padStart(2, '0')}-${String(day).padStart(2, '0')}`;
          const isToday = date === formatDate(today);
          const isSelected = date === selectedDate;

          return (
            <button
              key={day}
              onClick={() => goToJournal(date)}
              className={`
                text-xs p-1.5 rounded text-center transition-colors
                ${isToday ? 'bg-[var(--accent-100)] dark:bg-[var(--accent-900)]/30 text-[var(--accent-700)] dark:text-[var(--accent-300)] font-bold' : ''}
                ${isSelected && !isToday ? 'bg-gray-200 dark:bg-gray-700' : ''}
                hover:bg-gray-100 dark:hover:bg-gray-800
              `}
            >
              {day}
            </button>
          );
        })}
      </div>

      {/* Today button */}
      <button
        onClick={() => goToJournal(formatDate(today))}
        disabled={creating}
        className="mt-4 w-full px-3 py-2 bg-[var(--accent-500)] text-white rounded text-sm hover:bg-[var(--accent-600)] disabled:opacity-50"
      >
        {creating ? 'Creating...' : `Today: ${formatDate(today)}`}
      </button>
    </div>
  );
}
