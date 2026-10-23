//! Shared theme-editor constants for the Settings UI primitives.
//!
//! Single source of truth for the theme color presets, the settings locale set,
//! and the swatch/slider/input sizing values so the desktop ThemeTab and the
//! mobile MobileThemeSection render identical controls and stay in parity. Any
//! change to a swatch size, slider max width, spacing, or preset must be made
//! here only.

export const SETTINGS_LOCALES = ['en', 'de', 'it', 'fr', 'es', 'zh', 'ja', 'el'] as const;

export const THEME_PRIMARY_PRESETS = [
  '#f97316', '#ef4444', '#3b82f6', '#8b5cf6',
  '#10b981', '#f59e0b', '#ec4899', '#06b6d4',
] as const;

export const THEME_SECONDARY_PRESETS = [
  '#6b7280', '#78716c', '#a1a1aa', '#71717a',
  '#52525b', '#3f3f46', '#27272a',
] as const;

export const THEME_SWATCH_SIZE = 28;
export const THEME_SWATCH_GAP = 0.5;
export const THEME_COLOR_INPUT_WIDTH = { desktop: 40, mobile: 44 } as const;
export const THEME_HEX_FIELD_WIDTH = 160;
export const THEME_SLIDER_MAX_WIDTH = { desktop: 400, mobile: 300 } as const;
export const THEME_FONT_SIZE_RANGE = { min: 12, max: 28, step: 1, default: 16 } as const;
