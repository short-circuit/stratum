//! Visual filter builder — condition list + live query preview.
//!
//! Renders the builder rows (via [`ConditionRow`]) plus the generated EDN
//! preview with Copy / Run actions. A single self-contained unit shared by the
//! desktop and mobile panel variants. Validation problems (missing value, lone
//! `or` row) are surfaced inline and disable Run.

import { useMemo } from 'react';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Typography from '@mui/material/Typography';
import Alert from '@mui/material/Alert';
import IconButton from '@mui/material/IconButton';
import Tooltip from '@mui/material/Tooltip';
import ContentCopyIcon from '@mui/icons-material/ContentCopy';
import AddIcon from '@mui/icons-material/Add';
import { useTranslation } from 'react-i18next';
import {
  FIELDS,
  type BuildResult,
  type FilterSpec,
} from '../../lib/datalogBuilder';
import ConditionRow from './ConditionRow';

export interface BuilderModeProps {
  rows: FilterSpec[];
  setRows: (updater: FilterSpec[] | ((prev: FilterSpec[]) => FilterSpec[])) => void;
  /** Live build result from `buildDatalog(rows)`. */
  generated: BuildResult;
  onRun: () => void;
  disabled?: boolean;
}

export default function BuilderMode({ rows, setRows, generated, onRun, disabled }: BuilderModeProps) {
  const { t } = useTranslation();
  const fields = useMemo(() => FIELDS, []);

  const addRow = () => {
    const id = `r${Date.now()}`;
    setRows((prev) => [
      ...prev,
      {
        id,
        field: 'content',
        op: 'contains',
        value: '',
        combiner: 'and',
      },
    ]);
  };

  const updateRow = (index: number, next: FilterSpec) => {
    setRows((prev) => prev.map((r, i) => (i === index ? next : r)));
  };

  const removeRow = (index: number) => {
    setRows((prev) => prev.filter((_, i) => i !== index));
  };

  const moveRow = (index: number, dir: -1 | 1) => {
    setRows((prev) => {
      const target = index + dir;
      if (target < 0 || target >= prev.length) return prev;
      const copy = [...prev];
      const [row] = copy.splice(index, 1);
      copy.splice(target, 0, row);
      return copy;
    });
  };

  const copyToEditor = async () => {
    try {
      await navigator.clipboard.writeText(generated.datalog);
    } catch {
      // Clipboard may be unavailable in insecure contexts — the preview is
      // still visible, so failure is non-fatal.
    }
  };

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', gap: 1 }}>
      <Box
        sx={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          flexWrap: 'wrap',
          gap: 1,
        }}
      >
        <Typography variant="body2" color="text.secondary">
          {t('query.builder_hint')}
        </Typography>
        <Button
          size="small"
          variant="outlined"
          startIcon={<AddIcon />}
          onClick={addRow}
          aria-label={t('query.add_condition')}
        >
          {t('query.add_condition')}
        </Button>
      </Box>

      {rows.length === 0 ? (
        <Typography variant="body2" color="text.secondary" sx={{ py: 1 }}>
          {t('query.builder_empty')}
        </Typography>
      ) : (
        <Box sx={{ display: 'flex', flexDirection: 'column' }}>
          {rows.map((row, i) => (
            <ConditionRow
              key={row.id}
              row={row}
              index={i}
              isFirst={i === 0}
              fields={fields}
              onChange={(next) => updateRow(i, next)}
              onRemove={() => removeRow(i)}
              onMoveUp={() => moveRow(i, -1)}
              onMoveDown={() => moveRow(i, 1)}
            />
          ))}
        </Box>
      )}

      <Box
        sx={{
          mt: 1,
          p: 1.25,
          borderRadius: 1,
          bgcolor: 'action.hover',
          fontFamily: 'monospace',
          fontSize: '0.8125rem',
          whiteSpace: 'pre-wrap',
          wordBreak: 'break-word',
          display: 'flex',
          flexDirection: 'column',
          gap: 1,
        }}
      >
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
          <Typography
            variant="caption"
            sx={{
              fontWeight: 600,
              color: generated.valid ? 'text.secondary' : 'error.main',
            }}
          >
            {t('query.generated_query')}
          </Typography>
          <Box sx={{ flex: 1 }} />
          {generated.valid && (
            <Tooltip title={t('query.copy_to_editor')} arrow>
              <span>
                <IconButton
                  size="small"
                  onClick={copyToEditor}
                  aria-label={t('query.copy_to_editor')}
                >
                  <ContentCopyIcon fontSize="small" />
                </IconButton>
              </span>
            </Tooltip>
          )}
        </Box>
        {generated.valid ? (
          <Typography
            variant="caption"
            component="pre"
            sx={{ m: 0, fontFamily: 'inherit', whiteSpace: 'pre-wrap', wordBreak: 'break-word' }}
          >
            {generated.datalog}
          </Typography>
        ) : (
          <Typography variant="caption" color="error.main">
            {generated.error ? t(generated.error) : generated.error}
          </Typography>
        )}
      </Box>

      {generated.valid && (
        <Button
          variant="contained"
          size="small"
          onClick={onRun}
          disabled={disabled}
          sx={{ alignSelf: 'flex-start' }}
        >
          {disabled ? t('query.running') : t('query.run')}
        </Button>
      )}

      {!generated.valid && (
        <Alert severity="error" sx={{ py: 0 }}>
          {generated.error ? t(generated.error) : generated.error}
        </Alert>
      )}
    </Box>
  );
}
