# Localization

Stratum ships a fully translatable interface. All user-facing strings are routed
through the `i18next` layer (`t(...)`), and the active UI language is a persisted
setting — no restart required.

<!-- SCREENSHOT: [settings-locale] Language selector in Settings → Appearance -->

## Supported Languages

| Code  | Language   | Status                 |
|-------|------------|------------------------|
| `en`  | English    | Source of truth        |
| `de`  | German     | Full translation       |
| `it`  | Italian    | Full translation       |
| `fr`  | French     | Full translation       |
| `es`  | Spanish    | Full translation       |
| `zh`  | Chinese    | Placeholder (en values)|
| `ja`  | Japanese   | Placeholder (en values)|
| `el`  | Greek      | Placeholder (en values)|

`en` is always available as the **fallback**: a missing key never renders the raw
key or crashes the app. `zh`, `ja`, and `el` currently mirror English values
(marked `in progress` in `_meta.status`) — they render correctly and only need
translation work.

## Changing the Language

1. Open **Settings → Appearance**
2. Select a language from the **Language** dropdown
3. The interface updates immediately — no restart required

The choice is persisted to `.pkm/config.toml` under the `locale` key
(ISO 639-1 code, default `en`) and is read at startup. `locale` serializes at
the top level of the config file (no section wrapper):

```toml
# .pkm/config.toml — top-level key, next to vault_path
locale = "de"
```

## Locale-Aware Formatting

Dates, numbers, and calendar labels follow the active locale's conventions via
the `Intl` helpers in `src/i18n/` (`formatDate`, `formatNumber`,
`localizedMonthNames`, `localizedDayHeaders`). The **Journal** calendar and any
number/date rendering use these helpers automatically.

## Developer Workflow

### Adding a new UI string

Never hardcode user-facing copy. Use `t(...)`:

```tsx
import { useTranslation } from 'react-i18next';
const { t } = useTranslation();
return <Button>{t('common.save')}</Button>;
```

Add the key (and all translations) to `src/i18n/locales/<lang>.json` under the
appropriate section.

### The i18n gate

`src/i18n/i18nGate.test.ts` is a static AST scan that **fails `npm run test`**
if a user-facing English string is hardcoded as JSX text or a
`label/placeholder/title/aria-label` literal outside `t()`. It walks `src/`
(excluding tests, workers, and `i18n/` itself) and reports each violation as
`path:line — 'literal'`.

Run it in isolation:

```bash
npx vitest run src/i18n/i18nGate.test.ts
```

The gate's allowlist covers non-UI literals (CSS, units, URLs, enum tokens,
imports, SVG paths, LaTeX). It is intentionally **conservative**: when the scan
flags something that IS user-facing, translate it — do not extend the allowlist.

### Locale parity gate

`src/i18n/localeParity.test.ts` enforces the release contract:

- all 8 locale files expose the **identical flattened key set**;
- every key exists in `en` (extra keys in any locale fail);
- placeholder locales (`zh`/`ja`/`el`) mirror `en` values — no missing-key path;
- full locales (`de`/`it`/`fr`/`es`) contain **no untranslated English** except
  a reviewed allowlist of brands, endonyms, and technical loanwords.

When adding a locale key, add it to **all 8 files** in the same commit. When
translating a full locale, never leave an English value behind.

### Runtime fallback behavior

The runtime i18next fallback chain is `active locale → en`. There is **no**
raw-key rendering: if a key is missing from the active locale but present in
`en`, the `en` value is used. Only a key missing from `en` itself would surface
as the key string — the parity gate prevents that.
