//! Shell state tests.
//!
//! These tests exercise navigation, geometry and the export/error flows
//! without opening a window, so they run on the host and on CI without a
//! display server.

use std::sync::Arc;

use stratum_i18n::{t, t_global, Locale};

use super::*;
use crate::provider::{
    journal_path, FixtureProvider, PanelData, QuickCaptureResult, ReminderRow, SettingsData,
    ShellData, ShellSnapshot, FIXTURE_TODAY,
};
// `gpui_kit::*` (re-exported through `super`) brings gpui's `test` attribute
// macro into scope, shadowing the built-in attribute. Import the built-in
// explicitly so these unit tests run without a GPUI app context.
use crate::router::{Panel, ShortcutAction, NAV_ITEMS};
use ::core::prelude::v1::test;

fn fixture_shell() -> Shell {
    Shell::new(Arc::new(FixtureProvider::new()))
}

/// A provider whose export always fails, used to test the error bar.
struct FailingProvider;

impl ShellData for FailingProvider {
    fn snapshot(&self) -> ShellSnapshot {
        ShellSnapshot {
            vault: None,
            recents: Vec::new(),
        }
    }

    fn refresh(&self) -> ShellSnapshot {
        self.snapshot()
    }

    fn export_html(&self) -> Result<u64, String> {
        Err("disk full".to_string())
    }
}

impl PanelData for FailingProvider {}

impl SettingsData for FailingProvider {}

#[test]
fn shell_starts_on_the_journal_route() {
    let shell = fixture_shell();
    assert_eq!(shell.active_panel(), Panel::Journal);
    assert_eq!(shell.route(), "/journal");
    assert!(!shell.sidebar_collapsed());
    assert!(!shell.read_mode());
    assert_eq!(shell.error(), None);
    assert_eq!(shell.toast_message(), None);
}

#[test]
fn navigation_updates_the_route_and_clears_the_page() {
    let mut shell = fixture_shell();
    shell.open_page("pages/Welcome.md");
    assert_eq!(shell.route(), "/page/pages%2FWelcome.md");
    shell.navigate(Panel::Graph);
    assert_eq!(shell.active_panel(), Panel::Graph);
    assert_eq!(shell.current_page(), None);
    assert_eq!(shell.route(), "/graph");
}

#[test]
fn every_nav_item_is_reachable() {
    let mut shell = fixture_shell();
    for item in NAV_ITEMS {
        shell.navigate(item.panel);
        assert_eq!(shell.active_panel(), item.panel);
        assert_eq!(shell.route(), item.path);
    }
}

#[test]
fn sidebar_toggle_switches_between_the_shipped_widths() {
    let mut shell = fixture_shell();
    assert_eq!(shell.sidebar_width(), 224.0);
    assert!(shell.toggle_sidebar());
    assert_eq!(shell.sidebar_width(), 52.0);
    assert!(!shell.toggle_sidebar());
    assert_eq!(shell.sidebar_width(), 224.0);
}

#[test]
fn read_mode_toggles() {
    let mut shell = fixture_shell();
    assert!(shell.toggle_read_mode());
    assert!(shell.read_mode());
    assert!(!shell.toggle_read_mode());
    assert!(!shell.read_mode());
}

#[test]
fn dismiss_overlay_clears_the_error_before_the_toast() {
    let mut shell = fixture_shell();
    shell.show_error("boom");
    shell.push_toast("saved");
    assert!(shell.dismiss_overlay());
    assert_eq!(shell.error(), None);
    assert_eq!(shell.toast_message(), Some("saved"));
    assert!(shell.dismiss_overlay());
    assert_eq!(shell.toast_message(), None);
    assert!(!shell.dismiss_overlay());
}

#[test]
fn export_success_pushes_the_localized_toast() {
    let mut shell = fixture_shell();
    assert!(shell.export());
    let message = shell.toast_message().expect("toast is set");
    assert!(message.contains("87"));
    assert!(message.contains("/tmp/stratum-export"));
    assert_eq!(shell.error(), None);
}

#[test]
fn export_failure_shows_the_localized_error_bar() {
    let mut shell = Shell::new(Arc::new(FailingProvider));
    assert!(!shell.export());
    let error = shell.error().expect("error is set");
    assert!(error.starts_with(&t(Locale::En, "sidebar.export_failed")));
    assert!(error.contains("disk full"));
    assert_eq!(shell.toast_message(), None);
}

#[test]
fn new_page_button_uses_the_localized_action_name() {
    let mut shell = fixture_shell();
    shell.request_new_page();
    assert_eq!(
        shell.toast_message(),
        Some(t(Locale::En, "sidebar.new_page").as_str())
    );
}

#[test]
fn shortcuts_change_shell_state() {
    let mut shell = fixture_shell();
    assert!(shell.apply_shortcut(ShortcutAction::ToggleSidebar));
    assert!(shell.sidebar_collapsed());
    assert!(shell.apply_shortcut(ShortcutAction::ToggleReadMode));
    assert!(shell.read_mode());
    shell.show_error("boom");
    assert!(shell.apply_shortcut(ShortcutAction::DismissOverlay));
    assert_eq!(shell.error(), None);
}

#[test]
fn create_intent_markdown_path_opens_the_page() {
    let mut shell = fixture_shell();
    shell.handle_editor_navigation("pages/New.md");
    assert_eq!(shell.current_page(), Some("pages/New.md"));
    assert_eq!(shell.active_panel(), Panel::Page);
}

#[test]
fn unresolved_bare_slug_falls_back_to_search() {
    let mut shell = fixture_shell();
    shell.handle_editor_navigation("Missing");
    assert_eq!(shell.active_panel(), Panel::Search);
    assert_eq!(shell.current_page(), None);
}

#[test]
fn refresh_reads_the_provider_snapshot() {
    let mut shell = fixture_shell();
    let before = shell.snapshot().clone();
    shell.refresh();
    assert_eq!(shell.snapshot(), &before);
}

#[test]
fn shell_keys_are_present_in_every_catalog() {
    for locale in Locale::ALL {
        for key in SHELL_I18N_KEYS {
            assert_ne!(t(locale, key), key, "{} missing in {}", key, locale.code());
        }
    }
}

#[test]
fn every_visible_panel_and_nav_label_has_a_catalog_key() {
    for panel in Panel::ALL {
        assert!(
            SHELL_I18N_KEYS.contains(&panel.title_key()),
            "{} title key {}",
            panel.id(),
            panel.title_key()
        );
    }
    for item in NAV_ITEMS {
        assert!(
            SHELL_I18N_KEYS.contains(&item.label_key),
            "{} label key {}",
            item.id,
            item.label_key
        );
    }
}

/// A fixture reminder row used by the toaster tests.
fn due_reminder() -> ReminderRow {
    ReminderRow {
        block_id: "reminder-1".to_string(),
        page_path: "pages/fixture.md".to_string(),
        page_title: Some("Fixture Note".to_string()),
        content: "Review the fixture outline".to_string(),
        due: "2026-11-10T09:00:00Z".to_string(),
    }
}

#[test]
fn reminder_toast_announces_once_and_rearms_after_snooze() {
    let mut shell = fixture_shell();
    let reminder = due_reminder();
    let toast = shell
        .next_due_toast(std::slice::from_ref(&reminder))
        .expect("the first due reminder toasts");
    assert!(toast
        .message
        .contains(&t(Locale::En, "reminders.toast.due_title")));
    assert!(toast.message.contains("Review the fixture outline"));
    assert_eq!(
        toast.action_label.as_deref(),
        Some(t(Locale::En, "reminders.toast.open").as_str())
    );
    assert_eq!(
        toast.action,
        Some(ToastAction::ReminderOpen {
            page_path: "pages/fixture.md".to_string(),
            block_id: "reminder-1".to_string(),
        })
    );

    assert_eq!(
        shell.next_due_toast(std::slice::from_ref(&reminder)),
        None,
        "the same reminder must not toast twice"
    );

    // Snoozing/clearing drops the reminder from the due set, which prunes the
    // remembered id so the next due window announces it again.
    assert_eq!(shell.next_due_toast(&[]), None);
    assert!(shell
        .next_due_toast(std::slice::from_ref(&reminder))
        .is_some());
}

#[test]
fn toast_action_opens_the_reminder_page_and_dismisses_the_toast() {
    let mut shell = fixture_shell();
    shell.toaster.toast = Some(Toast::with_action(
        "Reminder due",
        "Open",
        ToastAction::ReminderOpen {
            page_path: "pages/fixture.md".to_string(),
            block_id: "reminder-1".to_string(),
        },
    ));
    assert!(shell.activate_toast_action());
    assert_eq!(shell.current_page(), Some("pages/fixture.md"));
    assert_eq!(shell.active_panel(), Panel::Page);
    assert_eq!(shell.toast_message(), None);
    assert!(!shell.activate_toast_action(), "no action left to run");
}

#[test]
fn quick_capture_request_is_pending_until_the_next_render() {
    let mut shell = fixture_shell();
    shell.request_quick_capture(true);
    assert_eq!(shell.pending_quick_capture, Some(true));
    assert!(!shell.quick_capture.is_open(), "opening needs a window");
    shell.request_quick_capture(false);
    assert_eq!(shell.pending_quick_capture, Some(false));
}

#[test]
fn quick_capture_empty_text_closes_without_writing() {
    let mut shell = fixture_shell();
    shell.quick_capture.open = true;
    shell.quick_capture.text = "   ".to_string();
    shell.save_quick_capture();
    assert!(!shell.quick_capture.is_open());
    assert_eq!(shell.toast_message(), None);
    assert!(shell.quick_capture.error().is_none());
}

#[test]
fn quick_capture_saves_into_the_journal_and_confirms() {
    let mut shell = fixture_shell();
    shell.quick_capture.open = true;
    shell.quick_capture.text = "capture this thought".to_string();
    shell.save_quick_capture();
    assert!(!shell.quick_capture.is_open());
    let toast = shell.toast_message().unwrap_or_default();
    assert!(
        toast.contains("journals/"),
        "the confirmation names the captured page: {toast}"
    );

    // The capture landed as a new block on the journal page.
    let page = journal_path(FIXTURE_TODAY);
    let blocks = shell.provider.load_blocks(&page).unwrap_or_default();
    assert!(blocks
        .iter()
        .any(|block| block.content == "capture this thought"));
}

#[test]
fn quick_capture_reports_a_provider_failure_and_stays_open() {
    struct FailingCapture;

    // Every `PanelData` and `SettingsData` method has a default, so the failing
    // provider only has to answer the shell's own calls.
    impl PanelData for FailingCapture {}
    impl SettingsData for FailingCapture {}

    impl ShellData for FailingCapture {
        fn snapshot(&self) -> ShellSnapshot {
            ShellSnapshot {
                vault: None,
                recents: Vec::new(),
            }
        }

        fn refresh(&self) -> ShellSnapshot {
            self.snapshot()
        }

        fn export_html(&self) -> Result<u64, String> {
            Ok(0)
        }

        fn quick_capture_save(&self, _text: String) -> Result<QuickCaptureResult, String> {
            Err("disk full".to_string())
        }
    }

    let mut shell = Shell::new(Arc::new(FailingCapture));
    shell.quick_capture.open = true;
    shell.quick_capture.text = "note".to_string();
    shell.save_quick_capture();
    assert!(
        shell.quick_capture.is_open(),
        "a failure keeps the dialog open"
    );
    assert_eq!(shell.quick_capture.error(), Some("disk full"));
}

#[test]
fn quick_capture_voice_flow_records_transcribes_and_closes() {
    let mut shell = fixture_shell();
    shell.quick_capture.open = true;
    shell.quick_capture.voice = true;
    shell.start_voice_capture();
    assert!(shell.quick_capture.is_recording());
    shell.stop_voice_capture();
    assert!(!shell.quick_capture.is_recording());
    assert!(!shell.quick_capture.is_open());
    assert_eq!(
        shell.toast_message(),
        Some(t_global("quickcapture.voice_done").as_str())
    );
}

#[test]
fn dismiss_overlay_closes_the_quick_capture_first() {
    let mut shell = fixture_shell();
    shell.quick_capture.open = true;
    shell.toaster.toast = Some(Toast::message("older toast"));
    assert!(shell.dismiss_overlay());
    assert!(
        !shell.quick_capture.is_open(),
        "the dialog is dismissed first"
    );
    assert_eq!(shell.toast_message(), Some("older toast"));
    assert!(shell.dismiss_overlay(), "then the toast");
    assert_eq!(shell.toast_message(), None);
}
