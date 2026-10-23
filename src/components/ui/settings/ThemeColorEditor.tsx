//! Shared theme color editor.
//!
//! Renders a preset swatch picker plus a native color input and a hex text
//! field for custom colors. Shared by the desktop ThemeTab and the mobile
//! MobileThemeSection so both surfaces render identical, in-parity controls.
//! Accessible names for the native color input and hex field are localized.

import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import TextField from '@mui/material/TextField';
import ToggleButtonGroup from '@mui/material/ToggleButtonGroup';
import ToggleButton from '@mui/material/ToggleButton';
import { useTranslation } from 'react-i18next';
import type { SxProps, Theme } from '@mui/material';
import {
  THEME_SWATCH_SIZE,
  THEME_SWATCH_GAP,
  THEME_COLOR_INPUT_WIDTH,
  THEME_HEX_FIELD_WIDTH,
} from './themeConstants';

interface ThemeColorEditorProps {
  label: string;
  value: string;
  presets: readonly string[];
  onChange: (v: string) => void;
  variant?: 'desktop' | 'mobile';
  sx?: SxProps<Theme>;
}

export default function ThemeColorEditor({
  label,
  value,
  presets,
  onChange,
  variant = 'desktop',
  sx,
}: ThemeColorEditorProps) {
  const { t } = useTranslation();
  return (
    <Box sx={sx}>
      <Typography variant="caption" sx={{ fontWeight: 500, color: 'text.secondary', display: 'block', mb: 0.75 }}>
        {label}
      </Typography>
      <ToggleButtonGroup
        value={value}
        exclusive
        onChange={(_, v) => v && onChange(v)}
        size="small"
        sx={{ flexWrap: 'wrap', gap: THEME_SWATCH_GAP, mb: variant === 'mobile' ? 0.5 : 1 }}
      >
        {presets.map(color => (
          <ToggleButton
            key={color}
            value={color}
            size="small"
            sx={{
              width: THEME_SWATCH_SIZE,
              height: THEME_SWATCH_SIZE,
              minWidth: THEME_SWATCH_SIZE,
              p: 0,
              borderRadius: '50%!important',
              border: 2,
              borderColor: value === color ? 'text.primary' : 'transparent',
              bgcolor: color,
              '&:hover': { bgcolor: color },
              '&.Mui-selected': { bgcolor: color, '&:hover': { bgcolor: color } },
            }}
          />
        ))}
      </ToggleButtonGroup>
      <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 1.5 }}>
        <TextField
          type="color"
          value={value}
          onChange={e => onChange(e.target.value)}
          sx={{
            width: THEME_COLOR_INPUT_WIDTH[variant],
            '& .MuiInputBase-root': { p: 0.25 },
            '& input': { cursor: 'pointer', height: 32, p: 0 },
          }}
          slotProps={{ input: { 'aria-label': t('settings.pick_color') } }}
        />
        <TextField
          size="small"
          value={value}
          onChange={e => onChange(e.target.value)}
          sx={{
            ...(variant === 'desktop' ? { width: THEME_HEX_FIELD_WIDTH } : { flex: 1 }),
            '& .MuiInputBase-input': { fontFamily: 'monospace', fontSize: '0.75rem' },
          }}
          slotProps={{ input: { 'aria-label': t('settings.hex_color') } }}
        />
      </Box>
    </Box>
  );
}
