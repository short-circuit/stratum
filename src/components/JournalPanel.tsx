import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import Box from '@mui/material/Box';
import IconButton from '@mui/material/IconButton';
import Button from '@mui/material/Button';
import Typography from '@mui/material/Typography';
import Alert from '@mui/material/Alert';
import ChevronLeftIcon from '@mui/icons-material/ChevronLeft';
import ChevronRightIcon from '@mui/icons-material/ChevronRight';
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

const MONTHS = [
  'January', 'February', 'March', 'April', 'May', 'June',
  'July', 'August', 'September', 'October', 'November', 'December',
];

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
    const today = formatDate(new Date());
    const path = journalPath(today);
    api.createPage(path, today)
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

  return (
    <Box sx={{ p: 3, maxWidth: 400, mx: 'auto' }}>
      {error && (
        <Alert severity="error" sx={{ mb: 2 }} onClose={() => setError(null)}>
          {error}
        </Alert>
      )}

      {/* Month navigation */}
      <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', mb: 2 }}>
        <IconButton size="small" onClick={prevMonth}>
          <ChevronLeftIcon />
        </IconButton>
        <Typography variant="subtitle2" sx={{ fontWeight: 600 }}>
          {MONTHS[viewMonth]} {viewYear}
        </Typography>
        <IconButton size="small" onClick={nextMonth}>
          <ChevronRightIcon />
        </IconButton>
      </Box>

      {/* Day headers */}
      <Box sx={{ display: 'grid', gridTemplateColumns: 'repeat(7, 1fr)', gap: 0.25, textAlign: 'center', mb: 0.5 }}>
        {['Su', 'Mo', 'Tu', 'We', 'Th', 'Fr', 'Sa'].map(d => (
          <Typography key={d} variant="caption" color="text.disabled" sx={{ py: 0.5 }}>{d}</Typography>
        ))}
      </Box>

      {/* Calendar grid */}
      <Box sx={{ display: 'grid', gridTemplateColumns: 'repeat(7, 1fr)', gap: 0.25 }}>
        {Array.from({ length: firstDay }).map((_, i) => (
          <Box key={`empty-${i}`} />
        ))}
        {Array.from({ length: daysInMonth }).map((_, i) => {
          const day = i + 1;
          const date = `${viewYear}-${String(viewMonth + 1).padStart(2, '0')}-${String(day).padStart(2, '0')}`;
          const isToday = date === formatDate(today);
          const isSelected = date === selectedDate;

          return (
            <Button
              key={day}
              size="small"
              onClick={() => goToJournal(date)}
              sx={{
                minWidth: 0, p: 0.75, fontSize: '0.75rem', borderRadius: 1,
                fontWeight: isToday ? 700 : 400,
                bgcolor: isToday ? 'primary.light' : isSelected ? 'action.selected' : 'transparent',
                color: isToday ? 'primary.contrastText' : undefined,
                '&:hover': { bgcolor: isToday ? 'primary.light' : 'action.hover' },
              }}
            >
              {day}
            </Button>
          );
        })}
      </Box>

      {/* Today button */}
      <Button
        variant="contained"
        fullWidth
        onClick={() => goToJournal(formatDate(today))}
        disabled={creating}
        sx={{ mt: 2 }}
      >
        {creating ? 'Creating...' : `Today: ${formatDate(today)}`}
      </Button>
    </Box>
  );
}
