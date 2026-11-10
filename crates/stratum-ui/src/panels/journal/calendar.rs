//! Journal calendar popup.
//!
//! Reproduces the shipped desktop journal calendar: a title row with a close
//! control, a month header with previous/next month buttons, Sunday-first short
//! weekday labels and a seven-column day grid. Journal dates and today are
//! highlighted, and selecting a day closes the popup and opens that journal
//! page.
//!
//! gpui 0.2 has no CSS grid layout, so [`week_rows`] pads the month grid into
//! seven-slot weeks and the renderer builds one flex row per week. The helper is
//! pure and unit-tested without a window.

use gpui_kit::assets::IconName;
use gpui_kit::component::{ActiveTheme, Icon, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use super::dates::{calendar_grid, month_label, weekday_short_key, CalendarCell, CalendarGrid};
use super::JournalView;

/// Render the calendar popup.
pub fn calendar_popup(view: &JournalView, cx: &mut Context<JournalView>) -> impl IntoElement {
    let theme = cx.theme();
    let background = theme.colors.background;
    let panel = theme.colors.secondary;
    let border = theme.colors.border;
    let foreground = theme.colors.foreground;
    let muted = theme.colors.muted_foreground;
    let accent = theme.colors.accent;
    let accent_foreground = theme.colors.accent_foreground;

    let (year, month) = view.view_month();
    let grid = calendar_grid(year, month, view.journal_dates(), view.today());
    let weeks = grid.as_ref().map(week_rows).unwrap_or_default();

    // Anchor under the header's calendar control on the right, matching the
    // shipped Popover's `anchorOrigin` (bottom-left of the trigger).
    div()
        .absolute()
        .top(px(52.0))
        .right(px(12.0))
        .w(px(300.0))
        .p(px(12.0))
        .rounded(px(8.0))
        .bg(panel)
        .border_1()
        .border_color(border)
        .flex()
        .flex_col()
        .gap(px(8.0))
        .child(popup_header(muted, foreground, cx))
        .child(month_header(year, month, muted, foreground, cx))
        .child(weekday_header(muted))
        .children(weeks.into_iter().map(|week| {
            week_row(
                week,
                background,
                accent,
                accent_foreground,
                foreground,
                muted,
                cx,
            )
        }))
}

/// Title row with the localized popup name and close control.
fn popup_header(muted: Hsla, foreground: Hsla, cx: &mut Context<JournalView>) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .text_size(px(13.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(foreground)
                .child(t_global("journal_calendar.journal_calendar")),
        )
        .child(
            div()
                .id("journal-calendar-close")
                .flex()
                .items_center()
                .gap(px(4.0))
                .px(px(6.0))
                .py(px(2.0))
                .rounded(px(4.0))
                .text_size(px(11.0))
                .text_color(muted)
                .cursor_pointer()
                .child(Icon::new(IconName::X).with_size(px(12.0)))
                .child(div().child(t_global("journal_calendar.close_calendar")))
                .on_click(cx.listener(|view, _, _, cx| view.close_calendar(cx))),
        )
}

/// Month header with previous/next month controls.
fn month_header(
    year: i32,
    month: u32,
    muted: Hsla,
    foreground: Hsla,
    cx: &mut Context<JournalView>,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .child(month_arrow(
            "journal-calendar-previous-month",
            IconName::ChevronLeft,
            muted,
            cx,
            |view, cx| view.previous_month(cx),
        ))
        .child(
            div()
                .text_size(px(13.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(foreground)
                .child(month_label(year, month)),
        )
        .child(month_arrow(
            "journal-calendar-next-month",
            IconName::ChevronRight,
            muted,
            cx,
            |view, cx| view.next_month(cx),
        ))
}

/// One month-navigation arrow.
fn month_arrow(
    id: &'static str,
    icon: IconName,
    muted: Hsla,
    cx: &mut Context<JournalView>,
    action: impl Fn(&mut JournalView, &mut Context<JournalView>) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .size(px(22.0))
        .rounded(px(4.0))
        .cursor_pointer()
        .on_click(cx.listener(move |view, _, _, cx| action(view, cx)))
        .child(Icon::new(icon).with_size(px(14.0)).text_color(muted))
}

/// The Sunday-first short weekday header.
fn weekday_header(muted: Hsla) -> impl IntoElement {
    let mut row = div().flex().gap(px(2.0));
    for index in 0..7 {
        row = row.child(
            div()
                .flex()
                .flex_1()
                .items_center()
                .justify_center()
                .h(px(18.0))
                .text_size(px(10.0))
                .text_color(muted)
                .child(t_global(weekday_short_key(index))),
        );
    }
    row
}

/// One seven-slot calendar week.
fn week_row(
    week: Vec<Option<CalendarCell>>,
    background: Hsla,
    accent: Hsla,
    accent_foreground: Hsla,
    foreground: Hsla,
    muted: Hsla,
    cx: &mut Context<JournalView>,
) -> impl IntoElement {
    let mut row = div().flex().gap(px(2.0));
    for slot in week {
        row = match slot {
            Some(cell) => row.child(day_cell(
                &cell,
                background,
                accent,
                accent_foreground,
                foreground,
                muted,
                cx,
            )),
            None => row.child(div().flex_1().h(px(26.0))),
        };
    }
    row
}

/// One clickable day cell.
fn day_cell(
    cell: &CalendarCell,
    background: Hsla,
    accent: Hsla,
    accent_foreground: Hsla,
    foreground: Hsla,
    muted: Hsla,
    cx: &mut Context<JournalView>,
) -> impl IntoElement {
    let (fill, color, weight) = if cell.is_today {
        (accent, accent_foreground, FontWeight::BOLD)
    } else if cell.is_journal {
        (muted.opacity(0.2), foreground, FontWeight::SEMIBOLD)
    } else {
        (background, muted, FontWeight::NORMAL)
    };
    let date = cell.date.clone();
    div()
        .id(format!("journal-day-{}", cell.date))
        .flex()
        .flex_1()
        .items_center()
        .justify_center()
        .h(px(26.0))
        .rounded(px(4.0))
        .bg(fill)
        .text_color(color)
        .text_size(px(12.0))
        .font_weight(weight)
        .cursor_pointer()
        .child(cell.day.to_string())
        .on_click(cx.listener(move |view, _, _, cx| view.select_date(&date, cx)))
}

/// Pad a month grid into Sunday-based seven-slot weeks.
///
/// Leading blanks come from the grid's first weekday; trailing slots are filled
/// with `None` so every row has exactly seven cells.
pub fn week_rows(grid: &CalendarGrid) -> Vec<Vec<Option<CalendarCell>>> {
    let mut slots: Vec<Option<CalendarCell>> =
        Vec::with_capacity(grid.leading_blanks + grid.cells.len() + 6);
    slots.extend(std::iter::repeat_n(None, grid.leading_blanks));
    slots.extend(grid.cells.iter().cloned().map(Some));
    while !slots.len().is_multiple_of(7) {
        slots.push(None);
    }
    slots.chunks(7).map(|week| week.to_vec()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::panels::journal::dates::calendar_grid;
    // `gpui_kit::*` re-exports gpui's `test` attribute macro, which shadows the
    // built-in attribute. Import the built-in explicitly so these plain unit
    // tests stay cheap (no GPUI app context is created).
    use ::core::prelude::v1::test;

    #[test]
    fn week_rows_pad_to_seven_slots() {
        let grid = calendar_grid(2026, 11, &[], "2026-11-10").expect("valid month");
        let weeks = week_rows(&grid);
        assert!(!weeks.is_empty());
        for week in &weeks {
            assert_eq!(week.len(), 7, "every week row has seven slots");
        }
        let first = weeks[0][grid.leading_blanks]
            .as_ref()
            .expect("day one follows the leading blanks");
        assert_eq!(first.day, 1);
        assert_eq!(first.date, "2026-11-01");
    }

    #[test]
    fn week_rows_preserve_leading_blanks() {
        // 2026-02-01 is a Sunday, so this month has no leading blanks; use a
        // month that starts mid-week to exercise the padding path.
        let grid = calendar_grid(2026, 7, &[], "2026-11-10").expect("valid month");
        assert!(grid.leading_blanks > 0);
        let weeks = week_rows(&grid);
        for slot in &weeks[0][..grid.leading_blanks] {
            assert!(slot.is_none());
        }
        assert!(weeks[0][grid.leading_blanks].is_some());
    }

    #[test]
    fn week_rows_mark_journal_and_today_cells() {
        let grid = calendar_grid(2026, 11, &["2026-11-09".to_string()], "2026-11-10")
            .expect("valid month");
        let weeks = week_rows(&grid);
        let cells: Vec<&CalendarCell> = weeks.iter().flatten().flatten().collect();
        assert!(cells.iter().any(|cell| cell.is_journal && cell.day == 9));
        assert!(cells.iter().any(|cell| cell.is_today && cell.day == 10));
    }
}
