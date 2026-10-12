import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import CardContent from '@mui/material/CardContent';
import Button from '@mui/material/Button';
import Typography from '@mui/material/Typography';
import Stack from '@mui/material/Stack';
import { useTranslation } from 'react-i18next';
import { RATINGS, useFlashcardReview } from './FlashcardsPanel.shared';

export default function FlashcardsPanelMobile() {
  const { t } = useTranslation();
  const { cards, current, showBack, setShowBack, message, review, reset, card } = useFlashcardReview();

  if (cards.length === 0) {
    return (
      <Box sx={{ p: 2 }}>
        <Typography variant="h5" sx={{ fontWeight: 600, mb: 2 }}>{t('flashcards.flashcards')}</Typography>
        <Typography variant="body2" color="text.secondary">
          {t('flashcards.no_flashcards_found_create_blocks_with')} <Box component="code" sx={{ bgcolor: 'action.hover', px: 0.5, borderRadius: 0.5 }}>{t('flashcards.question')}</Box> and
          {' '}<Box component="code" sx={{ bgcolor: 'action.hover', px: 0.5, borderRadius: 0.5 }}>{t('flashcards.answer')}</Box> {t('flashcards.properties_to_generate_cards')}
        </Typography>
      </Box>
    );
  }

  if (current >= cards.length) {
    return (
      <Box sx={{ p: 2, textAlign: 'center' }}>
        <Typography variant="h5" sx={{ fontWeight: 600, mb: 1 }}>{t('flashcards.session_complete')}</Typography>
        <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
          {t('flashcards.reviewed_cards', { count: cards.length })}
        </Typography>
        <Button variant="contained" size="large" onClick={reset}>
          {t('flashcards.start_again')}
        </Button>
      </Box>
    );
  }

  return (
    <Box sx={{ p: 1.5 }}>
      <Typography variant="h5" sx={{ fontWeight: 600, mb: 0.5 }}>{t('flashcards.flashcards')}</Typography>
      <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 1.5 }}>
        {t('flashcards.card_of_count', { current: current + 1, total: cards.length })}
        {card!.next_review && ` ${t('flashcards._next')} ${card!.next_review}`}
      </Typography>
      <Card
        sx={{
          minHeight: 240,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          cursor: 'pointer',
          mb: 1.5,
          border: 2,
          borderColor: 'divider',
          borderRadius: 3,
          width: '100%',
        }}
        onClick={() => setShowBack(!showBack)}
      >
        <CardContent sx={{ width: '100%', px: 2 }}>
          <Box sx={{ textAlign: 'center', width: '100%' }}>
            <Typography variant="caption" color="text.disabled" sx={{ display: 'block', mb: 1 }}>
              {showBack ? t('flashcards.answer_label') : t('flashcards.question_label')}
            </Typography>
            <Typography variant="h5" sx={{ fontWeight: 500, fontSize: '1.35rem', lineHeight: 1.5, wordBreak: 'break-word' }}>
              {showBack ? card!.back : card!.front}
            </Typography>
            <Typography variant="caption" color="text.disabled" sx={{ display: 'block', mt: 2 }}>
              {showBack ? t('flashcards.tap_for_question') : t('flashcards.tap_to_reveal_answer')}
            </Typography>
          </Box>
        </CardContent>
      </Card>

      <Typography variant="caption" color="text.disabled" sx={{ display: 'block', textAlign: 'center', mb: 1.5, fontSize: '0.7rem' }}>
        {t('flashcards.source')} {card!.page_path}
      </Typography>

      {message && (
        <Typography variant="body1" color="primary" sx={{ textAlign: 'center', mb: 1.5, fontWeight: 500 }}>
          {message}
        </Typography>
      )}

      {showBack && !message && (
        <Stack spacing={1} sx={{ px: 0.5 }}>
          {RATINGS.map(({ label, q, color }) => (
            <Button
              key={q}
              variant="contained"
              color={color}
              size="large"
              fullWidth
              onClick={() => review(q)}
              sx={{ py: 1.2, fontSize: '1rem' }}
            >
              {label}
            </Button>
          ))}
        </Stack>
      )}
    </Box>
  );
}
