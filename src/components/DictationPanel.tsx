import { useCallback, useEffect, useRef, useState } from 'react';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Chip from '@mui/material/Chip';
import CircularProgress from '@mui/material/CircularProgress';
import Divider from '@mui/material/Divider';
import FormControlLabel from '@mui/material/FormControlLabel';
import IconButton from '@mui/material/IconButton';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import Checkbox from '@mui/material/Checkbox';
import Alert from '@mui/material/Alert';
import MicIcon from '@mui/icons-material/Mic';
import StopIcon from '@mui/icons-material/Stop';
import GraphicEqIcon from '@mui/icons-material/GraphicEq';
import CloseIcon from '@mui/icons-material/Close';
import VolumeUpIcon from '@mui/icons-material/VolumeUp';
import { listen } from '@tauri-apps/api/event';
import * as api from '../lib/commands';
import type { DictationResultDto } from '../lib/types';

type Stage = 'idle' | 'recording' | 'ready' | 'transcribing' | 'done' | 'error';

const STAGE_LABELS: Record<string, string> = {
  transcribing: 'Transcribing…',
  diarizing: 'Separating speakers…',
  identifying: 'Identifying voices…',
  summarizing: 'Summarizing…',
  linking: 'Linking notes & tags…',
};

function speakerLabel(id: string): string {
  const m = id.split('_').pop();
  const n = m ? parseInt(m, 10) : NaN;
  return `Speaker ${Number.isFinite(n) ? n + 1 : 0}`;
}

function fmt(secs: number): string {
  const s = Math.max(0, Math.round(secs));
  const m = Math.floor(s / 60);
  return `${m}:${String(s % 60).padStart(2, '0')}`;
}

export default function DictationPanel({
  pagePath,
  onInserted,
}: {
  pagePath: string;
  onInserted?: (ids: string[]) => void;
}) {
  const [stage, setStage] = useState<Stage>('idle');
  const [elapsed, setElapsed] = useState(0);
  const [recordingPath, setRecordingPath] = useState<string | null>(null);
  const [result, setResult] = useState<DictationResultDto | null>(null);
  const [stageName, setStageName] = useState('');
  const [diarize, setDiarize] = useState(true);
  const [summarize, setSummarize] = useState(true);
  const [identify, setIdentify] = useState(true);
  const [names, setNames] = useState<Record<string, string>>({});
  const [nameDraft, setNameDraft] = useState<Record<string, string>>({});
  const [error, setError] = useState<string | null>(null);
  const timerRef = useRef<number | null>(null);

  useEffect(() => {
    const un = listen<{ stage: string }>('dictation:stage', e => {
      setStageName(e.payload.stage);
    });
    return () => {
      un.then(f => f());
    };
  }, []);

  useEffect(() => () => {
    if (timerRef.current !== null) clearInterval(timerRef.current);
  }, []);

  const start = useCallback(async () => {
    setError(null);
    try {
      const dto = await api.dictationStart(pagePath);
      setRecordingPath(dto.recording_path);
      setStage('recording');
      setElapsed(0);
      setResult(null);
      timerRef.current = window.setInterval(() => setElapsed(e => e + 1), 1000);
    } catch (e) {
      setError(String(e));
    }
  }, [pagePath]);

  const stop = useCallback(async () => {
    if (timerRef.current !== null) clearInterval(timerRef.current);
    try {
      await api.dictationStop();
      setStage('ready');
    } catch (e) {
      setError(String(e));
    }
  }, []);

  const cancel = useCallback(async () => {
    if (timerRef.current !== null) clearInterval(timerRef.current);
    try {
      await api.dictationCancel();
    } catch {
      // nothing to cancel
    }
    setStage('idle');
    setRecordingPath(null);
    setError(null);
  }, []);

  const transcribe = useCallback(async () => {
    if (!recordingPath) return;
    setStage('transcribing');
    setError(null);
    try {
      const res = await api.dictationTranscribe(recordingPath, pagePath, {
        diarize,
        summarize,
        identify,
      });
      setResult(res);
      setNames(res.speaker_names);
      setStage('done');
      onInserted?.(res.inserted_block_ids);
    } catch (e) {
      setStage('error');
      setError(String(e));
    }
  }, [recordingPath, pagePath, diarize, summarize, identify, onInserted]);

  const assign = useCallback(
    async (speakerId: string, enroll: boolean) => {
      if (!recordingPath) return;
      const name = (nameDraft[speakerId] ?? '').trim();
      if (!name) return;
      setError(null);
      try {
        const res = await api.speakerAssign(recordingPath, speakerId, name, enroll);
        setNames(res.speaker_names);
        setResult(r => (r ? { ...r, speaker_names: res.speaker_names } : r));
        onInserted?.(res.inserted_block_ids);
      } catch (e) {
        setError(String(e));
      }
    },
    [recordingPath, nameDraft, onInserted],
  );

  const speakers = result
    ? [...new Set(result.turns.map(t => t.speaker).filter((s): s is string => s !== null))]
    : [];

  return (
    <Box sx={{ px: 3, py: 1.5, borderBottom: 1, borderColor: 'divider' }}>
      {error && (
        <Alert severity="error" sx={{ mb: 1 }} onClose={() => setError(null)}>
          {error}
        </Alert>
      )}

      {/* Record controls */}
      {stage === 'idle' && (
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1.5 }}>
          <Button
            size="small"
            variant="contained"
            startIcon={<MicIcon />}
            onClick={start}
            sx={{ textTransform: 'none' }}
          >
            Record voice memo
          </Button>
          <Typography variant="caption" color="text.secondary">
            Saves the clip and transcribes it into this note
          </Typography>
        </Box>
      )}

      {stage === 'recording' && (
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1.5 }}>
          <Chip
            icon={<MicIcon />}
            label={`Recording… ${fmt(elapsed)}`}
            color="error"
            size="small"
            sx={{ '& .MuiChip-icon': { animation: 'pulse 1s infinite' } }}
          />
          <Button size="small" variant="contained" startIcon={<StopIcon />} onClick={stop} sx={{ textTransform: 'none' }}>
            Stop & save
          </Button>
          <IconButton size="small" onClick={cancel} title="Cancel recording">
            <CloseIcon fontSize="small" />
          </IconButton>
        </Box>
      )}

      {stage === 'ready' && recordingPath && (
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1.5, flexWrap: 'wrap' }}>
          <Chip icon={<VolumeUpIcon />} label="Clip saved" size="small" />
          <FormControlLabel
            control={<Checkbox size="small" checked={diarize} onChange={e => setDiarize(e.target.checked)} />}
            label={<Typography variant="caption">Diarize speakers</Typography>}
          />
          <FormControlLabel
            control={<Checkbox size="small" checked={summarize} onChange={e => setSummarize(e.target.checked)} />}
            label={<Typography variant="caption">Summarize</Typography>}
          />
          <FormControlLabel
            control={<Checkbox size="small" checked={identify} onChange={e => setIdentify(e.target.checked)} />}
            label={<Typography variant="caption">Identify voices</Typography>}
          />
          <Button size="small" variant="contained" startIcon={<GraphicEqIcon />} onClick={transcribe} sx={{ textTransform: 'none' }}>
            Transcribe
          </Button>
        </Box>
      )}

      {stage === 'transcribing' && (
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1.5 }}>
          <CircularProgress size={18} />
          <Typography variant="body2">{STAGE_LABELS[stageName] ?? 'Working…'}</Typography>
        </Box>
      )}

      {/* Result + speaker assignment */}
      {stage === 'done' && result && (
        <Box>
          <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 1, flexWrap: 'wrap' }}>
            <Chip size="small" color="success" label={`${result.num_speakers} speaker${result.num_speakers === 1 ? '' : 's'}${result.diarized ? ' (diarized)' : ''}`} />
            <Chip size="small" variant="outlined" label={`${fmt(result.duration_secs)} · ${result.clip_rel_path}`} />
            {result.tags.map(t => (
              <Chip key={t} size="small" variant="outlined" label={`#${t}`} />
            ))}
            {result.related.map(r => (
              <Chip key={r} size="small" variant="outlined" label={`[[${r}]]`} />
            ))}
          </Box>

          {result.summary && (
            <Typography variant="body2" sx={{ mb: 1, fontStyle: 'italic', color: 'text.secondary' }}>
              {result.summary}
            </Typography>
          )}

          <Divider sx={{ my: 1 }} />

          {speakers.length > 0 && (
            <Box sx={{ display: 'flex', flexDirection: 'column', gap: 1, mb: 1 }}>
              <Typography variant="caption" color="text.secondary">
                Assign names to voices (persisted for future recordings):
              </Typography>
              {speakers.map(id => {
                const known = names[id];
                return (
                  <Box key={id} sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                    <Typography variant="body2" sx={{ minWidth: 90 }}>
                      {known ?? speakerLabel(id)}
                    </Typography>
                    <TextField
                      size="small"
                      placeholder="Name…"
                      value={nameDraft[id] ?? ''}
                      onChange={e => setNameDraft(d => ({ ...d, [id]: e.target.value }))}
                      sx={{ width: 160 }}
                    />
                    <Button
                      size="small"
                      variant="outlined"
                      disabled={!(nameDraft[id] ?? '').trim()}
                      onClick={() => assign(id, false)}
                      sx={{ textTransform: 'none' }}
                    >
                      Assign
                    </Button>
                    <Button
                      size="small"
                      variant="outlined"
                      color="secondary"
                      disabled={!(nameDraft[id] ?? '').trim()}
                      onClick={() => assign(id, true)}
                      sx={{ textTransform: 'none' }}
                    >
                      Assign + enroll voice
                    </Button>
                  </Box>
                );
              })}
            </Box>
          )}

          <Typography variant="caption" color="text.secondary">
            Memo inserted at the end of the note.
          </Typography>
          <Button size="small" sx={{ ml: 2, textTransform: 'none' }} onClick={() => { setStage('idle'); setResult(null); setRecordingPath(null); }}>
            New memo
          </Button>
        </Box>
      )}

      {stage === 'error' && (
        <Button size="small" onClick={() => setStage('idle')} sx={{ textTransform: 'none' }}>
          Back
        </Button>
      )}
    </Box>
  );
}
