//! Shared theme font-size slider.
//!
//! Renders a labeled font-size slider with min/max scale captions. Shared by
//! the desktop ThemeTab and the mobile MobileThemeSection so both surfaces
//! render identical, in-parity controls.

import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import Slider from '@mui/material/Slider';
import { useTranslation } from 'react-i18next';
import type { SxProps, Theme } from '@mui/material';
import { THEME_SLIDER_MAX_WIDTH, THEME_FONT_SIZE_RANGE } from './themeConstants';

interface ThemeFontSizeSliderProps {
  value: number;
  onChange: (v: number) => void;
  variant?: 'desktop' | 'mobile';
  sx?: SxProps<Theme>;
}

export default function ThemeFontSizeSlider({ value, onChange, variant = 'desktop', sx }: ThemeFontSizeSliderProps) {
  const { t } = useTranslation();
  return (
    <Box sx={{ maxWidth: THEME_SLIDER_MAX_WIDTH[variant], ...sx }}>
      <Typography variant="caption" sx={{ fontWeight: 500, color: 'text.secondary', display: 'block', mb: 1 }}>
        {t('settings.font_size')} {value}px
      </Typography>
      <Slider
        value={value}
        min={THEME_FONT_SIZE_RANGE.min}
        max={THEME_FONT_SIZE_RANGE.max}
        step={THEME_FONT_SIZE_RANGE.step}
        onChange={(_, v) => onChange(v as number)}
        valueLabelDisplay="auto"
      />
      <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
        <Typography variant="caption" color="text.disabled">12px</Typography>
        <Typography variant="caption" color="text.disabled">20px</Typography>
        <Typography variant="caption" color="text.disabled">28px</Typography>
      </Box>
    </Box>
  );
}
