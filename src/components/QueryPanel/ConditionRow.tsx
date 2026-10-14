//! Visual filter builder — one condition row.
//!
//! Renders a single [`FilterSpec`] row: combiner select (AND/OR, hidden on the
//! first row), field select, operator select (options restricted to the
//! selected field), value input (hidden for present/absent), move up/down and
//! remove buttons. Every control carries an aria-label including the row index
//! so the builder is fully keyboard/screen-reader operable.

import Box from '@mui/material/Box';
import TextField from '@mui/material/TextField';
import Select, { type SelectChangeEvent } from '@mui/material/Select';
import IconButton from '@mui/material/IconButton';
import Tooltip from '@mui/material/Tooltip';
import ArrowUpwardIcon from '@mui/icons-material/ArrowUpward';
import ArrowDownwardIcon from '@mui/icons-material/ArrowDownward';
import CloseIcon from '@mui/icons-material/Close';
import { useTranslation } from 'react-i18next';
import {
  FIELDS,
  OPS,
  type FieldDef,
  type FilterSpec,
  type OpId,
} from '../../lib/datalogBuilder';

export interface ConditionRowProps {
  row: FilterSpec;
  index: number;
  isFirst: boolean;
  /** Full whitelist — the field select options come from FieldDef[] (UI may filter). */
  fields: FieldDef[];
  onChange: (next: FilterSpec) => void;
  onRemove: () => void;
  onMoveUp: () => void;
  onMoveDown: () => void;
}

export default function ConditionRow({
  row,
  index,
  isFirst,
  fields,
  onChange,
  onRemove,
  onMoveUp,
  onMoveDown,
}: ConditionRowProps) {
  const { t } = useTranslation();
  const fieldDef = fields.find((f) => f.id === row.field) ?? fields[0];
  const opDef = OPS[row.op];
  const needsValue = opDef?.needsValue ?? false;
  const canMoveUp = !isFirst;
  const combiner = isFirst ? 'and' : row.combiner;

  const handleField = (e: SelectChangeEvent<string>) => {
    const next = FIELDS.find((f) => f.id === e.target.value);
    if (!next) return;
    // Changing the field may make the current op invalid — fall back to the
    // first op the new field supports.
    const op = next.ops.includes(row.op) ? row.op : next.ops[0];
    onChange({ ...row, field: next.id, op });
  };

  const handleOp = (e: SelectChangeEvent<string>) => {
    onChange({ ...row, op: e.target.value as OpId });
  };

  return (
    <Box
      sx={{
        display: 'flex',
        alignItems: 'center',
        gap: 1,
        flexWrap: 'wrap',
        py: 0.5,
      }}
    >
      {/* Combiner */}
      <Select
        native
        size="small"
        value={combiner}
        onChange={(e) => onChange({ ...row, combiner: e.target.value as 'and' | 'or' })}
        disabled={isFirst}
        aria-label={`${t('query.combiner')} ${index + 1}`}
        sx={{ minWidth: 76, visibility: isFirst ? 'hidden' : 'visible' }}
      >
        <option value="and">{t('query.and')}</option>
        <option value="or">{t('query.or')}</option>
      </Select>

      {/* Field */}
      <Select
        native
        size="small"
        value={row.field}
        onChange={handleField}
        aria-label={`${t('query.field')} ${index + 1}`}
        sx={{ minWidth: 170 }}
      >
        {fields.map((f) => (
          <option key={f.id} value={f.id}>
            {t(f.labelKey)}
          </option>
        ))}
      </Select>

      {/* Operator */}
      <Select
        native
        size="small"
        value={row.op}
        onChange={handleOp}
        aria-label={`${t('query.op')} ${index + 1}`}
        sx={{ minWidth: 130 }}
      >
        {fieldDef.ops.map((op) => (
          <option key={op} value={op}>
            {t(OPS[op].labelKey)}
          </option>
        ))}
      </Select>

      {/* Value */}
      {needsValue && (
        <TextField
          size="small"
          value={row.value}
          onChange={(e) => onChange({ ...row, value: e.target.value })}
          label={`${t('query.value')} ${index + 1}`}
          placeholder={
            row.field === 'marker'
              ? 'TODO'
              : row.field === 'priority'
                ? 'A'
                : row.field === 'has_wikilink'
                  ? 'page'
                  : ''
          }
          slotProps={{
            htmlInput: { 'aria-label': `${t('query.value')} ${index + 1}` },
          }}
          sx={{ minWidth: 160 }}
        />
      )}

      {/* Move / remove */}
      <Box sx={{ display: 'flex', ml: 'auto' }}>
        <Tooltip title={t('query.move_up')} arrow>
          <span>
            <IconButton
              size="small"
              onClick={onMoveUp}
              disabled={!canMoveUp}
              aria-label={`${t('query.move_up')} ${index + 1}`}
            >
              <ArrowUpwardIcon fontSize="small" />
            </IconButton>
          </span>
        </Tooltip>
        <Tooltip title={t('query.move_down')} arrow>
          <span>
            <IconButton
              size="small"
              onClick={onMoveDown}
              disabled={false}
              aria-label={`${t('query.move_down')} ${index + 1}`}
            >
              <ArrowDownwardIcon fontSize="small" />
            </IconButton>
          </span>
        </Tooltip>
        <Tooltip title={t('query.remove_condition')} arrow>
          <IconButton
            size="small"
            color="error"
            onClick={onRemove}
            aria-label={`${t('query.remove_condition')} ${index + 1}`}
          >
            <CloseIcon fontSize="small" />
          </IconButton>
        </Tooltip>
      </Box>
    </Box>
  );
}
