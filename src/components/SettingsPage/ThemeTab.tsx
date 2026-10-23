import { useTranslation } from 'react-i18next';
import FormControl from '@mui/material/FormControl';
import InputLabel from '@mui/material/InputLabel';
import Select from '@mui/material/Select';
import MenuItem from '@mui/material/MenuItem';
import {
  SettingsSection,
  SettingsRow,
  ThemeColorEditor,
  ThemeFontSizeSlider,
  SETTINGS_LOCALES,
  THEME_PRIMARY_PRESETS,
  THEME_SECONDARY_PRESETS,
} from '../ui/settings';

interface ThemeTabProps {
  theme: { dark_mode: boolean; primary_color: string; secondary_color: string; font_size: number };
  onThemeChange: (patch: Partial<ThemeTabProps['theme']>) => void;
  locale: string;
  onLocaleChange: (locale: string) => void;
}

export default function ThemeTab({ theme, onThemeChange, locale, onLocaleChange }: ThemeTabProps) {
  const { t } = useTranslation();
  return (
    <SettingsSection title={t('settings.theme')} variant="desktop">
      <FormControl size="small" sx={{ mb: 2, minWidth: 200 }}>
        <InputLabel>{t('settings.language_label')}</InputLabel>
        <Select
          label={t('settings.language_label')}
          value={locale}
          onChange={e => onLocaleChange(e.target.value as string)}
        >
          {SETTINGS_LOCALES.map(code => (
            <MenuItem key={code} value={code}>
              {t(`settings.langs.${code}`)}
            </MenuItem>
          ))}
        </Select>
      </FormControl>

      <SettingsRow
        label={t('settings.dark_mode')}
        checked={theme.dark_mode}
        onCheckedChange={v => onThemeChange({ dark_mode: v })}
        sx={{ mb: 2 }}
      />

      <ThemeColorEditor
        label={t('settings.primary_buttons_accents')}
        value={theme.primary_color}
        presets={THEME_PRIMARY_PRESETS}
        onChange={v => onThemeChange({ primary_color: v })}
        variant="desktop"
        sx={{ mb: 2.5 }}
      />

      <ThemeColorEditor
        label={t('settings.secondary_backgrounds_borders')}
        value={theme.secondary_color}
        presets={THEME_SECONDARY_PRESETS}
        onChange={v => onThemeChange({ secondary_color: v })}
        variant="desktop"
        sx={{ mb: 2.5 }}
      />

      <ThemeFontSizeSlider
        value={theme.font_size || 16}
        onChange={v => onThemeChange({ font_size: v })}
        variant="desktop"
        sx={{ mb: 2 }}
      />
    </SettingsSection>
  );
}
