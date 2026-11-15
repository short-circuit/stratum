//! Platform-bridge contract tests.
//!
//! The bridge maps the settings DTO onto the tray/shortcut/notification
//! preferences, localizes the tray labels and reports capabilities without
//! touching an OS handle in the inert case. These checks live in an integration
//! test because the lib-test build of this crate cannot expand another
//! `#[test]` in the shell module without exceeding the default recursion limit.

use stratum_ui::platform::{
    instance_name, notification_open_label, preferences_from, tray_labels, PlatformBridge,
};
use stratum_ui::provider::SettingsDto;
use stratum_ui::provider::{FixtureProvider, SettingsData};
use stratum_ui::shell::settings_fingerprint;

/// The fixture's settings, which carry the shipped defaults.
fn baseline() -> SettingsDto {
    FixtureProvider::new()
        .settings()
        .expect("the fixture provider answers settings")
}

#[test]
fn preferences_follow_the_settings() {
    let mut settings = baseline();
    settings.tray.enabled = false;
    settings.tray.close_to_tray = true;
    settings.tray.quick_capture_shortcut = "Ctrl+Alt+K".into();
    settings.reminders.enabled = false;
    settings.reminders.sound = true;

    let prefs = preferences_from(&settings);
    assert!(!prefs.tray_enabled);
    assert!(prefs.close_to_tray);
    assert_eq!(prefs.quick_capture_shortcut, "Ctrl+Alt+K");
    assert!(!prefs.notifications_enabled);
    assert!(prefs.notification_sound);
}

#[test]
fn preferences_mirror_the_default_settings() {
    let settings = baseline();
    let prefs = preferences_from(&settings);
    assert_eq!(prefs.tray_enabled, settings.tray.enabled);
    assert_eq!(prefs.close_to_tray, settings.tray.close_to_tray);
    assert_eq!(
        prefs.quick_capture_shortcut,
        settings.tray.quick_capture_shortcut
    );
    assert_eq!(prefs.notifications_enabled, settings.reminders.enabled);
    assert_eq!(prefs.notification_sound, settings.reminders.sound);
}

#[test]
fn inert_bridge_reports_no_capabilities() {
    let mut bridge = PlatformBridge::inert();
    assert!(bridge.poll().is_empty());
    assert!(!bridge.close_to_tray());
    assert!(!bridge.tray_present());
    assert!(bridge.shortcut().is_none());
    assert!(bridge.shortcut_error().is_none());

    bridge.apply_settings(&baseline());
    assert!(bridge.poll().is_empty());
}

#[test]
fn tray_labels_and_notification_action_are_localized() {
    let labels = tray_labels();
    assert_eq!(labels.open, stratum_i18n::t_global("tray.open"));
    assert_eq!(
        labels.quick_capture,
        stratum_i18n::t_global("quickcapture.title")
    );
    assert_eq!(
        labels.voice_capture,
        stratum_i18n::t_global("quickcapture.voice")
    );
    assert_eq!(labels.sync_now, stratum_i18n::t_global("tray.sync_now"));
    assert_eq!(labels.quit, stratum_i18n::t_global("tray.quit"));
    assert_eq!(
        notification_open_label(),
        stratum_i18n::t_global("reminders.toast.open")
    );
}

#[test]
fn tray_labels_exist_in_every_catalog() {
    // The release gate walks the same table; this catches a missing `tray`
    // namespace before the tray renders a raw key.
    for locale in stratum_i18n::Locale::ALL {
        for key in ["tray.open", "tray.sync_now", "tray.quit"] {
            let value = stratum_i18n::t(locale, key);
            assert_ne!(value, key, "{key} missing from {locale:?}");
            assert!(!value.is_empty(), "{key} empty in {locale:?}");
        }
    }
}

#[test]
fn instance_name_is_versioned() {
    let name = instance_name();
    assert!(name.starts_with("stratum-"));
    assert!(name.len() > "stratum-".len());
}

#[test]
fn fingerprint_tracks_every_platform_field() {
    let before = settings_fingerprint(&baseline());

    let mut settings = baseline();
    settings.tray.enabled = !settings.tray.enabled;
    assert_ne!(settings_fingerprint(&settings), before);

    let mut settings = baseline();
    settings.tray.close_to_tray = !settings.tray.close_to_tray;
    assert_ne!(settings_fingerprint(&settings), before);

    let mut settings = baseline();
    settings.tray.quick_capture_shortcut = "Ctrl+Alt+K".into();
    assert_ne!(settings_fingerprint(&settings), before);

    let mut settings = baseline();
    settings.reminders.enabled = !settings.reminders.enabled;
    assert_ne!(settings_fingerprint(&settings), before);

    let mut settings = baseline();
    settings.reminders.sound = !settings.reminders.sound;
    assert_ne!(settings_fingerprint(&settings), before);
}

#[test]
fn fingerprint_is_stable_for_unchanged_settings() {
    assert_eq!(
        settings_fingerprint(&baseline()),
        settings_fingerprint(&baseline())
    );
}
