//! Locale parity gate.
//!
//! These tests fail the build when a locale drifts from the English reference,
//! when an interpolation placeholder uses an unsupported syntax, or when the
//! embedded catalog no longer matches the frozen web application's catalog
//! while that directory still exists.

use std::collections::BTreeSet;
use std::path::PathBuf;

use stratum_i18n::{extra_keys, interpolate, key_set, missing_keys, tr, Catalog, Locale, LOCALES};

/// Every supported locale must expose exactly the English key set.
#[test]
fn all_locales_have_identical_key_sets() {
    let reference = key_set(Locale::En).expect("English catalog loads");
    for locale in Locale::ALL {
        let actual = key_set(locale).expect("catalog loads");
        let missing: BTreeSet<_> = reference.difference(actual).cloned().collect();
        let extra: BTreeSet<_> = actual.difference(reference).cloned().collect();
        assert!(
            missing.is_empty(),
            "{} is missing {} keys: {:?}",
            locale.code(),
            missing.len(),
            missing.iter().take(10).collect::<Vec<_>>()
        );
        assert!(
            extra.is_empty(),
            "{} has {} extra keys: {:?}",
            locale.code(),
            extra.len(),
            extra.iter().take(10).collect::<Vec<_>>()
        );
        assert!(missing_keys(locale).unwrap().is_empty());
        assert!(extra_keys(locale).unwrap().is_empty());
    }
}

/// Validate the placeholder syntax of every user-facing catalog value.
#[test]
fn placeholders_use_only_supported_syntax() {
    for locale in Locale::ALL {
        let catalog = Catalog::load(locale).expect("catalog loads");
        for key in catalog.keys() {
            let value = catalog.get(key).expect("key resolves");
            assert!(
                !value.contains("%{"),
                "{}:{} uses percent-style interpolation",
                locale.code(),
                key
            );
            assert!(
                !value.contains("${"),
                "{}:{} uses template-string interpolation",
                locale.code(),
                key
            );
            validate_braces(locale.code(), key, value);
        }
    }
}

/// Scan `{{...}}` runs and reject anything outside `[A-Za-z0-9_ ]`.
fn validate_braces(code: &str, key: &str, value: &str) {
    let bytes = value.as_bytes();
    let mut cursor = 0;
    while cursor + 1 < bytes.len() {
        if bytes[cursor] == b'{' && bytes[cursor + 1] == b'{' {
            let rest = &value[cursor + 2..];
            let end = rest
                .find("}}")
                .unwrap_or_else(|| panic!("{code}:{key} has an unclosed placeholder in {value:?}"));
            let inner = &rest[..end];
            assert!(
                !inner.is_empty(),
                "{code}:{key} has an empty placeholder in {value:?}"
            );
            assert!(
                inner
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b' '),
                "{code}:{key} has an invalid placeholder {{{{{inner}}}}} in {value:?}"
            );
            cursor += 2 + end + 2;
        } else {
            cursor += 1;
        }
    }
}

/// Golden interpolation results taken from the shipped catalogs.
#[test]
fn interpolation_goldens() {
    assert_eq!(
        tr(
            Locale::En,
            "sidebar.move_page_success",
            &[("old", "a"), ("new", "b")]
        ),
        "Moved \"a\" to \"b\"."
    );
    assert_eq!(
        tr(
            Locale::En,
            "reminders.humanize.due_in_min",
            &[("minutes", "5")]
        ),
        "Due in 5 min"
    );
    assert_eq!(interpolate("x {{missing}} y", &[]), "x {{missing}} y");
}

/// The embedded catalogs must remain byte-identical to the web application's
/// catalogs while that directory is still present. The assertion is skipped
/// when the frozen web app has been deleted at cutover.
#[test]
fn embedded_catalogs_match_the_web_application() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let web_dir = manifest_dir.join("../../src/i18n/locales");
    if !web_dir.is_dir() {
        eprintln!(
            "skipping web-catalog drift check: {} is absent",
            web_dir.display()
        );
        return;
    }
    for code in LOCALES {
        let embedded = std::fs::read(manifest_dir.join("locales").join(format!("{code}.json")))
            .expect("embedded catalog is readable");
        let reference =
            std::fs::read(web_dir.join(format!("{code}.json"))).expect("web catalog is readable");
        assert_eq!(
            embedded, reference,
            "embedded {code}.json drifted from src/i18n/locales/{code}.json"
        );
    }
}
