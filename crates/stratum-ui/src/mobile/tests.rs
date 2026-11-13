//! Mobile shell state tests.
//!
//! These tests exercise the constructors, the bottom-navigation and More-sheet
//! flows, the back stack, the host-services seam and the release catalog gate
//! without opening a window, so they run on the host and on CI without a
//! display server.

use std::sync::Arc;

use stratum_i18n::{t, Locale};

use super::*;
use crate::provider::{FixtureProvider, ShellData};
use crate::router::Panel;
use crate::shell::Shell;
use ::core::prelude::v1::test;

fn mobile_shell() -> Shell {
    Shell::new_mobile(Arc::new(FixtureProvider::new()))
}

/// A host-services implementation with distinguishable answers, used to prove
/// [`Shell::set_mobile_services`] installs the caller's implementation.
struct RecordingServices;

impl MobileServices for RecordingServices {
    fn request_mic_permission(&self) -> Result<bool, String> {
        Ok(true)
    }

    fn start_dictation(&self) -> Result<(), String> {
        Ok(())
    }

    fn stop_dictation(&self) -> Result<String, String> {
        Ok("/tmp/dictation.flac".to_string())
    }

    fn pick_file(&self) -> Result<Option<String>, String> {
        Ok(Some("/tmp/note.md".to_string()))
    }

    fn notify(&self, _title: &str, _body: &str) -> Result<(), String> {
        Ok(())
    }

    fn set_clipboard(&self, _text: &str) -> Result<(), String> {
        Ok(())
    }

    fn open_external(&self, _url: &str) -> Result<(), String> {
        Ok(())
    }
}

#[test]
fn new_mobile_marks_the_shell_mobile() {
    let shell = mobile_shell();
    assert!(shell.mobile());
    assert_eq!(shell.active_panel(), Panel::Journal);
    assert_eq!(shell.route(), "/journal");
    assert!(shell.mobile_nav().stack().is_empty());
    assert!(!shell.mobile_nav().more_open());
    assert!(!shell.mobile_settings_open());
}

#[test]
fn desktop_shell_stays_non_mobile() {
    let shell = Shell::new(Arc::new(FixtureProvider::new()));
    assert!(!shell.mobile());
}

#[test]
fn with_platform_mobile_marks_the_shell_mobile() {
    let provider: Arc<dyn ShellData> = Arc::new(FixtureProvider::new());
    let shell = Shell::with_platform_mobile(provider, crate::platform::PlatformBridge::inert());
    assert!(shell.mobile());
}

#[test]
fn mobile_services_default_to_the_noop_host() {
    let shell = mobile_shell();
    assert_eq!(shell.mobile_services().request_mic_permission(), Ok(false));
    assert_eq!(shell.mobile_services().pick_file(), Ok(None));
}

#[test]
fn set_mobile_services_installs_the_host_implementation() {
    let mut shell = mobile_shell();
    shell.set_mobile_services(Arc::new(RecordingServices));
    assert_eq!(shell.mobile_services().request_mic_permission(), Ok(true));
    assert_eq!(
        shell.mobile_services().pick_file(),
        Ok(Some("/tmp/note.md".to_string()))
    );
}

#[test]
fn primary_tab_pushes_the_previous_panel_and_navigates() {
    let mut shell = mobile_shell();
    let search = tab_for_panel(Panel::Search).expect("search tab");
    shell.mobile_activate_tab(search);
    assert_eq!(shell.active_panel(), Panel::Search);
    assert_eq!(shell.route(), "/search");
    assert_eq!(
        shell.mobile_nav().top().map(|entry| entry.panel),
        Some(Panel::Journal)
    );
}

#[test]
fn activating_the_current_panel_does_not_grow_the_stack() {
    let mut shell = mobile_shell();
    let journal = tab_for_panel(Panel::Journal).expect("journal tab");
    shell.mobile_activate_tab(journal);
    assert_eq!(shell.active_panel(), Panel::Journal);
    assert!(shell.mobile_nav().stack().is_empty());
}

#[test]
fn more_tab_opens_the_sheet_without_navigating() {
    let mut shell = mobile_shell();
    let more = MOBILE_TABS
        .into_iter()
        .find(|tab| tab.target.is_none())
        .expect("more tab");
    shell.mobile_activate_tab(more);
    assert_eq!(shell.active_panel(), Panel::Journal);
    assert!(shell.mobile_nav().more_open());
}

#[test]
fn every_more_entry_navigates_and_closes_the_sheet() {
    let mut shell = mobile_shell();
    for entry in MORE_ENTRIES {
        shell.mobile_open_more();
        shell.mobile_activate_more(entry.panel);
        assert_eq!(shell.active_panel(), entry.panel);
        assert_eq!(shell.route(), crate::path_for(entry.panel, None));
        assert!(!shell.mobile_nav().more_open());
    }
    // Every More row leaves a different panel behind, so each navigation
    // pushes one entry onto the back stack.
    assert_eq!(shell.mobile_nav().stack().len(), MORE_ENTRIES.len());
}

#[test]
fn back_closes_the_more_sheet_before_navigating() {
    let mut shell = mobile_shell();
    shell.mobile_activate_more(Panel::Settings);
    shell.mobile_open_more();
    assert!(shell.mobile_back());
    assert!(!shell.mobile_nav().more_open());
    assert_eq!(shell.active_panel(), Panel::Settings);
}

#[test]
fn back_closes_the_settings_accordion_before_navigating() {
    let mut shell = mobile_shell();
    shell.mobile_activate_more(Panel::Settings);
    shell.mobile_toggle_settings_accordion();
    assert!(shell.mobile_settings_open());
    assert!(shell.mobile_back());
    assert!(!shell.mobile_settings_open());
    assert_eq!(shell.active_panel(), Panel::Settings);
}

#[test]
fn back_pops_the_visited_panel() {
    let mut shell = mobile_shell();
    shell.mobile_activate_more(Panel::Search);
    assert!(shell.mobile_back());
    assert_eq!(shell.active_panel(), Panel::Journal);
    assert!(!shell.mobile_back());
}

#[test]
fn mobile_set_more_toggles_the_sheet() {
    let mut shell = mobile_shell();
    shell.mobile_set_more(true);
    assert!(shell.mobile_nav().more_open());
    shell.mobile_set_more(false);
    assert!(!shell.mobile_nav().more_open());
}

#[test]
fn mobile_i18n_keys_are_complete_in_every_locale() {
    for locale in Locale::ALL {
        for key in MOBILE_I18N_KEYS {
            assert_ne!(t(locale, key), key, "{} missing in {}", key, locale.code());
        }
    }
}

#[test]
fn mobile_i18n_keys_are_sorted_and_unique() {
    let mut keys = MOBILE_I18N_KEYS.to_vec();
    keys.sort_unstable();
    keys.dedup();
    assert_eq!(keys.len(), MOBILE_I18N_KEYS.len());
    assert_eq!(keys, MOBILE_I18N_KEYS.to_vec());
}

#[test]
fn mobile_i18n_keys_cover_every_navigation_label() {
    for tab in MOBILE_TABS {
        assert!(
            MOBILE_I18N_KEYS.contains(&tab.label_key),
            "missing tab label: {}",
            tab.label_key
        );
    }
    for entry in MORE_ENTRIES {
        assert!(
            MOBILE_I18N_KEYS.contains(&entry.label_key),
            "missing sheet label: {}",
            entry.label_key
        );
    }
}
