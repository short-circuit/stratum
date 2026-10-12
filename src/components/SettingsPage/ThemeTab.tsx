import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import Slider from '@mui/material/Slider';
import Switch from '@mui/material/Switch';
import TextField from '@mui/material/TextField';
import FormControlLabel from '@mui/material/FormControlLabel';
import ToggleButtonGroup from '@mui/material/ToggleButtonGroup';
import ToggleButton from '@mui/material/ToggleButton';
import FormControl from '@mui/material/FormControl';
import InputLabel from '@mui/material/InputLabel';
import Select from '@mui/material/Select';
import MenuItem from '@mui/material/MenuItem';
import { useTranslation } from 'react-i18next';

const LOCALES = ['en', 'de', 'it', 'fr', 'es', 'zh', 'ja', 'el'];

const PRESET_COLORS = [
  '#f97316', '#ef4444', '#3b82f6', '#8b5cf6',
  '#10b981', '#f59e0b', '#ec4899', '#06b6d4',
];

const SECONDARY_COLORS = [
  '#6b7280', '#78716c', '#a1a1aa', '#71717a',
  '#52525b', '#3f3f46', '#27272a',
];

interface ThemeTabProps {
  theme: { dark_mode: boolean; primary_color: string; secondary_color: string; font_size: number };
  onThemeChange: (patch: Partial<ThemeTabProps['theme']>) => void;
  locale: string;
  onLocaleChange: (locale: string) => void;
}

export default function ThemeTab({ theme, onThemeChange, locale, onLocaleChange }: ThemeTabProps) {
  const { t } = useTranslation();
  return (
    <Box>
      <Typography variant="subtitle2" sx={{ mb: 1.5, color: 'text.secondary' }}>{t('settings.theme')}</Typography>

      <FormControl size="small" sx={{ mb: 2, minWidth: 200 }}>
        <InputLabel>{t('settings.language_label')}</InputLabel>
        <Select
          label={t('settings.language_label')}
          value={locale}
          onChange={e => onLocaleChange(e.target.value as string)}
        >
          {LOCALES.map(code => (
            <MenuItem key={code} value={code}>
              {t(`settings.langs.${code}`)}
            </MenuItem>
          ))}
        </Select>
      </FormControl>

      <FormControlLabel
        control={<Switch checked={theme.dark_mode} onChange={e => onThemeChange({ dark_mode: e.target.checked })} />}
        label={t('settings.dark_mode')}
        sx={{ mb: 2 }}
      />

      {/* Primary color */}
      <Box sx={{ mb: 2.5 }}>
        <Typography variant="caption" sx={{ fontWeight: 500, color: 'text.secondary', display: 'block', mb: 0.75 }}>
          {t('settings.primary_buttons_accents')}
        </Typography>
        <ToggleButtonGroup
          value={theme.primary_color}
          exclusive
          onChange={(_, v) => v && onThemeChange({ primary_color: v })}
          sx={{ flexWrap: 'wrap', gap: 0.5, mb: 1 }}
        >
          {PRESET_COLORS.map(color => (
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
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
          <TextField
            type="color"
            value={theme.primary_color}
            onChange={e => onThemeChange({ primary_color: e.target.value })}
            sx={{ width: 40, '& .MuiInputBase-root': { p: 0.25 }, '& input': { cursor: 'pointer', height: 32, p: 0 } }}
          />
          <TextField
            size="small"
            value={theme.primary_color}
            onChange={e => onThemeChange({ primary_color: e.target.value })}
            placeholder="#f97316"
            sx={{ width: 160, '& .MuiInputBase-input': { fontFamily: 'monospace', fontSize: '0.75rem' } }}
          />
        </Box>
      </Box>

      {/* Secondary color */}
      <Box sx={{ mb: 2.5 }}>
        <Typography variant="caption" sx={{ fontWeight: 500, color: 'text.secondary', display: 'block', mb: 0.75 }}>
          {t('settings.secondary_backgrounds_borders')}
        </Typography>
        <ToggleButtonGroup
          value={theme.secondary_color}
          exclusive
          onChange={(_, v) => v && onThemeChange({ secondary_color: v })}
          sx={{ flexWrap: 'wrap', gap: 0.5, mb: 1 }}
        >
          {SECONDARY_COLORS.map(color => (
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
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
          <TextField
            type="color"
            value={theme.secondary_color}
            onChange={e => onThemeChange({ secondary_color: e.target.value })}
            sx={{ width: 40, '& .MuiInputBase-root': { p: 0.25 }, '& input': { cursor: 'pointer', height: 32, p: 0 } }}
          />
          <TextField
            size="small"
            value={theme.secondary_color}
            onChange={e => onThemeChange({ secondary_color: e.target.value })}
            placeholder="#6b7280"
            sx={{ width: 160, '& .MuiInputBase-input': { fontFamily: 'monospace', fontSize: '0.75rem' } }}
          />
        </Box>
      </Box>

      {/* Font size */}
      <Box sx={{ mb: 2, maxWidth: 400 }}>
        <Typography variant="caption" sx={{ fontWeight: 500, color: 'text.secondary', display: 'block', mb: 1 }}>
          {t('settings.font_size')} {theme.font_size || 16}px
        </Typography>
        <Slider
          value={theme.font_size || 16}
          min={12}
          max={28}
          step={1}
          onChange={(_, v) => onThemeChange({ font_size: v as number })}
          valueLabelDisplay="auto"
        />
        <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
          <Typography variant="caption" color="text.disabled">12px</Typography>
          <Typography variant="caption" color="text.disabled">20px</Typography>
          <Typography variant="caption" color="text.disabled">28px</Typography>
        </Box>
      </Box>
    </Box>
  );
}
