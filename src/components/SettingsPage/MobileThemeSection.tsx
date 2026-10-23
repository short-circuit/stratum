import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import FormControl from '@mui/material/FormControl';
import InputLabel from '@mui/material/InputLabel';
import Select from '@mui/material/Select';
import MenuItem from '@mui/material/MenuItem';
import {
  SettingsSection,
  SettingsAccordion,
  SettingsRow,
  ThemeColorEditor,
  ThemeFontSizeSlider,
  SETTINGS_LOCALES,
  THEME_PRIMARY_PRESETS,
  THEME_SECONDARY_PRESETS,
} from '../ui/settings';

// ---------------------------------------------------------------------------
// Mobile theme settings section.
// Extracted from SettingsPage.mobile.tsx during the E6 sizing-gate refactor so
// the mobile settings screen stays under the 400-line component gate.
//
// Parity: mirrors desktop ThemeTab — presets AND a custom color picker + hex
// field for both primary and secondary, and the font-size scale labels.
// Stacked vertically for touch usability.
// ---------------------------------------------------------------------------

export interface MobileThemeSectionProps {
  theme: {
    dark_mode: boolean;
    primary_color: string;
    secondary_color: string;
    font_size: number;
  };
  updateTheme: (patch: any) => void;
  locale: string;
  onLocaleChange: (locale: string) => void;
}

export default function MobileThemeSection({ theme, updateTheme, locale, onLocaleChange }: MobileThemeSectionProps) {
  const { t } = useTranslation();
  const [themeExpanded, setThemeExpanded] = useState(false);
  return (
    <SettingsSection title={t('settings.theme')} variant="mobile" divider>
      <SettingsAccordion
        summary={
          <Typography variant="body2" color="text.secondary">
            {themeExpanded ? t('settings.hide_theme_config') : t('settings.configure_theme')}
          </Typography>
        }
        expanded={themeExpanded}
        onChange={setThemeExpanded}
      >
        <Box sx={{ display: 'flex', flexDirection: 'column', gap: 1.5 }}>
          <FormControl size="small" fullWidth sx={{ mb: 1.5 }}>
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
            onCheckedChange={v => updateTheme({ dark_mode: v })}
            sx={{ mb: 1.5 }}
          />
          <ThemeColorEditor
            label={t('settings.primary_buttons_accents')}
            value={theme.primary_color}
            presets={THEME_PRIMARY_PRESETS}
            onChange={v => updateTheme({ primary_color: v })}
            variant="mobile"
            sx={{ mb: 1.5 }}
          />
          <ThemeColorEditor
            label={t('settings.secondary_backgrounds_borders')}
            value={theme.secondary_color}
            presets={THEME_SECONDARY_PRESETS}
            onChange={v => updateTheme({ secondary_color: v })}
            variant="mobile"
            sx={{ mb: 1.5 }}
          />
          <ThemeFontSizeSlider
            value={theme.font_size || 16}
            onChange={v => updateTheme({ font_size: v })}
            variant="mobile"
          />
        </Box>
      </SettingsAccordion>
    </SettingsSection>
  );
}
