//! i18n bootstrap for Stratum.
//!
//! Configures a single global i18next instance with the 8 bundled locales
//! (ISO 639-1), wired through `react-i18next` for `useTranslation()` in
//! components. Locale detection uses localStorage (key `stratum-locale`) then
//! the browser navigator, with `en` always available as the fallback so a
//! missing key never crashes or renders a raw key.
//!
//! Also exposes locale-aware formatting helpers (dates, numbers, calendar
//! labels) for the component-string sweep tasks that run after this phase.

import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import LanguageDetector from 'i18next-browser-languagedetector';

import en from './locales/en.json';
import de from './locales/de.json';
import it from './locales/it.json';
import fr from './locales/fr.json';
import es from './locales/es.json';
import zh from './locales/zh.json';
import ja from './locales/ja.json';
import el from './locales/el.json';

/** Supported UI languages (ISO 639-1). `en` is the fallback and source of truth. */
export const supportedLngs = ['en', 'de', 'it', 'fr', 'es', 'zh', 'ja', 'el'] as const;

/** localStorage key used by the language detector cache. */
export const LOCALE_STORAGE_KEY = 'stratum-locale';

const resources = {
  en: { translation: en },
  de: { translation: de },
  it: { translation: it },
  fr: { translation: fr },
  es: { translation: es },
  zh: { translation: zh },
  ja: { translation: ja },
  el: { translation: el },
} as const;

let initialized = false;

/**
 * Initialize the global i18next instance.
 *
 * Safe to call multiple times (main.tsx boot + tests). If already initialized,
 * an optional `explicitLocale` re-targets the current language immediately.
 * A failure here never throws into the boot path (callers may best-effort it).
 */
export async function initI18n(explicitLocale?: string): Promise<void> {
  if (initialized) {
    if (explicitLocale && isSupported(explicitLocale)) {
      await i18n.changeLanguage(explicitLocale);
    }
    return;
  }

  await i18n.use(LanguageDetector).use(initReactI18next).init({
    resources,
    supportedLngs,
    fallbackLng: 'en',
    ns: ['translation'],
    defaultNS: 'translation',
    keySeparator: '.',
    nsSeparator: false,
    interpolation: { escapeValue: false, prefix: '{{', suffix: '}}' },
    detection: {
      order: ['localStorage', 'navigator'],
      caches: ['localStorage'],
      lookupLocalStorage: LOCALE_STORAGE_KEY,
    },
  });

  initialized = true;

  if (explicitLocale && isSupported(explicitLocale)) {
    await i18n.changeLanguage(explicitLocale);
  }
}

function isSupported(locale: string): boolean {
  return (supportedLngs as readonly string[]).includes(locale);
}

/**
 * Set the active UI locale (persisted via the detector's localStorage cache).
 * Unknown locales fall back to `en`.
 */
export async function setLocale(locale: string): Promise<void> {
  const target = isSupported(locale) ? locale : 'en';
  await i18n.changeLanguage(target);
  // Ensure persistence even if the detector cache is not wired to react to
  // changeLanguage in some environment (e.g. jsdom tests).
  try {
    window.localStorage.setItem(LOCALE_STORAGE_KEY, target);
  } catch {
    // localStorage unavailable (e.g. privacy mode) — ignore.
  }
}

/** Current normalized base locale (e.g. `de`), falling back to `en`. */
export function getLocale(): string {
  const raw = i18n.language || 'en';
  const base = raw.split('-')[0];
  return isSupported(base) ? base : 'en';
}

/**
 * Resolve a BCP-47 tag safe for `Intl`, converting the internal `en` base to
 * `en-US` (other bases are already valid ISO 639-1 tags usable by `Intl`).
 */
export function resolveLocale(): string {
  const base = getLocale();
  return base === 'en' ? 'en-US' : base;
}

/** Format a date using the current locale (default: YYYY-MM-DD). */
export function formatDate(iso: string | Date, opts?: Intl.DateTimeFormatOptions): string {
  const date = typeof iso === 'string' ? new Date(iso) : iso;
  return new Intl.DateTimeFormat(
    resolveLocale(),
    opts ?? { year: 'numeric', month: '2-digit', day: '2-digit' },
  ).format(date);
}

/** Format a date and time using the current locale. */
export function formatDateTime(
  iso: string | Date,
  opts?: Intl.DateTimeFormatOptions,
): string {
  const date = typeof iso === 'string' ? new Date(iso) : iso;
  return new Intl.DateTimeFormat(
    resolveLocale(),
    opts ?? { year: 'numeric', month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit' },
  ).format(date);
}

/** Format a number using the current locale. */
export function formatNumber(n: number, opts?: Intl.NumberFormatOptions): string {
  return new Intl.NumberFormat(resolveLocale(), opts).format(n);
}

/** Full month names for the current locale (January..December). */
export function localizedMonthNames(): string[] {
  const fmt = new Intl.DateTimeFormat(resolveLocale(), { month: 'long' });
  return Array.from({ length: 12 }, (_, i) => fmt.format(new Date(2000, i, 1)));
}

/**
 * Seven weekday short labels for the current locale, in Sunday-first order
 * (the calendar's existing grid order is preserved — only the labels are
 * localized).
 */
export function localizedDayHeaders(): string[] {
  const fmt = new Intl.DateTimeFormat(resolveLocale(), { weekday: 'short' });
  // 2000-01-02 is a Sunday; subsequent days advance through the week.
  return Array.from({ length: 7 }, (_, i) => fmt.format(new Date(2000, 0, 2 + i)));
}

export { i18n };
export default i18n;
