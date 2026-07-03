import { useState, useEffect } from 'react';
import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import CardContent from '@mui/material/CardContent';
import Button from '@mui/material/Button';
import Typography from '@mui/material/Typography';
import Stack from '@mui/material/Stack';
import * as api from '../lib/commands';

interface CardData {
  id: string;
  front: string;
  back: string;
  page_path: string;
  ease_factor: number;
  interval_days: number;
  repetitions: number;
  next_review: string;
}

const RATINGS = [
  { label: 'Blackout', q: 0, color: 'error' as const },
  { label: 'Hard', q: 2, color: 'warning' as const },
  { label: 'Good', q: 3, color: 'success' as const },
  { label: 'Easy', q: 5, color: 'primary' as const },
];

export default function FlashcardsPanel() {
  const [cards, setCards] = useState<CardData[]>([]);
  const [current, setCurrent] = useState(0);
  const [showBack, setShowBack] = useState(false);
  const [message, setMessage] = useState('');

  useEffect(() => {
    api.generateFlashcards().then(setCards).catch(console.error);
  }, []);

  const review = async (quality: number) => {
    if (current >= cards.length) return;
    try {
      await api.reviewCard(cards[current].id, quality);
      setMessage(quality >= 3 ? 'Correct!' : 'Review again soon');
      setTimeout(() => {
        setShowBack(false);
        setMessage('');
        setCurrent(c => c + 1);
      }, 800);
    } catch (e) {
      setMessage(`Error: ${e}`);
    }
  };

  if (cards.length === 0) {
    return (
      <Box sx={{ p: 3, maxWidth: 600, mx: 'auto' }}>
        <Typography variant="h5" sx={{ fontWeight: 600, mb: 2 }}>Flashcards</Typography>
        <Typography variant="body2" color="text.secondary">
          No flashcards found. Create blocks with <Box component="code" sx={{ bgcolor: 'action.hover', px: 0.5, borderRadius: 0.5 }}>question::</Box> and
          {' '}<Box component="code" sx={{ bgcolor: 'action.hover', px: 0.5, borderRadius: 0.5 }}>answer::</Box> properties to generate cards.
        </Typography>
      </Box>
    );
  }

  if (current >= cards.length) {
    return (
      <Box sx={{ p: 3, maxWidth: 600, mx: 'auto', textAlign: 'center' }}>
        <Typography variant="h5" sx={{ fontWeight: 600, mb: 1 }}>Session Complete!</Typography>
        <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
          Reviewed {cards.length} card{cards.length !== 1 ? 's' : ''}.
        </Typography>
        <Button variant="contained" onClick={() => { setCurrent(0); setShowBack(false); }}>
          Start Again
        </Button>
      </Box>
    );
  }

  const card = cards[current];

  return (
    <Box sx={{ p: 3, maxWidth: 600, mx: 'auto' }}>
      <Typography variant="h5" sx={{ fontWeight: 600, mb: 0.5 }}>Flashcards</Typography>
      <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 2 }}>
        Card {current + 1} of {cards.length}
        {card.next_review && ` · Next: ${card.next_review}`}
      </Typography>

      {/* Card */}
      <Card
        sx={{ minHeight: 200, display: 'flex', alignItems: 'center', justifyContent: 'center', cursor: 'pointer', mb: 2, border: 2, borderColor: 'divider' }}
        onClick={() => setShowBack(!showBack)}
      >
        <CardContent>
          <Box sx={{ textAlign: 'center', maxWidth: 400 }}>
            <Typography variant="caption" color="text.disabled" sx={{ display: 'block', mb: 1 }}>
              {showBack ? 'Answer' : 'Question'}
            </Typography>
            <Typography variant="h6" sx={{ fontWeight: 500 }}>
              {showBack ? card.back : card.front}
            </Typography>
            <Typography variant="caption" color="text.disabled" sx={{ display: 'block', mt: 2 }}>
              {showBack ? 'Click to see question' : 'Click to reveal answer'}
            </Typography>
          </Box>
        </CardContent>
      </Card>

      {/* Source */}
      <Typography variant="caption" color="text.disabled" sx={{ display: 'block', textAlign: 'center', mb: 2 }}>
        Source: {card.page_path} · Ease: {card.ease_factor.toFixed(1)} · Interval: {card.interval_days}d
      </Typography>

      {message && (
        <Typography variant="body2" color="primary" sx={{ textAlign: 'center', mb: 1.5, fontWeight: 500 }}>
          {message}
        </Typography>
      )}

      {/* SM-2 rating buttons */}
      {showBack && !message && (
        <Stack direction="row" spacing={1} sx={{ justifyContent: 'center' }}>
          {RATINGS.map(({ label, q, color }) => (
            <Button key={q} variant="contained" color={color} size="small" onClick={() => review(q)}>
              {label}
            </Button>
          ))}
        </Stack>
      )}
    </Box>
  );
}
