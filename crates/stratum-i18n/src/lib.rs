//! Embedded locale catalogs for the Stratum user interface.
//!
//! The eight catalogs under `locales/` are byte-identical copies of the
//! shipped web application's `src/i18n/locales/*.json` files. They are
//! embedded into the binary with [`include_str!`], flattened into dotted
//! lookup keys (`sidebar.recent`) and cached process-wide with [`OnceLock`].
//!
//! The module provides i18next-compatible `{{name}}` interpolation and the
//! parity helpers used by the release gate: [`key_set`], [`missing_keys`] and
//! [`extra_keys`]. A missing translation deliberately resolves to the raw key
//! so an unfinished catalog is visible in the UI instead of silently falling
//! back to English.
//!
//! ```
//! use stratum_i18n::{t, tr, Locale};
//!
//! assert_eq!(t(Locale::En, "sidebar.recent"), "Recent");
//! assert_eq!(
//!     tr(Locale::En, "sidebar.move_page_success", &[("old", "a"), ("new", "b")]),
//!     "Moved \"a\" to \"b\"."
//! );
//! ```

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::OnceLock;

use pkm_core::PkmError;
use serde_json::Value;

/// Locale codes in the order used by [`Locale::ALL`].
pub const LOCALES: [&str; 8] = ["en", "de", "el", "es", "fr", "it", "ja", "zh"];

/// A supported user-interface locale.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Locale {
    /// English (the reference catalog).
    En,
    /// German.
    De,
    /// Greek.
    El,
    /// Spanish.
    Es,
    /// French.
    Fr,
    /// Italian.
    It,
    /// Japanese.
    Ja,
    /// Chinese.
    Zh,
}

impl Locale {
    /// Every supported locale, in [`LOCALES`] order.
    pub const ALL: [Locale; 8] = [
        Locale::En,
        Locale::De,
        Locale::El,
        Locale::Es,
        Locale::Fr,
        Locale::It,
        Locale::Ja,
        Locale::Zh,
    ];

    /// The stable locale code (`en`, `de`, …).
    pub fn code(self) -> &'static str {
        match self {
            Locale::En => "en",
            Locale::De => "de",
            Locale::El => "el",
            Locale::Es => "es",
            Locale::Fr => "fr",
            Locale::It => "it",
            Locale::Ja => "ja",
            Locale::Zh => "zh",
        }
    }

    /// Resolve a locale code, returning `None` for unsupported codes.
    pub fn from_code(code: &str) -> Option<Locale> {
        match code {
            "en" => Some(Locale::En),
            "de" => Some(Locale::De),
            "el" => Some(Locale::El),
            "es" => Some(Locale::Es),
            "fr" => Some(Locale::Fr),
            "it" => Some(Locale::It),
            "ja" => Some(Locale::Ja),
            "zh" => Some(Locale::Zh),
            _ => None,
        }
    }

    /// The English name of the language, as shown in the locale picker.
    pub fn display_name(self) -> &'static str {
        match self {
            Locale::En => "English",
            Locale::De => "Deutsch",
            Locale::El => "Ελληνικά",
            Locale::Es => "Español",
            Locale::Fr => "Français",
            Locale::It => "Italiano",
            Locale::Ja => "日本語",
            Locale::Zh => "中文",
        }
    }

    /// Internal dense index used by the process-wide active-locale atom.
    fn index(self) -> usize {
        match self {
            Locale::En => 0,
            Locale::De => 1,
            Locale::El => 2,
            Locale::Es => 3,
            Locale::Fr => 4,
            Locale::It => 5,
            Locale::Ja => 6,
            Locale::Zh => 7,
        }
    }

    /// Reverse of [`Self::index`].
    fn from_index(index: usize) -> Locale {
        Locale::ALL.get(index).copied().unwrap_or(Locale::En)
    }
}

/// A flattened locale catalog.
///
/// The map keeps the top-level `_meta` object under dotted keys, but those
/// entries are deliberately omitted from [`Catalog::keys`] so parity checks
/// compare only user-facing translation keys.
pub struct Catalog {
    entries: BTreeMap<String, String>,
    keys: BTreeSet<String>,
}

impl Catalog {
    /// Load and cache the embedded catalog for `locale`.
    ///
    /// Parsing happens once per process. An invalid embedded catalog is a
    /// build-time defect, surfaced as [`PkmError::I18n`].
    pub fn load(locale: Locale) -> Result<&'static Catalog, PkmError> {
        static CATALOGS: OnceLock<Vec<Catalog>> = OnceLock::new();
        if let Some(catalogs) = CATALOGS.get() {
            return Ok(&catalogs[locale.index()]);
        }
        let catalogs = build_catalogs()?;
        // A concurrent caller may win the race; either way the stored value is
        // identical, so the losing copy is simply dropped.
        let _ = CATALOGS.set(catalogs);
        let catalogs = CATALOGS
            .get()
            .ok_or_else(|| PkmError::I18n("catalog cache failed to initialize".into()))?;
        Ok(&catalogs[locale.index()])
    }

    /// Look up a flattened key. Returns `None` when the key is absent.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries.get(key).map(String::as_str)
    }

    /// The user-facing translation keys in this catalog (`_meta` excluded).
    pub fn keys(&self) -> &BTreeSet<String> {
        &self.keys
    }
}

/// Embedded catalog sources, indexed in [`Locale::ALL`] order.
fn sources() -> [&'static str; 8] {
    [
        include_str!("../locales/en.json"),
        include_str!("../locales/de.json"),
        include_str!("../locales/el.json"),
        include_str!("../locales/es.json"),
        include_str!("../locales/fr.json"),
        include_str!("../locales/it.json"),
        include_str!("../locales/ja.json"),
        include_str!("../locales/zh.json"),
    ]
}

/// Parse every embedded catalog. The `_meta` object is kept in the entry map
/// but excluded from the parity key set.
fn build_catalogs() -> Result<Vec<Catalog>, PkmError> {
    sources()
        .iter()
        .map(|source| Catalog::parse(source))
        .collect()
}

impl Catalog {
    /// Parse one JSON catalog into flattened entries.
    fn parse(source: &str) -> Result<Catalog, PkmError> {
        let value: Value = serde_json::from_str(source)
            .map_err(|e| PkmError::I18n(format!("invalid embedded catalog: {e}")))?;
        let mut entries = BTreeMap::new();
        let mut keys = BTreeSet::new();
        flatten(&value, "", &mut entries, &mut keys, true);
        Ok(Catalog { entries, keys })
    }
}

/// Flatten a JSON value into dotted keys.
///
/// Objects recurse; strings, numbers, booleans, nulls and arrays are leaves.
/// Arrays are stored as compact JSON because i18next arrays are used as a
/// single value rather than as independent keys. `_meta` is passed with
/// `include_keys = false` so it remains addressable but stays out of parity.
fn flatten(
    value: &Value,
    prefix: &str,
    entries: &mut BTreeMap<String, String>,
    keys: &mut BTreeSet<String>,
    include_keys: bool,
) {
    match value {
        Value::Object(map) => {
            for (name, child) in map {
                let key = if prefix.is_empty() {
                    name.clone()
                } else {
                    format!("{prefix}.{name}")
                };
                let child_keys = include_keys && !(prefix.is_empty() && name == "_meta");
                flatten(child, &key, entries, keys, child_keys);
            }
        }
        Value::Array(_) => {
            entries.insert(prefix.to_string(), value.to_string());
            if include_keys {
                keys.insert(prefix.to_string());
            }
        }
        Value::String(text) => {
            entries.insert(prefix.to_string(), text.clone());
            if include_keys {
                keys.insert(prefix.to_string());
            }
        }
        Value::Number(number) => {
            entries.insert(prefix.to_string(), number.to_string());
            if include_keys {
                keys.insert(prefix.to_string());
            }
        }
        Value::Bool(flag) => {
            entries.insert(prefix.to_string(), flag.to_string());
            if include_keys {
                keys.insert(prefix.to_string());
            }
        }
        Value::Null => {
            entries.insert(prefix.to_string(), "null".to_string());
            if include_keys {
                keys.insert(prefix.to_string());
            }
        }
    }
}

/// i18next-compatible interpolation.
///
/// Replaces every `{{name}}` where `name` is a non-empty sequence of
/// `[A-Za-z0-9_]` characters; surrounding spaces are trimmed, so
/// `{{ name }}` is accepted. Unknown placeholders stay verbatim, matching
/// i18next when a value is missing. Replacement is single-pass: a `{{...}}`
/// that appears inside a substituted value is not scanned again.
pub fn interpolate(template: &str, vars: &[(&str, &str)]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut cursor = 0;
    while cursor < template.len() {
        let rest = &template[cursor..];
        if let Some(stripped) = rest.strip_prefix("{{") {
            if let Some(end) = stripped.find("}}") {
                let inner = &stripped[..end];
                let name = inner.trim();
                if is_valid_placeholder(name) {
                    if let Some((_, value)) = vars.iter().find(|(key, _)| *key == name) {
                        out.push_str(value);
                        cursor += 2 + end + 2;
                        continue;
                    }
                }
                // Keep the placeholder byte-for-byte, including its spacing.
                out.push_str(&rest[..2 + end + 2]);
                cursor += 2 + end + 2;
                continue;
            }
        }
        // Advance exactly one UTF-8 scalar so slicing stays on a boundary.
        let ch = rest.chars().next().expect("cursor is inside the string");
        out.push(ch);
        cursor += ch.len_utf8();
    }
    out
}

/// Whether `name` is a valid interpolation placeholder name.
fn is_valid_placeholder(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

/// Resolve `key` in `locale`, returning the key itself when missing.
pub fn t(locale: Locale, key: &str) -> String {
    match Catalog::load(locale) {
        Ok(catalog) => catalog.get(key).unwrap_or(key).to_string(),
        Err(_) => key.to_string(),
    }
}

/// Resolve and interpolate `key` in `locale`.
///
/// Missing keys are returned verbatim (without interpolation) so the raw key
/// is visible in the UI.
pub fn tr(locale: Locale, key: &str, vars: &[(&str, &str)]) -> String {
    match Catalog::load(locale) {
        Ok(catalog) => match catalog.get(key) {
            Some(template) => interpolate(template, vars),
            None => key.to_string(),
        },
        Err(_) => key.to_string(),
    }
}

/// Process-wide active locale for the UI. Defaults to English.
static ACTIVE_LOCALE: AtomicU8 = AtomicU8::new(0);

/// Set the process-wide active locale.
pub fn set_locale(locale: Locale) {
    ACTIVE_LOCALE.store(locale.index() as u8, Ordering::SeqCst);
}

/// The process-wide active locale.
pub fn current_locale() -> Locale {
    Locale::from_index(ACTIVE_LOCALE.load(Ordering::SeqCst) as usize)
}

/// Resolve `key` in the process-wide active locale.
pub fn t_global(key: &str) -> String {
    t(current_locale(), key)
}

/// Resolve and interpolate `key` in the process-wide active locale.
pub fn tr_global(key: &str, vars: &[(&str, &str)]) -> String {
    tr(current_locale(), key, vars)
}

/// The flattened user-facing key set for `locale` (`_meta` excluded).
pub fn key_set(locale: Locale) -> Result<&'static BTreeSet<String>, PkmError> {
    Ok(Catalog::load(locale)?.keys())
}

/// Keys present in English but missing from `locale`, sorted.
pub fn missing_keys(locale: Locale) -> Result<Vec<String>, PkmError> {
    let reference = key_set(Locale::En)?;
    let target = key_set(locale)?;
    Ok(reference.difference(target).cloned().collect())
}

/// Keys present in `locale` but absent from English, sorted.
pub fn extra_keys(locale: Locale) -> Result<Vec<String>, PkmError> {
    let reference = key_set(Locale::En)?;
    let target = key_set(locale)?;
    Ok(target.difference(reference).cloned().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    // These two tests use the private JSON parser, so they stay in the unit
    // module; the rest of the catalog contract lives in tests/catalog.rs.

    #[test]
    fn nested_keys_are_flattened() {
        let catalog = Catalog::parse(r#"{"a":{"b":{"c":"value"}}}"#).unwrap();
        assert_eq!(catalog.get("a.b.c"), Some("value"));
        assert!(catalog.keys().contains("a.b.c"));
    }

    #[test]
    fn arrays_are_leaves() {
        let catalog = Catalog::parse(r#"{"a":["x","y"]}"#).unwrap();
        assert_eq!(catalog.get("a"), Some("[\"x\",\"y\"]"));
        assert!(catalog.keys().contains("a"));
    }
}
