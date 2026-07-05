import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import CardContent from '@mui/material/CardContent';
import Button from '@mui/material/Button';
import Typography from '@mui/material/Typography';
import Stack from '@mui/material/Stack';
import { RATINGS, useFlashcardReview } from './FlashcardsPanel.shared';

export default function FlashcardsPanelDesktop() {
  const { cards, current, showBack, setShowBack, message, review, reset, card } = useFlashcardReview();

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
        <Button variant="contained" onClick={reset}>
          Start Again
        </Button>
      </Box>
    );
  }

  return (
    <Box sx={{ p: 3, maxWidth: 600, mx: 'auto' }}>
      <Typography variant="h5" sx={{ fontWeight: 600, mb: 0.5 }}>Flashcards</Typography>
      <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 2 }}>
        Card {current + 1} of {cards.length}
        {card!.next_review && ` · Next: ${card!.next_review}`}
      </Typography>

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
              {showBack ? card!.back : card!.front}
            </Typography>
            <Typography variant="caption" color="text.disabled" sx={{ display: 'block', mt: 2 }}>
              {showBack ? 'Click to see question' : 'Click to reveal answer'}
            </Typography>
          </Box>
        </CardContent>
      </Card>

      <Typography variant="caption" color="text.disabled" sx={{ display: 'block', textAlign: 'center', mb: 2 }}>
        Source: {card!.page_path} · Ease: {card!.ease_factor.toFixed(1)} · Interval: {card!.interval_days}d
      </Typography>

      {message && (
        <Typography variant="body2" color="primary" sx={{ textAlign: 'center', mb: 1.5, fontWeight: 500 }}>
          {message}
        </Typography>
      )}

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
