import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import CardContent from '@mui/material/CardContent';
import Button from '@mui/material/Button';
import Typography from '@mui/material/Typography';
import Stack from '@mui/material/Stack';
import { useTranslation } from 'react-i18next';
import { RATINGS, useFlashcardReview } from './FlashcardsPanel.shared';

export default function FlashcardsPanelDesktop() {
  const { t } = useTranslation();
  const { cards, current, showBack, setShowBack, message, review, reset, card } = useFlashcardReview();

  if (cards.length === 0) {
    return (
      <Box sx={{ height: '100%', display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
        <Box sx={{ flex: 1, overflow: 'auto', p: 3, maxWidth: 600, mx: 'auto' }}>
        <Typography variant="h5" sx={{ fontWeight: 600, mb: 2 }}>{t('flashcards.flashcards')}</Typography>
        <Typography variant="body2" color="text.secondary">
          {t('flashcards.no_flashcards_found_create_blocks_with')} <Box component="code" sx={{ bgcolor: 'action.hover', px: 0.5, borderRadius: 0.5 }}>{t('flashcards.question')}</Box> and
          {' '}<Box component="code" sx={{ bgcolor: 'action.hover', px: 0.5, borderRadius: 0.5 }}>{t('flashcards.answer')}</Box> {t('flashcards.properties_to_generate_cards')}
        </Typography>
      </Box>
      </Box>
    );
  }

  if (current >= cards.length) {
    return (
      <Box sx={{ height: '100%', display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
        <Box sx={{ flex: 1, overflow: 'auto', p: 3, maxWidth: 600, mx: 'auto', textAlign: 'center' }}>
        <Typography variant="h5" sx={{ fontWeight: 600, mb: 1 }}>{t('flashcards.session_complete')}</Typography>
        <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
          {t('flashcards.reviewed_cards', { count: cards.length })}
        </Typography>
        <Button variant="contained" onClick={reset}>
          {t('flashcards.start_again')}
        </Button>
      </Box>
      </Box>
    );
  }

  return (
    <Box sx={{ height: '100%', display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
      <Box sx={{ flex: 1, overflow: 'auto', p: 3, maxWidth: 600, mx: 'auto' }}>
      <Typography variant="h5" sx={{ fontWeight: 600, mb: 0.5 }}>{t('flashcards.flashcards')}</Typography>
      <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 2 }}>
        {t('flashcards.card_of_count', { current: current + 1, total: cards.length })}
        {card!.next_review && ` ${t('flashcards._next')} ${card!.next_review}`}
      </Typography>

      <Card
        sx={{ minHeight: 200, display: 'flex', alignItems: 'center', justifyContent: 'center', cursor: 'pointer', mb: 2, border: 2, borderColor: 'divider' }}
        onClick={() => setShowBack(!showBack)}
      >
        <CardContent>
          <Box sx={{ textAlign: 'center', maxWidth: 400 }}>
            <Typography variant="caption" color="text.disabled" sx={{ display: 'block', mb: 1 }}>
              {showBack ? t('flashcards.answer_label') : t('flashcards.question_label')}
            </Typography>
            <Typography variant="h6" sx={{ fontWeight: 500 }}>
              {showBack ? card!.back : card!.front}
            </Typography>
            <Typography variant="caption" color="text.disabled" sx={{ display: 'block', mt: 2 }}>
              {showBack ? t('flashcards.click_to_see_question') : t('flashcards.click_to_reveal_answer')}
            </Typography>
          </Box>
        </CardContent>
      </Card>

      <Typography variant="caption" color="text.disabled" sx={{ display: 'block', textAlign: 'center', mb: 2 }}>
        {t('flashcards.source')} {card!.page_path} {t('flashcards._ease')} {card!.ease_factor.toFixed(1)} {t('flashcards._interval')} {card!.interval_days}d
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
    </Box>
  );
}
