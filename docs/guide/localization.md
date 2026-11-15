# Localization

Stratum ships a fully translatable interface. All user-facing strings are routed
through `stratum-i18n` (`t_global(...)` / `tr_global(...)`), and the active UI
language is a persisted setting — no restart required.

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
the formatting helpers in `stratum-i18n` (date/number formatting,
`localizedMonthNames`, `localizedDayHeaders`). The **Journal** calendar and any
number/date rendering use these helpers automatically.

## Developer Workflow

### Adding a new UI string

Never hardcode user-facing copy. Use the catalog:

```rust
use stratum_i18n::t_global;

let label = t_global("common.save");
```

Add the key (and all translations) to **all eight** catalogs under
`crates/stratum-i18n/locales/` under the appropriate section, using the
`{{var}}` interpolation style.

### The raw-key gate

`crates/stratum-i18n/tests/raw_keys.rs` walks `crates/stratum-ui/src`, extracts
every literal key passed to `t_global`/`tr_global` and **fails `cargo test`**
when a key is absent from the English catalog (and therefore from every catalog,
which the parity gate keeps in lock-step). A missing key would otherwise render
as the raw key string.

Run it in isolation:

```bash
cargo test -p stratum-i18n --test raw_keys
```

### Catalog parity gate

`stratum-i18n`'s parity helpers (`key_set`, `missing_keys`, `extra_keys`) and the
panel contract tests enforce the release contract:

- all 8 catalogs expose the **identical flattened key set**;
- every key exists in `en` (extra keys in any locale fail);
- placeholder locales (`zh`/`ja`/`el`) mirror `en` values — no missing-key path;
- full locales (`de`/`it`/`fr`/`es`) contain **no untranslated English** except
  a reviewed allowlist of brands, endonyms, and technical loanwords.

Every panel also declares a `*_I18N_KEYS` gate list that
`crates/stratum-ui/tests/*_contract.rs` verifies against all eight catalogs.

When adding a locale key, add it to **all 8 catalogs** in the same commit. When
translating a full locale, never leave an English value behind.

### Runtime fallback behavior

The runtime fallback chain is `active locale → en`. There is **no** raw-key
rendering: if a key is missing from the active locale but present in `en`, the
`en` value is used. Only a key missing from `en` itself would surface as the key
string — the raw-key gate prevents that.
