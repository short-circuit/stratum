import { useEffect, useState } from 'react';
import Dialog from '@mui/material/Dialog';
import DialogTitle from '@mui/material/DialogTitle';
import DialogContent from '@mui/material/DialogContent';
import DialogActions from '@mui/material/DialogActions';
import TextField from '@mui/material/TextField';
import Button from '@mui/material/Button';
import Typography from '@mui/material/Typography';
import IconButton from '@mui/material/IconButton';
import InputAdornment from '@mui/material/InputAdornment';
import CloseIcon from '@mui/icons-material/Close';
import { useTranslation } from 'react-i18next';
import type { SavedQuery } from '../../lib/types';

export type SavedQueryDialogKind = 'save' | 'rename';

export interface SavedQueryDialogState {
  kind: SavedQueryDialogKind;
  /** Name to prefill; empty for "save new". */
  initial: string;
}

interface Props {
  dialog: SavedQueryDialogState | null;
  /** The saved query being renamed (only for kind === 'rename'). */
  target?: SavedQuery | null;
  onDismiss: () => void;
  onSubmit: (name: string) => void;
}

/**
 * Shared modal for saving the current query under a name, or renaming an
 * existing saved query. Kept in one place so desktop and mobile stay in sync.
 */
export default function SavedQueryDialogs({ dialog, target, onDismiss, onSubmit }: Props) {
  const { t } = useTranslation();
  const [name, setName] = useState('');
  const [error, setError] = useState<string | null>(null);

  // Re-seed the name field whenever a dialog opens so stale text never leaks
  // between invocations.
  useEffect(() => {
    if (dialog) {
      setName(dialog.initial);
      setError(null);
    }
  }, [dialog]);

  const isRename = dialog?.kind === 'rename';

  const handleSubmit = () => {
    if (!name.trim()) {
      setError(t('query.name_must_not_be_empty'));
      return;
    }
    onSubmit(name.trim());
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
  };

  return (
    <Dialog open={Boolean(dialog)} onClose={onDismiss} maxWidth="xs" fullWidth>
      <DialogTitle sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <Typography variant="body1" sx={{ fontWeight: 600 }}>
          {isRename ? t('query.rename_saved_query') : t('query.save_query_label')}
        </Typography>
        <IconButton size="small" onClick={onDismiss} aria-label={t('query.close')}>
          <CloseIcon fontSize="small" />
        </IconButton>
      </DialogTitle>
      <DialogContent>
        <TextField
          autoFocus
          fullWidth
          label={t('query.name')}
          placeholder="e.g. All TODO tasks"
          value={name}
          onChange={(e) => {
            setName(e.target.value);
            if (error) setError(null);
          }}
          onKeyDown={handleKeyDown}
          error={Boolean(error)}
          helperText={error}
          sx={{ mt: 0.5 }}
          slotProps={{
            input: {
              endAdornment: name && (
                <InputAdornment position="end">
                  <IconButton size="small" onClick={() => setName('')} aria-label={t('query.clear_name')}>
                    <CloseIcon fontSize="small" />
                  </IconButton>
                </InputAdornment>
              ),
            },
          }}
        />
        {isRename && target && (
          <Typography variant="caption" color="text.disabled" sx={{ display: 'block', mt: 1 }}>
            {t('query.renaming')}{target.name}”.
          </Typography>
        )}
        <Typography variant="caption" color="text.disabled" sx={{ display: 'block', mt: 1 }}>
          {isRename
            ? t('query.renamed_saved_queries_sync_with_your_vault')
            : t('query.saved_queries_are_stored_in_your_vault_and_sync_across_devic')}
        </Typography>
      </DialogContent>
      <DialogActions sx={{ px: 3, pb: 2 }}>
        <Button onClick={onDismiss} size="small">{t('query.cancel')}</Button>
        <Button variant="contained" size="small" onClick={handleSubmit} disabled={!name.trim()}>
          {isRename ? t('query.rename') : 'Save'}
        </Button>
      </DialogActions>
    </Dialog>
  );
}
