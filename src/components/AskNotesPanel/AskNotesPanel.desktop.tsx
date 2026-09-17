import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import Button from '@mui/material/Button';
import { useAskNotes, AskNotesInput, AskNotesAnswer } from './AskNotesPanel.shared';

export default function AskNotesPanelDesktop() {
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
    <Box sx={{ maxWidth: 760, mx: 'auto', p: { xs: 2, md: 3 } }}>
      <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <Typography variant="h5" component="h1">
          Ask your notes
        </Typography>
        {result && (
          <Button size="small" onClick={reset}>
            Clear
          </Button>
        )}
      </Box>
      <Typography variant="body2" color="text.secondary" sx={{ mt: 0.5, mb: 2 }}>
        Ask a question and get an answer grounded in your vault, with cited sources.
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
