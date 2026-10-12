import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import Button from '@mui/material/Button';
import { useAskNotes, AskNotesInput, AskNotesAnswer } from './AskNotesPanel.shared';
import { useTranslation } from 'react-i18next';

export default function AskNotesPanelMobile() {
  const { t } = useTranslation();
  const {
    question,
    setQuestion,
    result,
    error,
    loading,
    speaking,
    speakError,
    runQuery,
    speakAnswer,
    reset,
  } = useAskNotes();

  return (
    <Box sx={{ p: 1.5, width: '100%' }}>
      <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <Typography variant="h6" component="h1">
          {t('ask_notes.ask_your_notes')}
        </Typography>
        {result && (
          <Button size="small" onClick={reset}>
            {t('ask_notes.clear')}
          </Button>
        )}
      </Box>
      <Typography variant="body2" color="text.secondary" sx={{ mt: 0.5, mb: 1.5 }}>
        {t('ask_notes.get_answers_grounded_in_your_vault_with_cited_sources')}
      </Typography>

      <AskNotesInput
        question={question}
        onQuestionChange={setQuestion}
        onAsk={runQuery}
        disabled={loading}
      />

      <AskNotesAnswer
        error={error}
        loading={loading}
        speaking={speaking}
        speakError={speakError}
        result={result}
        onSpeak={speakAnswer}
      />
    </Box>
  );
}
