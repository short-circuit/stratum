import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import Divider from '@mui/material/Divider';
import Switch from '@mui/material/Switch';
import FormControlLabel from '@mui/material/FormControlLabel';
import Slider from '@mui/material/Slider';
import ToggleButtonGroup from '@mui/material/ToggleButtonGroup';
import ToggleButton from '@mui/material/ToggleButton';

// ---------------------------------------------------------------------------
// Mobile theme settings section.
// Extracted from SettingsPage.mobile.tsx during the E6 sizing-gate refactor so
// the mobile settings screen stays under the 400-line component gate.
// ---------------------------------------------------------------------------

const PRIMARY_SWATCHES = [
  '#f97316', '#ef4444', '#3b82f6', '#8b5cf6',
  '#10b981', '#f59e0b', '#ec4899', '#06b6d4',
];

const SECONDARY_SWATCHES = [
  '#6b7280', '#78716c', '#a1a1aa', '#71717a',
  '#52525b', '#3f3f46', '#27272a',
];

export interface MobileThemeSectionProps {
  theme: {
    dark_mode: boolean;
    primary_color: string;
    secondary_color: string;
    font_size: number;
  };
  updateTheme: (patch: any) => void;
}

export default function MobileThemeSection({ theme, updateTheme }: MobileThemeSectionProps) {
  return (
    <Box sx={{ px: 2, pt: 3 }}>
      <Typography variant="subtitle1" sx={{ fontWeight: 600, mb: 1 }}>
        Theme
      </Typography>
      <Divider sx={{ mb: 1.5 }} />
      <FormControlLabel
        control={
          <Switch
            checked={theme.dark_mode}
            onChange={e => updateTheme({ dark_mode: e.target.checked })}
          />
        }
        label="Dark mode"
        sx={{ mb: 1.5 }}
      />
      <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 0.5 }}>
        Primary color
      </Typography>
      <ToggleButtonGroup
        value={theme.primary_color}
        exclusive
        onChange={(_, v) => v && updateTheme({ primary_color: v })}
        sx={{ flexWrap: 'wrap', gap: 0.5, mb: 1.5 }}
      >
        {PRIMARY_SWATCHES.map(color => (
          <ToggleButton
            key={color}
            value={color}
            size="small"
            sx={{
              width: 28, height: 28, minWidth: 28, p: 0, borderRadius: '50%!important',
              border: 2, borderColor: theme.primary_color === color ? 'text.primary' : 'transparent',
              bgcolor: color, '&:hover': { bgcolor: color },
              '&.Mui-selected': { bgcolor: color, '&:hover': { bgcolor: color } },
            }}
          />
        ))}
      </ToggleButtonGroup>
      <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 0.5 }}>
        Secondary color
      </Typography>
      <ToggleButtonGroup
        value={theme.secondary_color}
        exclusive
        onChange={(_, v) => v && updateTheme({ secondary_color: v })}
        sx={{ flexWrap: 'wrap', gap: 0.5, mb: 1.5 }}
      >
        {SECONDARY_SWATCHES.map(color => (
          <ToggleButton
            key={color}
            value={color}
            size="small"
            sx={{
              width: 28, height: 28, minWidth: 28, p: 0, borderRadius: '50%!important',
              border: 2, borderColor: theme.secondary_color === color ? 'text.primary' : 'transparent',
              bgcolor: color, '&:hover': { bgcolor: color },
              '&.Mui-selected': { bgcolor: color, '&:hover': { bgcolor: color } },
            }}
          />
        ))}
      </ToggleButtonGroup>
      <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 0.5 }}>
        Font Size: {theme.font_size || 16}px
      </Typography>
      <Slider
        value={theme.font_size || 16}
        min={12}
        max={28}
        step={1}
        onChange={(_, v) => updateTheme({ font_size: v as number })}
        valueLabelDisplay="auto"
        sx={{ mb: 1, maxWidth: 300 }}
      />
    </Box>
  );
}
