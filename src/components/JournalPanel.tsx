import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
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
  const today = new Date();
  const [selectedDate, setSelectedDate] = useState(formatDate(today));
  const [viewMonth, setViewMonth] = useState(today.getMonth());
  const [viewYear, setViewYear] = useState(today.getFullYear());

  useEffect(() => {
    // Ensure today's journal exists
    const path = journalPath(formatDate(today));
    api.createPage(path, formatDate(today)).catch(() => {
      // Already exists — that's fine
    });
  }, []);

  const goToJournal = (date: string) => {
    setSelectedDate(date);
    const path = journalPath(date);
    // Ensure page exists, then navigate
    api.createPage(path, date).catch(() => {});
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
                ${isToday ? 'bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-300 font-bold' : ''}
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
        className="mt-4 w-full px-3 py-2 bg-blue-500 text-white rounded text-sm hover:bg-blue-600"
      >
        Today: {formatDate(today)}
      </button>
    </div>
  );
}
