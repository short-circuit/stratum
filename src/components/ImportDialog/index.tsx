import { useState } from 'react';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import FormControl from '@mui/material/FormControl';
import InputLabel from '@mui/material/InputLabel';
import MenuItem from '@mui/material/MenuItem';
import Select from '@mui/material/Select';
import type { SelectChangeEvent } from '@mui/material/Select';
import CircularProgress from '@mui/material/CircularProgress';
import Alert from '@mui/material/Alert';
import Typography from '@mui/material/Typography';
import List from '@mui/material/List';
import ListItem from '@mui/material/ListItem';
import DialogTitle from '@mui/material/DialogTitle';
import DialogContent from '@mui/material/DialogContent';
import DialogActions from '@mui/material/DialogActions';
import FolderOpenIcon from '@mui/icons-material/FolderOpen';
import { useTranslation } from 'react-i18next';
import { ResponsiveDialog } from '../ui/ResponsiveDialog';
import { useStore } from '../../stores/appStore';
import { previewImport, importVault } from '../../lib/commands/import';
import type { ImportPreview, ImportReport, ImportFormat } from '../../lib/types';

interface Props {
  open: boolean;
  onClose: () => void;
}

/** Grouped preview counts, detected format, and any duplicate/conflict lists. */
interface PreviewSummaryProps {
  preview: ImportPreview;
}

/** Post-import report including the i18n summary string and per-error alerts. */
interface ReportSummaryProps {
  report: ImportReport;
}

function PreviewSummary({ preview }: PreviewSummaryProps) {
  const { t } = useTranslation();
  const empty = preview.total_pages === 0 && preview.total_assets === 0;

  return (
    <Box sx={{ mt: 2 }}>
      <Typography variant="caption" color="text.secondary" sx={{ display: 'block' }}>
        {t('importers.source_folder')}: {preview.source_path}
      </Typography>
      <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 1 }}>
        {t('importers.format')}: {t(`importers.format_${preview.format}`)}
      </Typography>
      {empty ? (
        <Typography variant="body2" color="text.secondary">{t('importers.preview_empty')}</Typography>
      ) : (
        <>
          <Typography variant="body2">
            {t('importers.pages')} ({preview.total_pages}) · {t('importers.assets')} ({preview.total_assets})
          </Typography>
          {preview.duplicates.length > 0 && (
            <Box sx={{ mt: 1 }}>
              <Typography variant="body2">{t('importers.duplicates')}</Typography>
              <List dense disablePadding>
                {preview.duplicates.map(d => (
                  <ListItem key={d} dense sx={{ px: 0 }}>
                    <Typography variant="caption">{d}</Typography>
                  </ListItem>
                ))}
              </List>
            </Box>
          )}
          {preview.conflicts.length > 0 && (
            <Box sx={{ mt: 1 }}>
              <Typography variant="body2">{t('importers.conflicts')}</Typography>
              <List dense disablePadding>
                {preview.conflicts.map(c => (
                  <ListItem key={c} dense sx={{ px: 0 }}>
                    <Typography variant="caption">{c}</Typography>
                  </ListItem>
                ))}
              </List>
            </Box>
          )}
        </>
      )}
    </Box>
  );
}

function ReportSummary({ report }: ReportSummaryProps) {
  const { t } = useTranslation();

  return (
    <Box sx={{ mt: 2 }}>
      <Typography variant="h6">{t('importers.report_title')}</Typography>
      <Typography variant="body2">
        {t('importers.report_summary', {
          pages: report.pages_imported,
          assets: report.assets_imported,
          duplicates: report.duplicates_skipped,
          conflicts: report.conflicts_skipped,
        })}
      </Typography>
      {report.errors.map((err, i) => (
        <Alert key={i} severity="error" sx={{ mt: 1 }}>{err}</Alert>
      ))}
      <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mt: 1 }}>
        {t('importers.undo_note')}
      </Typography>
    </Box>
  );
}

/**
 * One-click vault importer dialog.
 *
 * Lets the user pick a source folder, choose (or auto-detect) the source
 * format, preview what would be imported, then run the import. On a successful
 * (non-dry) import the page list is refreshed and the report is shown inline;
 * the dialog stays open so the user can review the outcome before closing.
 */
export default function ImportDialog({ open, onClose }: Props) {
  const { t } = useTranslation();
  const [sourcePath, setSourcePath] = useState<string | null>(null);
  const [displayPath, setDisplayPath] = useState('');
  const [format, setFormat] = useState<ImportFormat>('auto');
  const [preview, setPreview] = useState<ImportPreview | null>(null);
  const [report, setReport] = useState<ImportReport | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [previewing, setPreviewing] = useState(false);
  const [importing, setImporting] = useState(false);

  const pickSource = async () => {
    try {
      // Native folder picker is only loaded on demand so desktop and web
      // builds (which lack the Tauri dialog plugin) never import it eagerly.
      const { open: pick } = await import('@tauri-apps/plugin-dialog');
      const selected = await pick({ directory: true, multiple: false });
      if (typeof selected === 'string') {
        setSourcePath(selected);
        setDisplayPath(selected);
        setPreview(null);
        setReport(null);
        setError(null);
      }
    } catch {
      setError(t('importers.source_not_found'));
    }
  };

  const handleFormatChange = (e: SelectChangeEvent) => {
    setFormat(e.target.value as ImportFormat);
  };

  const handlePreview = async () => {
    if (!sourcePath) return;
    setPreviewing(true);
    setError(null);
    try {
      const p = await previewImport(sourcePath, { format, dry_run: true });
      setPreview(p);
    } catch {
      setError(t('importers.detect_failed'));
    } finally {
      setPreviewing(false);
    }
  };

  const handleImport = async () => {
    if (!sourcePath) return;
    setImporting(true);
    setError(null);
    try {
      const r = await importVault(sourcePath, { format, dry_run: false });
      setReport(r);
      // Refresh the page list so newly imported pages appear immediately.
      void useStore.getState().loadPages();
    } catch {
      setError(t('importers.detect_failed'));
    } finally {
      setImporting(false);
    }
  };

  return (
    <ResponsiveDialog open={open} onClose={onClose} maxWidth="sm" fullWidth>
      <DialogTitle>{t('importers.import_notes')}</DialogTitle>
      <DialogContent dividers>
        <Typography variant="body2" sx={{ mb: 2 }}>
          {t('importers.select_source_folder')}
        </Typography>

        <Box sx={{ display: 'flex', gap: 1, alignItems: 'center', mb: 2 }}>
          <Typography variant="body2" color="text.secondary" noWrap sx={{ flex: 1 }}>
            {displayPath || t('importers.no_selection')}
          </Typography>
          <Button
            variant="outlined"
            size="small"
            onClick={pickSource}
            startIcon={<FolderOpenIcon />}
          >
            {t('importers.browse')}
          </Button>
        </Box>

        <FormControl size="small" sx={{ minWidth: 180, mb: 2 }}>
          <InputLabel id="import-format-label">{t('importers.format')}</InputLabel>
          <Select
            labelId="import-format-label"
            label={t('importers.format')}
            value={format}
            onChange={handleFormatChange}
          >
            <MenuItem value="auto">{t('importers.format_auto')}</MenuItem>
            <MenuItem value="obsidian">{t('importers.format_obsidian')}</MenuItem>
            <MenuItem value="logseq">{t('importers.format_logseq')}</MenuItem>
            <MenuItem value="generic">{t('importers.format_generic')}</MenuItem>
          </Select>
        </FormControl>

        {error && <Alert severity="error" sx={{ mb: 2 }}>{error}</Alert>}

        {sourcePath && (
          <Box sx={{ display: 'flex', gap: 1, mb: 1 }}>
            <Button
              variant="contained"
              size="small"
              onClick={handlePreview}
              disabled={previewing || importing}
            >
              {t('importers.preview')}
            </Button>
            <Button
              variant="contained"
              color="success"
              size="small"
              onClick={handleImport}
              disabled={importing}
              startIcon={importing ? <CircularProgress size={16} /> : undefined}
            >
              {importing ? t('importers.importing') : t('importers.import')}
            </Button>
          </Box>
        )}

        {previewing && (
          <Box sx={{ display: 'flex', justifyContent: 'center', py: 3 }}>
            <CircularProgress size={24} />
          </Box>
        )}

        {!previewing && preview && <PreviewSummary preview={preview} />}
        {!previewing && report && <ReportSummary report={report} />}
      </DialogContent>
      <DialogActions>
        <Button size="small" onClick={onClose}>{t('importers.close')}</Button>
      </DialogActions>
    </ResponsiveDialog>
  );
}
