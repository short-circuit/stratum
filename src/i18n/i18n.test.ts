import { describe, it, expect } from 'vitest';
import i18n, {
  initI18n,
  resolveLocale,
  formatDate,
  formatNumber,
  localizedMonthNames,
  localizedDayHeaders,
} from './index';

describe('i18n foundation', () => {
  it('renders the en value (not the raw key) when locale is en', async () => {
    await initI18n('en');
    expect(i18n.t('common.cancel')).toBe('Cancel');
    expect(i18n.t('common.cancel')).not.toBe('common.cancel');
  });

  it('switches to de and returns the native German value', async () => {
    await initI18n('de');
    expect(i18n.t('settings.language')).toBe('Sprache');
  });

  it('zh placeholder returns the en value without crashing', async () => {
    await initI18n('zh');
    // zh/ja/el mirror en, so the value equals the en source value (no missing-key crash).
    expect(i18n.t('settings.language')).toBe('Language');
    expect(i18n.t('common.cancel')).toBe('Cancel');
  });
});

describe('i18n locale formatting helpers', () => {
  it('resolves an en BCP-47 tag and formats dates/numbers', async () => {
    await initI18n('en');
    expect(resolveLocale()).toBe('en-US');
    expect(formatDate('2023-01-02')).toMatch(/2023/);
    expect(formatNumber(1234.5)).toBe('1,234.5');
  });

  it('returns 12 month names and 7 Sunday-first weekday headers', async () => {
    await initI18n('en');
    const months = localizedMonthNames();
    const days = localizedDayHeaders();
    expect(months).toHaveLength(12);
    expect(days).toHaveLength(7);
    // First header must be Sunday (2000-01-02 is a Sunday) for the en locale.
    expect(days[0]).toBe('Sun');
  });
});
