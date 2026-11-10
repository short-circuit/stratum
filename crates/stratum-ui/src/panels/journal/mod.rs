//! Journal surface for the native shell.
//!
//! [`JournalView`] reproduces the shipped desktop journal panel: a header with
//! prev/next day navigation, a clickable date that opens the calendar popup, a
//! read-mode toggle and the shared voice-capture control, followed by today's
//! editor and lazily loaded past days.
//!
//! The module is split so every file stays under the repository limit and the
//! pure models are testable without a window:
//!
//! - [`dates`] — date parsing/arithmetic, localized labels and calendar grids.
//! - [`stack`] — the deterministic past-day stacking/preload model.
//! - [`calendar`] — the calendar popup renderer.
//! - [`header`] — the journal header row.
//! - [`error`] — the error card with retry/repair actions.
//! - [`rows`] — today/past section rendering and placeholders.
//! - [`state`] — state accessors and the read-mode switch.
//! - [`navigation`] — day/month selection and calendar state transitions.
//! - [`reload`] — ensure/repair, editor recreation and stack synchronisation.
//!
//! [`JOURNAL_I18N_KEYS`] is the release gate list for every string this surface
//! renders.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{Datelike, Local};
use gpui_kit::component::ActiveTheme;
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use crate::panels::editor::EditorView;
use crate::panels::voice::VoiceCapture;
use crate::provider::{journal_path, ShellData};

pub mod calendar;
pub mod dates;
pub mod error;
pub mod header;
mod navigation;
mod reload;
pub mod rows;
pub mod stack;
mod state;

use stack::{StackPlan, DEFAULT_VISIBLE_WINDOW};

/// Every translation key the journal surface renders.
///
/// The contract test walks this list against all eight catalogs, so adding a
/// visible string without a catalog entry fails the build instead of leaking a
/// raw key into the UI.
pub const JOURNAL_I18N_KEYS: &[&str] = &[
    "common.loading",
    "common.retry",
    "dictation.transcribing",
    "journal.failed_to_create_journal",
    "journal.next_day",
    "journal.previous_day",
    "journal.repair_database",
    "journal_calendar.calendar",
    "journal_calendar.close_calendar",
    "journal_calendar.journal_calendar",
    "journal_calendar.month_april",
    "journal_calendar.month_august",
    "journal_calendar.month_december",
    "journal_calendar.month_february",
    "journal_calendar.month_january",
    "journal_calendar.month_july",
    "journal_calendar.month_june",
    "journal_calendar.month_march",
    "journal_calendar.month_may",
    "journal_calendar.month_november",
    "journal_calendar.month_october",
    "journal_calendar.month_september",
    "journal_calendar.next_month",
    "journal_calendar.previous_month",
    "journal_calendar.weekday_long_friday",
    "journal_calendar.weekday_long_monday",
    "journal_calendar.weekday_long_saturday",
    "journal_calendar.weekday_long_sunday",
    "journal_calendar.weekday_long_thursday",
    "journal_calendar.weekday_long_tuesday",
    "journal_calendar.weekday_long_wednesday",
    "journal_calendar.weekday_short_fri",
    "journal_calendar.weekday_short_mon",
    "journal_calendar.weekday_short_sat",
    "journal_calendar.weekday_short_sun",
    "journal_calendar.weekday_short_thu",
    "journal_calendar.weekday_short_tue",
    "journal_calendar.weekday_short_wed",
    "page.edit_mode",
    "page.read_mode",
    "voice_capture.cancel",
    "voice_capture.configure_stt_prompt",
    "voice_capture.error_fallback_inserted",
    "voice_capture.error_retry",
    "voice_capture.memo_inserted",
    "voice_capture.record",
    "voice_capture.recording",
    "voice_capture.retry_transcription",
];

/// One lazily materialised past-day section.
struct SectionState {
    date: String,
    path: String,
    editor: Option<Entity<EditorView>>,
}

/// The journal panel entity.
pub struct JournalView {
    provider: Arc<dyn ShellData>,
    today: String,
    target_date: String,
    calendar_open: bool,
    view_year: i32,
    view_month: u32,
    read_mode: bool,
    plan: StackPlan,
    journal_dates: Vec<String>,
    today_editor: Option<Entity<EditorView>>,
    sections: Vec<SectionState>,
    voice: Entity<VoiceCapture>,
    loading: bool,
    error: Option<String>,
    scroll: ScrollHandle,
    pending_scroll: Option<usize>,
    pending_navigation: Option<String>,
}

impl JournalView {
    /// Create the journal view and ensure today's page exists.
    ///
    /// `start_date` selects the initial target day (the `STRATUM_JOURNAL_DATE`
    /// capture flag); `calendar_open` starts with the popup expanded
    /// (`STRATUM_CALENDAR=1`). A failed ensure call surfaces the localized
    /// error card with retry/repair actions.
    pub fn new(
        provider: Arc<dyn ShellData>,
        start_date: Option<String>,
        calendar_open: bool,
        read_mode: bool,
        cx: &mut Context<Self>,
    ) -> Self {
        let today = Local::now().format("%Y-%m-%d").to_string();
        let target_date = start_date
            .filter(|date| dates::parse_date(date).is_some())
            .unwrap_or_else(|| today.clone());
        let mut error = None;
        if let Err(message) = provider.ensure_today_journal() {
            error = Some(format!(
                "{} {message}",
                t_global("journal.failed_to_create_journal")
            ));
        }
        if target_date != today {
            let _ = provider.create_journal_day(&target_date);
        }
        let journal_dates = provider.journal_dates();
        let plan = StackPlan::build(&journal_dates, &today, DEFAULT_VISIBLE_WINDOW);
        let editor_path = journal_path(&today);
        let editor_provider = provider.clone();
        let today_editor = Some(cx.new(move |cx| {
            let mut editor = EditorView::new(editor_path, editor_provider, read_mode, cx);
            editor.set_auto_height(true, cx);
            editor
        }));
        let voice_path = journal_path(&target_date);
        let voice_provider = provider.clone();
        let voice = cx.new(move |cx| VoiceCapture::new(voice_path, voice_provider, cx));
        let (view_year, view_month) = month_of(&target_date).unwrap_or_else(|| {
            let now = Local::now();
            (now.year(), now.month())
        });
        let mut view = Self {
            provider,
            today,
            target_date,
            calendar_open,
            view_year,
            view_month,
            read_mode,
            plan,
            journal_dates,
            today_editor,
            sections: Vec::new(),
            voice,
            loading: false,
            error,
            scroll: ScrollHandle::new(),
            pending_scroll: None,
            pending_navigation: None,
        };
        view.rebuild_sections(HashMap::new());
        view.sync_stack(0.0, cx);
        let target = view.target_date.clone();
        view.focus_date(&target, cx);
        view
    }
}

impl Render for JournalView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self
            .voice
            .update(cx, |voice, _| voice.take_reload_request())
        {
            self.reload_today_editor(cx);
        }
        self.sync_child_navigation(cx);
        // A pending scroll needs layout bounds from the previous frame; ask for
        // one more frame while they are still unknown.
        if !self.apply_pending_scroll() {
            cx.notify();
        }
        let offset = self.scroll_offset();
        self.sync_stack(offset, cx);

        let theme = cx.theme();
        let background = theme.colors.background;
        let foreground = theme.colors.foreground;

        div()
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .bg(background)
            .text_color(foreground)
            .child(header::header(self, cx))
            .when(self.error.is_some(), |element| {
                element.child(error::error_card(self, cx))
            })
            .child(
                div().flex().flex_col().flex_1().overflow_hidden().child(
                    div()
                        .id("journal-scroll")
                        .flex()
                        .flex_col()
                        .flex_1()
                        .overflow_y_scroll()
                        .track_scroll(&self.scroll)
                        .child(rows::today_section(self, cx))
                        .children(
                            (0..self.sections.len())
                                .map(|index| rows::section_row(self, index, cx)),
                        ),
                ),
            )
            // Rendered last so the popup paints above the stacked sections,
            // mirroring the shipped MUI portal rather than sitting behind them.
            .when(self.calendar_open, |element| {
                element.child(calendar::calendar_popup(self, cx))
            })
    }
}

/// Extract the year and month from a validated date.
fn month_of(date: &str) -> Option<(i32, u32)> {
    dates::parse_date(date).map(|parsed| (parsed.year(), parsed.month()))
}

/// Previous calendar month, wrapping at the year boundary.
fn previous_month(year: i32, month: u32) -> (i32, u32) {
    if month <= 1 {
        (year - 1, 12)
    } else {
        (year, month - 1)
    }
}

/// Next calendar month, wrapping at the year boundary.
fn next_month(year: i32, month: u32) -> (i32, u32) {
    if month >= 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // `gpui_kit::*` re-exports gpui's `test` attribute macro, which shadows the
    // built-in attribute. Import the built-in explicitly so these plain unit
    // tests stay cheap (no GPUI app context is created).
    use ::core::prelude::v1::test;

    #[test]
    fn month_navigation_wraps_at_year_boundaries() {
        assert_eq!(previous_month(2026, 1), (2025, 12));
        assert_eq!(next_month(2026, 12), (2027, 1));
        assert_eq!(previous_month(2026, 6), (2026, 5));
        assert_eq!(next_month(2026, 6), (2026, 7));
    }

    #[test]
    fn month_of_rejects_invalid_dates() {
        assert_eq!(month_of("2026-11-10"), Some((2026, 11)));
        assert_eq!(month_of("bogus"), None);
    }
}
