//! Catalog and interpolation tests.
//!
//! These exercise the public catalog API, the i18next-compatible
//! interpolation rules and the process-wide locale. Tests that need the
//! private JSON parser stay in the crate's unit-test module so the source file
//! stays under the repository's 500-line limit.

use stratum_i18n::{
    current_locale, extra_keys, interpolate, missing_keys, set_locale, t, t_global, tr, Catalog,
    Locale,
};

#[test]
fn locale_codes_round_trip() {
    for locale in Locale::ALL {
        assert_eq!(Locale::from_code(locale.code()), Some(locale));
    }
    assert_eq!(Locale::from_code("xx"), None);
}

#[test]
fn display_names_are_not_empty() {
    for locale in Locale::ALL {
        assert!(!locale.display_name().is_empty());
    }
}

#[test]
fn catalog_lookup_and_meta_exclusion() {
    let catalog = Catalog::load(Locale::En).expect("embedded catalog parses");
    assert_eq!(catalog.get("sidebar.recent"), Some("Recent"));
    assert_eq!(catalog.get("_meta.app"), Some("Stratum"));
    assert!(catalog.keys().contains("sidebar.recent"));
    assert!(!catalog.keys().iter().any(|key| key.starts_with("_meta.")));
}

#[test]
fn interpolation_replaces_known_values() {
    assert_eq!(
        interpolate(
            "Moved \"{{old}}\" to \"{{new}}\".",
            &[("old", "a"), ("new", "b")]
        ),
        "Moved \"a\" to \"b\"."
    );
    assert_eq!(interpolate("{{a}}{{b}}", &[("a", "1"), ("b", "2")]), "12");
}

#[test]
fn interpolation_accepts_spaced_placeholders() {
    assert_eq!(
        interpolate("{{ minutes }} min", &[("minutes", "5")]),
        "5 min"
    );
}

#[test]
fn interpolation_keeps_unknown_placeholders() {
    assert_eq!(interpolate("x {{missing}} y", &[]), "x {{missing}} y");
    assert_eq!(interpolate("x {{ missing }} y", &[]), "x {{ missing }} y");
}

#[test]
fn interpolation_is_single_pass() {
    // The replacement itself contains a placeholder, which must not be
    // scanned again.
    assert_eq!(interpolate("{{a}}", &[("a", "{{b}}"), ("b", "X")]), "{{b}}");
}

#[test]
fn interpolation_ignores_invalid_names() {
    assert_eq!(
        interpolate("{{bad-name}}", &[("bad-name", "x")]),
        "{{bad-name}}"
    );
    assert_eq!(interpolate("{{}}", &[("", "x")]), "{{}}");
}

#[test]
fn interpolation_handles_unclosed_braces() {
    assert_eq!(
        interpolate("open {{value", &[("value", "x")]),
        "open {{value"
    );
}

#[test]
fn tr_uses_golden_catalog_values() {
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
}

#[test]
fn missing_key_returns_the_key() {
    assert_eq!(t(Locale::En, "no.such.key"), "no.such.key");
    assert_eq!(tr(Locale::En, "no.such.key", &[("a", "b")]), "no.such.key");
}

#[test]
fn global_locale_defaults_to_english_and_round_trips() {
    // Tests share the process-wide atom, so restore English afterwards.
    assert_eq!(current_locale(), Locale::En);
    set_locale(Locale::Fr);
    assert_eq!(current_locale(), Locale::Fr);
    assert_eq!(t_global("sidebar.recent"), t(Locale::Fr, "sidebar.recent"));
    set_locale(Locale::En);
}

#[test]
fn parity_helpers_report_drift() {
    assert!(missing_keys(Locale::En).unwrap().is_empty());
    assert!(extra_keys(Locale::En).unwrap().is_empty());
    for locale in Locale::ALL {
        assert!(
            missing_keys(locale).unwrap().is_empty(),
            "{}",
            locale.code()
        );
        assert!(extra_keys(locale).unwrap().is_empty(), "{}", locale.code());
    }
}
