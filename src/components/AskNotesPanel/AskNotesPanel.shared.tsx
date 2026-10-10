import { useCallback, useState } from 'react';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import TextField from '@mui/material/TextField';
import Button from '@mui/material/Button';
import IconButton from '@mui/material/IconButton';
import Tooltip from '@mui/material/Tooltip';
import CircularProgress from '@mui/material/CircularProgress';
import Alert from '@mui/material/Alert';
import Paper from '@mui/material/Paper';
import List from '@mui/material/List';
import ListItem from '@mui/material/ListItem';
import ListItemText from '@mui/material/ListItemText';
import VolumeUpIcon from '@mui/icons-material/VolumeUp';
import VolumeOffIcon from '@mui/icons-material/VolumeOff';
import * as api from '../../lib/commands';
import { playAudio } from '../../lib/audio';
import { useTranslation } from 'react-i18next';
import type { RagQueryResultDto } from '../../lib/types';

/* eslint-disable react-refresh/only-export-components -- shared hooks + components, per repo pattern */

export function useAskNotes() {
  const [question, setQuestion] = useState('');
  const [result, setResult] = useState<RagQueryResultDto | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [speaking, setSpeaking] = useState(false);
  const [speakError, setSpeakError] = useState<string | null>(null);

  const runQuery = useCallback(async () => {
    if (!question.trim()) return;
    setLoading(true);
    setError(null);
    setResult(null);
    try {
      const res = await api.aiRagQuery(question.trim());
      setResult(res);
    } catch (e) {
      setError(String(e));
      setResult(null);
    } finally {
      setLoading(false);
    }
  }, [question]);

  // Read the generated answer aloud via the configured TTS endpoint.
  const speakAnswer = useCallback(async () => {
    if (!result || !result.answer.trim()) return;
    setSpeaking(true);
    setSpeakError(null);
    try {
      const audio = await api.ttsSpeak(result.answer);
      await playAudio(audio.audio_b64, audio.mime);
    } catch (e) {
      console.error('[TTS] read-aloud failed:', e);
      setSpeakError(String(e));
    } finally {
      setSpeaking(false);
    }
  }, [result]);

  const reset = useCallback(() => {
    setResult(null);
    setError(null);
    setSpeakError(null);
  }, []);

  return {
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
  };
}

export interface AskNotesBodyProps {
  error: string | null;
  loading: boolean;
  speaking: boolean;
  speakError: string | null;
  result: RagQueryResultDto | null;
}

/** Shared answer + source rendering (used by both desktop and mobile shells). */
export function AskNotesAnswer({
  error,
  loading,
  speaking,
  speakError,
  result,
  onSpeak,
}: AskNotesBodyProps & { onSpeak: () => void }) {
  const { t } = useTranslation();
  if (loading) {
    return (
      <Box sx={{ display: 'flex', alignItems: 'center', gap: 1.5, py: 3 }}>
        <CircularProgress size={18} />
        <Typography variant="body2" color="text.secondary">
          {t('ask_notes.searching_your_notes')}
        </Typography>
      </Box>
    );
  }

  if (error) {
    return (
      <Alert severity="error" sx={{ mt: 2 }}>
        {error}
      </Alert>
    );
  }

  if (speakError) {
    return (
      <Alert severity="warning" sx={{ mt: 2 }}>
        {t('aiformatting_toolbar.read_aloud_failed')} {speakError}
      </Alert>
    );
  }

  if (!result) return null;

  return (
    <Box sx={{ mt: 2 }}>
      <Paper
        variant="outlined"
        sx={{
          p: 2,
          borderRadius: 2,
          bgcolor: 'background.paper',
        }}
      >
        <Box sx={{ display: 'flex', alignItems: 'flex-start', gap: 1 }}>
          <Typography variant="body1" sx={{ flex: 1, whiteSpace: 'pre-wrap' }}>
            {result.answer}
          </Typography>
          {result.answer.trim() && (
            <Tooltip title={speaking ? t('ask_notes.playing') : t('editor.read_aloud')}>
              <IconButton size="small" onClick={onSpeak} disabled={speaking}>
                {speaking ? <VolumeOffIcon fontSize="small" /> : <VolumeUpIcon fontSize="small" />}
              </IconButton>
            </Tooltip>
          )}
        </Box>

        {result.had_sources || result.citations.length > 0 ? (
          <Box sx={{ mt: 2 }}>
            <Typography variant="overline" color="text.secondary">
              {t('ask_notes.sources')}
            </Typography>
            <List dense disablePadding>
              {result.citations.map((c, i) => (
                <ListItem key={`${c.path}-${i}`} disableGutters dense>
                  <ListItemText
                    primary={c.path}
                    secondary={
                      <>
                        <span>{c.snippet}</span>{' '}
                        <span style={{ opacity: 0.6 }}>({c.score.toFixed(2)})</span>
                      </>
                    }
                    slotProps={{ primary: { variant: 'body2', noWrap: true } }}
                  />
                </ListItem>
              ))}
            </List>
          </Box>
        ) : (
          <Typography variant="caption" color="text.secondary" sx={{ mt: 2, display: 'block' }}>
            {t('ask_notes.no_matching_notes_found_the_answer_is_based_on_general_knowl')}
          </Typography>
        )}
      </Paper>
    </Box>
  );
}

export function AskNotesInput({
  question,
  onQuestionChange,
  onAsk,
  disabled,
}: {
  question: string;
  onQuestionChange: (v: string) => void;
  onAsk: () => void;
  disabled: boolean;
}) {
  const { t } = useTranslation();
  return (
    <Box sx={{ display: 'flex', gap: 1, alignItems: 'flex-start' }}>
      <TextField
        fullWidth
        multiline
        minRows={1}
        maxRows={4}
        size="small"
        placeholder={t('ask_notes.ask_your_notes_e_g_what_did_i_decide_about_the_h')}
        value={question}
        onChange={(e) => onQuestionChange(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === 'Enter' && !e.shiftKey) {
            e.preventDefault();
            onAsk();
          }
        }}
        disabled={disabled}
      />
      <Button variant="contained" onClick={onAsk} disabled={disabled || !question.trim()}>
        {t('ask_notes.ask')}
      </Button>
    </Box>
  );
}
