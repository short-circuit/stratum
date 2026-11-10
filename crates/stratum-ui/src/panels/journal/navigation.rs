//! Journal day/month navigation.
//!
//! Selecting a date creates the journal page when needed, rebuilds the stack and
//! moves the calendar view month. All methods are thin state transitions on
//! [`JournalView`]; the pure date math lives in [`super::dates`].

use super::*;

impl JournalView {
    /// Move the target date one day back.
    pub fn previous_day(&mut self, cx: &mut Context<Self>) {
        if let Some(date) = dates::add_days(&self.target_date, -1) {
            self.select_date(&date, cx);
        }
    }

    /// Move the target date one day forward.
    pub fn next_day(&mut self, cx: &mut Context<Self>) {
        if let Some(date) = dates::add_days(&self.target_date, 1) {
            self.select_date(&date, cx);
        }
    }

    /// Select a date from the calendar or day arrows.
    ///
    /// Creates the journal page when it does not exist, refreshes the stack,
    /// closes the calendar and points the header at the new day.
    pub fn select_date(&mut self, date: &str, cx: &mut Context<Self>) {
        if dates::parse_date(date).is_none() {
            return;
        }
        if let Err(message) = self.provider.create_journal_day(date) {
            self.error = Some(format!(
                "{} {message}",
                t_global("journal.failed_to_create_journal")
            ));
            return;
        }
        self.error = None;
        self.target_date = date.to_string();
        if let Some((year, month)) = month_of(date) {
            self.view_year = year;
            self.view_month = month;
        }
        self.calendar_open = false;
        self.journal_dates = self.provider.journal_dates();
        self.plan = StackPlan::build(&self.journal_dates, &self.today, DEFAULT_VISIBLE_WINDOW);
        let existing = self.take_section_editors();
        self.rebuild_sections(existing);
        self.sync_stack(0.0, cx);
        cx.notify();
    }

    /// Open or close the calendar popup.
    pub fn toggle_calendar(&mut self, cx: &mut Context<Self>) {
        self.calendar_open = !self.calendar_open;
        if let Some((year, month)) = month_of(&self.target_date) {
            self.view_year = year;
            self.view_month = month;
        }
        cx.notify();
    }

    /// Close the calendar popup.
    pub fn close_calendar(&mut self, cx: &mut Context<Self>) {
        if self.calendar_open {
            self.calendar_open = false;
            cx.notify();
        }
    }

    /// Move the calendar view one month back.
    pub fn previous_month(&mut self, cx: &mut Context<Self>) {
        let (year, month) = previous_month(self.view_year, self.view_month);
        self.view_year = year;
        self.view_month = month;
        cx.notify();
    }

    /// Move the calendar view one month forward.
    pub fn next_month(&mut self, cx: &mut Context<Self>) {
        let (year, month) = next_month(self.view_year, self.view_month);
        self.view_year = year;
        self.view_month = month;
        cx.notify();
    }
}
