import { useEffect, useState } from 'react';
import Dialog from '@mui/material/Dialog';
import DialogTitle from '@mui/material/DialogTitle';
import DialogContent from '@mui/material/DialogContent';
import DialogActions from '@mui/material/DialogActions';
import TextField from '@mui/material/TextField';
import Button from '@mui/material/Button';
import Typography from '@mui/material/Typography';
import { useTranslation } from 'react-i18next';
import type { PageDto } from '../../lib/types';

interface Props {
  open: boolean;
  /** The page being moved; null when closed. */
  page: PageDto | null;
  /** Full page list used for the "already exists" conflict check. */
  pages: PageDto[];
  onClose: () => void;
  /** Called with (oldPath, newPath-normalized); resolves true on success (parent
   *  performs navigation + success toast), false on failure (dialog stays open). */
  onSubmit: (oldPath: string, newPath: string) => Promise<boolean>;
}

const normalizePath = (raw: string): string => {
  const t = raw.trim();
  return t && !t.endsWith('.md') ? t + '.md' : t;
};

/**
 * Controlled modal for moving/renaming a page by path. The backend rewrites
 * inbound wiki-links on move; this dialog is path-only (no title field) because
 * `move_page` does not rewrite the title.
 */
export default function MovePageDialog({ open, page, pages, onClose, onSubmit }: Props) {
  const { t } = useTranslation();
  const [path, setPath] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (open && page) {
      setPath(page.path);
      setError(null);
      setSaving(false);
    }
  }, [open, page]);

  const handleSubmit = () => {
    const final = normalizePath(path);
    if (!final) {
      setError(t('sidebar.path_must_not_be_empty'));
      return;
    }
    if (!final.startsWith('pages/')) {
      setError(t('sidebar.path_must_start_with_pages'));
      return;
    }
    if (final !== page?.path && pages.some(p => p.path === final)) {
      setError(t('sidebar.page_already_exists', { path: final }));
      return;
    }
    setSaving(true);
    void onSubmit(page!.path, final)
      .then(ok => {
        if (ok) onClose();
      })
      .finally(() => setSaving(false));
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
  };

  return (
    <Dialog open={open} onClose={onClose} maxWidth="xs" fullWidth>
      <DialogTitle>
        <Typography variant="body1" sx={{ fontWeight: 600 }}>
          {t('sidebar.move_page_title')}
        </Typography>
      </DialogTitle>
      <DialogContent>
        <Typography variant="caption" color="text.disabled" sx={{ display: 'block' }}>
          {t('sidebar.current_path')}: {page?.path}
        </Typography>
        <TextField
          autoFocus
          fullWidth
          label={t('sidebar.new_path')}
          value={path}
          placeholder={t('sidebar.new_path_placeholder')}
          onChange={e => {
            setPath(e.target.value);
            if (error) setError(null);
          }}
          onKeyDown={handleKeyDown}
          error={Boolean(error)}
          helperText={error}
          sx={{ mt: 1 }}
        />
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose} size="small">{t('sidebar.cancel')}</Button>
        <Button variant="contained" size="small" disabled={saving} onClick={handleSubmit}>
          {t('sidebar.move')}
        </Button>
      </DialogActions>
    </Dialog>
  );
}
