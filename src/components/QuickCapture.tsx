import { useCallback, useEffect, useRef, useState } from 'react';
import Dialog from '@mui/material/Dialog';
import DialogTitle from '@mui/material/DialogTitle';
import DialogContent from '@mui/material/DialogContent';
import DialogActions from '@mui/material/DialogActions';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import IconButton from '@mui/material/IconButton';
import Button from '@mui/material/Button';
import Box from '@mui/material/Box';
import Snackbar from '@mui/material/Snackbar';
import Alert from '@mui/material/Alert';
import MicIcon from '@mui/icons-material/Mic';
import { useTranslation } from 'react-i18next';
import {
  onQuickCaptureOpen,
  onAppQuit,
  quickCaptureSave,
  dictationStart,
  dictationStop,
  dictationTranscribe,
} from '../lib/commands';
import { getLatestLibraryJson } from '../lib/libraryStore';
import { useSettingsStore } from '../stores/settingsStore';

function todayJournalPath(): string {
  const d = new Date();
  const m = String(d.getMonth() + 1).padStart(2, '0');
  const day = String(d.getDate()).padStart(2, '0');
  return `journals/${d.getFullYear()}-${m}-${day}.md`;
}

export default function QuickCapture() {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const [voice, setVoice] = useState(false);
  const [recording, setRecording] = useState(false);
  const [text, setText] = useState('');
  const [snack, setSnack] = useState<{ msg: string; severity: 'success' | 'error' } | null>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const todayPath = useRef(todayJournalPath()).current;
  const captureTarget = useSettingsStore(s => s.settings?.tray?.capture_target ?? 'journal');

  const showSnack = (msg: string, severity: 'success' | 'error') => setSnack({ msg, severity });

  const close = useCallback(() => {
    setOpen(false);
    setRecording(false);
    setVoice(false);
    setText('');
  }, []);

  const startRecording = useCallback(async () => {
    setRecording(true);
    try {
      await dictationStart(todayPath);
    } catch (e) {
      showSnack(String(e), 'error');
      setRecording(false);
    }
  }, [todayPath]);

  const stopRecording = useCallback(async () => {
    try {
      const stopRes = await dictationStop();
      await dictationTranscribe(stopRes.recording_path, todayPath, {
        summarize: false,
        diarize: false,
        identify: false,
      });
      showSnack(t('quickcapture.voice_done'), 'success');
      close();
    } catch (e) {
      showSnack(String(e), 'error');
      setRecording(false);
    }
  }, [todayPath, t, close]);

  const save = useCallback(async () => {
    const value = text.trim();
    if (!value) {
      close();
      return;
    }
    try {
      const res = await quickCaptureSave(value);
      showSnack(t('quickcapture.saved', { page: res.page_path }), 'success');
      close();
    } catch (e) {
      showSnack(String(e), 'error');
    }
  }, [text, t, close]);

  const handleOpen = useCallback((withVoice: boolean) => {
    setText('');
    setVoice(withVoice);
    setOpen(true);
    setSnack(null);
    if (withVoice) {
      void startRecording();
    }
  }, [startRecording]);

  useEffect(() => {
    let unlistenOpen: (() => void) | undefined;
    let unlistenQuit: (() => void) | undefined;
    let cancelled = false;

    onQuickCaptureOpen(handleOpen).then(fn => { if (cancelled) fn(); else unlistenOpen = fn; });
    onAppQuit(async () => {
      const json = getLatestLibraryJson() ?? localStorage.getItem('stratum-library');
      if (!json) return;
      try {
        const { saveLibrary } = await import('../lib/commands');
        await saveLibrary(json);
      } catch {
        // best-effort quit save
      }
    }).then(fn => { if (cancelled) fn(); else unlistenQuit = fn; });

    return () => {
      cancelled = true;
      unlistenOpen?.();
      unlistenQuit?.();
    };
  }, [handleOpen]);

  useEffect(() => {
    if (open && !voice && inputRef.current) {
      const focus = () => inputRef.current?.focus();
      if (typeof requestAnimationFrame === 'function') requestAnimationFrame(focus);
      else focus();
    }
  }, [open, voice]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      void save();
    } else if (e.key === 'Escape') {
      close();
    }
  };

  return (
    <>
      <Dialog open={open} onClose={close} maxWidth="sm" fullWidth>
        <DialogTitle>{t('quickcapture.title')}</DialogTitle>
        <DialogContent>
          {voice ? (
            <Box sx={{ display: 'flex', alignItems: 'center', gap: 1.5, py: 2 }}>
              <MicIcon color="primary" />
              <Typography sx={{ flexGrow: 1 }}>{t('quickcapture.recording')}</Typography>
              <Button variant="contained" onClick={stopRecording} disabled={!recording}>
                {t('quickcapture.stop')}
              </Button>
            </Box>
          ) : (
            <TextField
              inputRef={inputRef}
              autoFocus
              multiline
              minRows={2}
              maxRows={4}
              fullWidth
              autoComplete="off"
              placeholder={t('quickcapture.placeholder')}
              value={text}
              onChange={e => setText(e.target.value)}
              onKeyDown={handleKeyDown}
            />
          )}
        </DialogContent>
        <DialogActions>
          <Typography variant="body2" color="text.secondary" sx={{ mr: 'auto', pl: 3 }}>
            {t('quickcapture.hint_save')}
          </Typography>
          {voice ? (
            <Typography variant="body2" color="text.secondary">
              {t('quickcapture.target_label')} {t('quickcapture.target_journal')}
            </Typography>
          ) : (
            <>
              <Typography variant="body2" color="text.secondary">
                {t('quickcapture.target_label')}{' '}
                {captureTarget === 'inbox'
                  ? t('quickcapture.target_inbox')
                  : t('quickcapture.target_journal')}
              </Typography>
              <IconButton
                aria-label={t('quickcapture.voice')}
                color="primary"
                onClick={() => {
                  setVoice(true);
                  void startRecording();
                }}
              >
                <MicIcon />
              </IconButton>
            </>
          )}
        </DialogActions>
      </Dialog>
      <Snackbar
        open={!!snack}
        autoHideDuration={4000}
        onClose={() => setSnack(null)}
        anchorOrigin={{ vertical: 'bottom', horizontal: 'center' }}
      >
        <Alert severity={snack?.severity ?? 'success'} variant="filled" onClose={() => setSnack(null)}>
          {snack?.msg}
        </Alert>
      </Snackbar>
    </>
  );
}
