//! Integration checks for the shell's parity contracts.
//!
//! These tests deliberately use only the public API so they catch accidental
//! visibility or API regressions that unit tests inside the crate would miss.

use std::sync::Arc;

use gpui_kit::AssetSource;
use stratum_i18n::{t, Locale};
use stratum_ui::provider::{provider_from_env, ShellData};
use stratum_ui::router::{NAV_ITEMS, ROUTES};
use stratum_ui::shell::{SHELL_COLLAPSED_WIDTH, SHELL_I18N_KEYS, SHELL_WIDTH};
use stratum_ui::theme::{derive_palette, hex_to_hsl, ThemeConfig};
use stratum_ui::{parse_path, path_for, Panel, StratumAssets};

#[test]
fn shell_widths_match_the_shipped_sidebar() {
    assert_eq!(SHELL_WIDTH, 224.0);
    assert_eq!(SHELL_COLLAPSED_WIDTH, 52.0);
}

#[test]
fn thirteen_routes_and_twelve_nav_rows() {
    assert_eq!(ROUTES.len(), 13);
    assert_eq!(NAV_ITEMS.len(), 12);
    for panel in Panel::ALL {
        assert_eq!(parse_path(&path_for(panel, None)), Some(panel));
    }
}

#[test]
fn home_redirects_to_the_journal() {
    assert_eq!(parse_path("/"), Some(Panel::Journal));
    assert_eq!(parse_path("/page"), Some(Panel::Page));
}

#[test]
fn theme_defaults_match_the_shipped_configuration() {
    let config = ThemeConfig::default();
    assert_eq!(config.primary_hex, "#f0883e");
    assert_eq!(config.secondary_hex, "#3a2418");
    assert!(config.dark);
    let palette = derive_palette(&config.primary_hex, &config.secondary_hex, config.dark);
    assert_eq!(palette.background_default, "hsl(21, 25%, 7%)");
    assert_eq!(hex_to_hsl("#f0883e").h, 25);
}

#[test]
fn fixture_provider_is_journal_free() {
    let provider = provider_from_env(std::env::temp_dir(), true);
    let snapshot = provider.snapshot();
    assert!(snapshot.vault.is_some());
    assert!(!snapshot.recents.is_empty());
    assert!(snapshot
        .recents
        .iter()
        .all(|page| !page.path.starts_with("journals/")));
    assert_eq!(snapshot, provider.refresh());
}

#[test]
fn fixture_export_reports_pages() {
    let provider: Arc<dyn ShellData> = provider_from_env(std::env::temp_dir(), true);
    assert_eq!(provider.export_html(), Ok(87));
}

#[test]
fn shell_i18n_keys_are_complete_in_every_locale() {
    for locale in Locale::ALL {
        for key in SHELL_I18N_KEYS {
            assert_ne!(t(locale, key), key, "{} missing in {}", key, locale.code());
        }
    }
}

#[test]
fn embedded_shell_assets_are_loadable() {
    let assets = StratumAssets::new();
    assert!(assets
        .load("icons/calendar.svg")
        .expect("asset load")
        .is_some());
    assert!(assets
        .load("fonts/Inter-Regular.ttf")
        .expect("asset load")
        .is_some());
}
