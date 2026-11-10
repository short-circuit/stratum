//! Journal date arithmetic and calendar grid construction.
//!
//! Every helper here is pure and window-free so the panel contract can be
//! tested without a GPUI app. Dates are `YYYY-MM-DD` strings validated through
//! [`chrono::NaiveDate`]; invalid input is rejected rather than silently
//! normalised.
//!
//! Month and weekday labels come from the `journal_calendar.*` catalog keys, and
//! the long display form is built with [`stratum_i18n::interpolate`] over the
//! literal `"{{weekday}}, {{month}} {{day}}, {{year}}"` template, so it has no
//! catalog key of its own.

use chrono::{Datelike, Duration, NaiveDate};
use stratum_i18n::{interpolate, t_global};

/// Parse a `YYYY-MM-DD` date string, rejecting impossible calendar dates.
pub fn parse_date(date: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()
}

/// Shift a `YYYY-MM-DD` date by `delta` days.
///
/// Returns `None` when the input is invalid or the result falls outside the
/// representable date range.
pub fn add_days(date: &str, delta: i64) -> Option<String> {
    let parsed = parse_date(date)?;
    parsed
        .checked_add_signed(Duration::days(delta))
        .map(|next| next.format("%Y-%m-%d").to_string())
}

/// The localized long display form for a journal date.
///
/// Invalid input falls back to the raw string so a malformed route parameter
/// still renders something instead of an empty label.
pub fn display_date(date: &str) -> String {
    let Some(parsed) = parse_date(date) else {
        return date.to_string();
    };
    let weekday = t_global(weekday_long_key(parsed.weekday().num_days_from_sunday()));
    let month = t_global(month_key(parsed.month()));
    let day = parsed.day().to_string();
    let year = parsed.year().to_string();
    interpolate(
        "{{weekday}}, {{month}} {{day}}, {{year}}",
        &[
            ("weekday", weekday.as_str()),
            ("month", month.as_str()),
            ("day", day.as_str()),
            ("year", year.as_str()),
        ],
    )
}

/// The localized month/year label shown above the calendar grid.
pub fn month_label(year: i32, month: u32) -> String {
    let name = t_global(month_key(month));
    format!("{name} {year}")
}

/// One day cell in the calendar grid.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalendarCell {
    /// Day of month (1-based).
    pub day: u32,
    /// `YYYY-MM-DD` date for the cell.
    pub date: String,
    /// Whether a journal page exists for this date.
    pub is_journal: bool,
    /// Whether this date is today.
    pub is_today: bool,
}

/// A calendar month grid: leading blanks plus the day cells.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalendarGrid {
    /// Empty cells before the first day (Sunday-based weekday index).
    pub leading_blanks: usize,
    /// One cell per day in the month.
    pub cells: Vec<CalendarCell>,
}

/// Build the calendar grid for `year`/`month`.
///
/// `journal_dates` marks existing journal pages and `today` marks the current
/// day. Returns `None` for an invalid month or year.
pub fn calendar_grid(
    year: i32,
    month: u32,
    journal_dates: &[String],
    today: &str,
) -> Option<CalendarGrid> {
    if !(1..=12).contains(&month) {
        return None;
    }
    let first = NaiveDate::from_ymd_opt(year, month, 1)?;
    let leading_blanks = first.weekday().num_days_from_sunday() as usize;
    let mut cells = Vec::new();
    let mut cursor = first;
    while cursor.month() == month {
        let date = cursor.format("%Y-%m-%d").to_string();
        cells.push(CalendarCell {
            day: cursor.day(),
            is_journal: journal_dates.iter().any(|existing| existing == &date),
            is_today: date == today,
            date,
        });
        cursor = cursor.succ_opt()?;
    }
    Some(CalendarGrid {
        leading_blanks,
        cells,
    })
}

/// Catalog key for a month number (1-based).
fn month_key(month: u32) -> &'static str {
    match month {
        1 => "journal_calendar.month_january",
        2 => "journal_calendar.month_february",
        3 => "journal_calendar.month_march",
        4 => "journal_calendar.month_april",
        5 => "journal_calendar.month_may",
        6 => "journal_calendar.month_june",
        7 => "journal_calendar.month_july",
        8 => "journal_calendar.month_august",
        9 => "journal_calendar.month_september",
        10 => "journal_calendar.month_october",
        11 => "journal_calendar.month_november",
        _ => "journal_calendar.month_december",
    }
}

/// Catalog key for a Sunday-based weekday index.
fn weekday_long_key(index: u32) -> &'static str {
    match index {
        0 => "journal_calendar.weekday_long_sunday",
        1 => "journal_calendar.weekday_long_monday",
        2 => "journal_calendar.weekday_long_tuesday",
        3 => "journal_calendar.weekday_long_wednesday",
        4 => "journal_calendar.weekday_long_thursday",
        5 => "journal_calendar.weekday_long_friday",
        _ => "journal_calendar.weekday_long_saturday",
    }
}

/// Catalog key for a Sunday-based short weekday index.
pub fn weekday_short_key(index: usize) -> &'static str {
    match index {
        0 => "journal_calendar.weekday_short_sun",
        1 => "journal_calendar.weekday_short_mon",
        2 => "journal_calendar.weekday_short_tue",
        3 => "journal_calendar.weekday_short_wed",
        4 => "journal_calendar.weekday_short_thu",
        5 => "journal_calendar.weekday_short_fri",
        _ => "journal_calendar.weekday_short_sat",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stratum_i18n::{set_locale, Locale};

    fn english<T>(run: impl FnOnce() -> T) -> T {
        set_locale(Locale::En);
        run()
    }

    #[test]
    fn parse_rejects_invalid_dates() {
        assert_eq!(parse_date("2026-11-10").map(|d| d.day()), Some(10));
        assert_eq!(parse_date("2026-02-30"), None);
        assert_eq!(parse_date("not-a-date"), None);
        assert_eq!(parse_date("2026-11-10T00:00:00"), None);
    }

    #[test]
    fn add_days_crosses_month_and_year_boundaries() {
        assert_eq!(add_days("2026-11-10", -1), Some("2026-11-09".to_string()));
        assert_eq!(add_days("2026-12-31", 1), Some("2027-01-01".to_string()));
        assert_eq!(add_days("2026-03-01", -1), Some("2026-02-28".to_string()));
        assert_eq!(add_days("2024-03-01", -1), Some("2024-02-29".to_string()));
        assert_eq!(add_days("bogus", 1), None);
    }

    #[test]
    fn display_date_uses_localized_month_and_weekday() {
        english(|| {
            assert_eq!(display_date("2026-11-10"), "Tuesday, November 10, 2026");
            assert_eq!(display_date("bogus"), "bogus");
        });
    }

    #[test]
    fn month_labels_and_weekday_keys_resolve() {
        english(|| {
            assert_eq!(month_label(2026, 11), "November 2026");
            assert_eq!(month_label(2026, 1), "January 2026");
            for index in 0..7 {
                let key = weekday_short_key(index);
                assert_ne!(t_global(key), key);
            }
        });
    }

    #[test]
    fn calendar_grid_marks_journal_and_today() {
        let grid = calendar_grid(2026, 11, &["2026-11-09".to_string()], "2026-11-10")
            .expect("valid month");
        assert_eq!(grid.leading_blanks, 0, "2026-11-01 is a Sunday");
        assert_eq!(grid.cells.len(), 30);
        assert!(grid
            .cells
            .iter()
            .any(|cell| cell.is_journal && cell.day == 9));
        assert!(grid
            .cells
            .iter()
            .any(|cell| cell.is_today && cell.day == 10));
        assert_eq!(grid.cells[0].date, "2026-11-01");
    }

    #[test]
    fn calendar_grid_rejects_invalid_month() {
        assert!(calendar_grid(2026, 0, &[], "2026-11-10").is_none());
        assert!(calendar_grid(2026, 13, &[], "2026-11-10").is_none());
    }
}
