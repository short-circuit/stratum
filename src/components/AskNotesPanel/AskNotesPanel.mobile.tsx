import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import Button from '@mui/material/Button';
import { useAskNotes, AskNotesInput, AskNotesAnswer } from './AskNotesPanel.shared';

export default function AskNotesPanelMobile() {
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
          Ask your notes
        </Typography>
        {result && (
          <Button size="small" onClick={reset}>
            Clear
          </Button>
        )}
      </Box>
      <Typography variant="body2" color="text.secondary" sx={{ mt: 0.5, mb: 1.5 }}>
        Get answers grounded in your vault, with cited sources.
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
