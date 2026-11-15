//! Window-free contract tests for the native journal surface.
//!
//! These tests exercise the pieces the gpui journal view delegates to: date
//! parsing/arithmetic and localized labels, the deterministic calendar grid and
//! seven-slot week layout, the lazy stack plan, the voice-capture stage machine
//! and the fixture journal/voice data. They also walk [`JOURNAL_I18N_KEYS`]
//! against all eight catalogs, so a visible string without a catalog entry fails
//! the build instead of leaking a raw key into the UI.

use chrono::Datelike;
use stratum_i18n::{t, Locale};
use stratum_ui::panels::journal::calendar::week_rows;
use stratum_ui::panels::journal::dates::{
    add_days, calendar_grid, display_date, month_label, parse_date, weekday_short_key,
};
use stratum_ui::panels::journal::stack::{StackPlan, DEFAULT_VISIBLE_WINDOW, PRELOAD_MARGIN};
use stratum_ui::panels::journal::JOURNAL_I18N_KEYS;
use stratum_ui::panels::voice::{format_timer, VoiceStage};
use stratum_ui::provider::{
    journal_path, FixtureProvider, ShellData, FIXTURE_JOURNAL_DATES, FIXTURE_RECORDING,
    FIXTURE_TODAY,
};

#[test]
fn journal_keys_are_present_in_every_catalog() {
    for locale in Locale::ALL {
        for &key in JOURNAL_I18N_KEYS {
            assert_ne!(t(locale, key), key, "{key} missing in {}", locale.code());
        }
    }
}

#[test]
fn journal_keys_cover_the_shipped_controls() {
    for key in [
        "journal.previous_day",
        "journal.next_day",
        "journal.repair_database",
        "journal.failed_to_create_journal",
        "journal_calendar.calendar",
        "journal_calendar.close_calendar",
        "journal_calendar.previous_month",
        "journal_calendar.next_month",
        "page.read_mode",
        "page.edit_mode",
        "voice_capture.record",
        "voice_capture.retry_transcription",
        "common.loading",
        "common.retry",
    ] {
        assert!(
            JOURNAL_I18N_KEYS.contains(&key),
            "{key} is missing from the release gate list"
        );
    }
}

#[test]
fn date_helpers_cross_boundaries_and_reject_invalid_input() {
    assert_eq!(parse_date("2026-11-10").map(|date| date.day()), Some(10));
    assert_eq!(parse_date("2026-02-30"), None);
    assert_eq!(add_days("2026-12-31", 1), Some("2027-01-01".to_string()));
    assert_eq!(add_days("2024-03-01", -1), Some("2024-02-29".to_string()));
    assert_eq!(add_days("bogus", 1), None);
}

#[test]
fn display_labels_and_weekday_keys_resolve() {
    let label = display_date("2026-11-10");
    assert!(
        label.contains("2026"),
        "display date lost the year: {label}"
    );
    let month = month_label(2026, 11);
    assert!(month.contains("2026"), "month label lost the year: {month}");
    for locale in Locale::ALL {
        for index in 0..7 {
            let key = weekday_short_key(index);
            assert_ne!(t(locale, key), key, "{key} missing in {}", locale.code());
        }
    }
}

#[test]
fn calendar_grid_marks_journal_days_and_today() {
    let grid =
        calendar_grid(2026, 11, &["2026-11-09".to_string()], "2026-11-10").expect("valid month");
    assert_eq!(grid.cells.len(), 30);
    assert!(grid
        .cells
        .iter()
        .any(|cell| cell.is_journal && cell.day == 9));
    assert!(grid
        .cells
        .iter()
        .any(|cell| cell.is_today && cell.day == 10));
    assert!(calendar_grid(2026, 13, &[], "2026-11-10").is_none());
}

#[test]
fn calendar_weeks_are_padded_to_seven_slots() {
    let grid = calendar_grid(2026, 11, &[], "2026-11-10").expect("valid month");
    let weeks = week_rows(&grid);
    assert!(!weeks.is_empty());
    for week in &weeks {
        assert_eq!(week.len(), 7);
    }
    let first = weeks[0][grid.leading_blanks]
        .as_ref()
        .expect("first day follows leading blanks");
    assert_eq!(first.date, "2026-11-01");
}

#[test]
fn stack_plan_starts_with_today_and_grows_in_windows() {
    let dates: Vec<String> = FIXTURE_JOURNAL_DATES
        .iter()
        .map(|date| date.to_string())
        .collect();
    let mut plan = StackPlan::build(&dates, FIXTURE_TODAY, DEFAULT_VISIBLE_WINDOW);
    assert_eq!(plan.sections()[0].date, FIXTURE_TODAY);
    assert!(plan.sections()[0].is_today);
    assert_eq!(plan.sections()[0].path, journal_path(FIXTURE_TODAY));
    assert_eq!(plan.total(), dates.len() + 1);

    let required = plan.load_indices(0.0);
    assert!(required.contains(&0), "today is always loaded");
    assert!(plan.visible() <= plan.total());
    const { assert!(PRELOAD_MARGIN > 0.0) };
    assert_eq!(plan.grow(DEFAULT_VISIBLE_WINDOW), plan.total());
}

#[test]
fn fixture_journal_and_voice_flow_is_deterministic() {
    let provider = FixtureProvider::new();
    provider.ensure_today_journal().expect("today page");
    let dates = provider.journal_dates();
    assert!(dates.contains(&FIXTURE_TODAY.to_string()));
    for date in FIXTURE_JOURNAL_DATES {
        assert!(dates.contains(&date.to_string()));
    }
    assert!(provider.stt_configured());
    assert_eq!(
        provider.voice_start(&journal_path(FIXTURE_TODAY)),
        Ok(FIXTURE_RECORDING.to_string())
    );
    assert_eq!(
        provider.voice_transcribe(FIXTURE_RECORDING, &journal_path(FIXTURE_TODAY)),
        Ok(2)
    );
}

#[test]
fn voice_stage_machine_matches_the_shipped_pipeline() {
    assert!(VoiceStage::Idle.can_start());
    assert!(VoiceStage::Recording.can_stop());
    assert!(VoiceStage::Fallback.can_retry());
    assert_eq!(VoiceStage::Idle.after_start(), VoiceStage::Recording);
    assert_eq!(
        VoiceStage::Recording.after_stop(true),
        VoiceStage::Transcribing
    );
    assert_eq!(
        VoiceStage::Recording.after_stop(false),
        VoiceStage::NeedsConfig
    );
    assert_eq!(format_timer(65), "1:05");
    for stage in [
        VoiceStage::Idle,
        VoiceStage::Recording,
        VoiceStage::Transcribing,
        VoiceStage::Inserted,
        VoiceStage::NeedsConfig,
        VoiceStage::Fallback,
        VoiceStage::Error,
    ] {
        assert_ne!(t(Locale::En, stage.label_key()), stage.label_key());
    }
}
