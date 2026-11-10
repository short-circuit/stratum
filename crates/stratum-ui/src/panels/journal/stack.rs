//! Past-day stacking model for the journal surface.
//!
//! The shipped React journal grows a lazy list of past days: the today entry is
//! always rendered, and older sections materialise as the panel scrolls. This
//! module keeps that policy pure and deterministic so it can be unit-tested
//! without a window. [`StackPlan::load_indices`] answers which visible sections
//! must be loaded for a scroll offset, using the shipped preload margin.

use crate::provider::journal_path;

/// Shipped initial visible window (25 past days).
pub const DEFAULT_VISIBLE_WINDOW: usize = 25;
/// Placeholder height used for cumulative offset math.
pub const SECTION_PLACEHOLDER_HEIGHT: f32 = 96.0;
/// Preload margin ahead of/behind the viewport (shipped `rootMargin: 400px`).
pub const PRELOAD_MARGIN: f32 = 400.0;

/// One stacked journal section.
#[derive(Clone, Debug, PartialEq)]
pub struct StackSection {
    /// Position in the stack (0 is today).
    pub index: usize,
    /// `YYYY-MM-DD` date for the section.
    pub date: String,
    /// Vault-relative page path.
    pub path: String,
    /// Cumulative placeholder offset in content pixels.
    pub offset: f32,
    /// Placeholder height in content pixels.
    pub height: f32,
    /// Whether this section is today's entry.
    pub is_today: bool,
}

/// A deterministic plan for the stacked past-day list.
#[derive(Clone, Debug, PartialEq)]
pub struct StackPlan {
    sections: Vec<StackSection>,
    visible: usize,
}

impl StackPlan {
    /// Build a plan from journal dates, today and the initial visible window.
    ///
    /// The first section is always today. Remaining sections are the other
    /// dates, newest first, capped at `window` (clamped to at least one
    /// section). Duplicate and malformed dates are ignored.
    pub fn build(journal_dates: &[String], today: &str, window: usize) -> Self {
        let mut past: Vec<String> = journal_dates
            .iter()
            .filter(|date| date.as_str() != today)
            .cloned()
            .collect();
        past.sort();
        past.dedup();
        past.reverse();

        let mut sections = Vec::with_capacity(past.len() + 1);
        let mut offset = 0.0;
        let mut push = |date: String, is_today: bool| {
            let index = sections.len();
            sections.push(StackSection {
                index,
                path: journal_path(&date),
                date,
                offset,
                height: SECTION_PLACEHOLDER_HEIGHT,
                is_today,
            });
            offset += SECTION_PLACEHOLDER_HEIGHT;
        };
        push(today.to_string(), true);
        for date in past {
            push(date, false);
        }
        let visible = window.max(1).min(sections.len());
        Self { sections, visible }
    }

    /// The full ordered section list.
    pub fn sections(&self) -> &[StackSection] {
        &self.sections
    }

    /// The number of sections currently materialised.
    pub fn visible(&self) -> usize {
        self.visible
    }

    /// The total number of sections.
    pub fn total(&self) -> usize {
        self.sections.len()
    }

    /// Grow the visible window by `delta`, returning the new visible count.
    pub fn grow(&mut self, delta: usize) -> usize {
        self.visible = (self.visible + delta).min(self.sections.len());
        self.visible
    }

    /// Cumulative placeholder height for the current visible window.
    pub fn total_height(&self) -> f32 {
        self.visible as f32 * SECTION_PLACEHOLDER_HEIGHT
    }

    /// The placeholder offset of one section, if it exists.
    pub fn offset_of(&self, index: usize) -> Option<f32> {
        self.sections.get(index).map(|section| section.offset)
    }

    /// Section indices that must be loaded for `scroll_offset`.
    ///
    /// Today is always loaded. Past sections are loaded when their placeholder
    /// band intersects the scroll position widened by [`PRELOAD_MARGIN`], and
    /// only while inside the visible window. Out-of-range offsets clamp to the
    /// first or last band, so a stale scroll value never produces an empty set.
    pub fn load_indices(&self, scroll_offset: f32) -> Vec<usize> {
        let top = scroll_offset.max(0.0);
        let bottom = top + SECTION_PLACEHOLDER_HEIGHT;
        let mut indices = Vec::new();
        for section in self.sections.iter().take(self.visible) {
            let start = section.offset - PRELOAD_MARGIN;
            let end = section.offset + section.height + PRELOAD_MARGIN;
            if section.is_today || (start <= bottom && end >= top) {
                indices.push(section.index);
            }
        }
        if self.visible > 1 {
            // An offset past the end still loads the last visible section so a
            // stale scroll value never leaves the tail as a permanent
            // placeholder.
            let last = self.visible - 1;
            if top > self.sections[last].offset && !indices.contains(&last) {
                indices.push(last);
            }
        }
        indices
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dates() -> Vec<String> {
        vec![
            "2026-11-09".to_string(),
            "2026-11-08".to_string(),
            "2026-11-07".to_string(),
        ]
    }

    #[test]
    fn empty_stack_still_has_today() {
        let plan = StackPlan::build(&[], "2026-11-10", DEFAULT_VISIBLE_WINDOW);
        assert_eq!(plan.total(), 1);
        assert_eq!(plan.visible(), 1);
        assert_eq!(plan.sections()[0].path, "journals/2026-11-10.md");
        assert!(plan.sections()[0].is_today);
        assert_eq!(plan.load_indices(0.0), vec![0]);
    }

    #[test]
    fn single_past_day_is_stacked_after_today() {
        let plan = StackPlan::build(&["2026-11-09".to_string()], "2026-11-10", 25);
        assert_eq!(plan.total(), 2);
        assert_eq!(plan.sections()[1].date, "2026-11-09");
        assert_eq!(plan.sections()[1].offset, SECTION_PLACEHOLDER_HEIGHT);
        assert!(!plan.sections()[1].is_today);
    }

    #[test]
    fn window_growth_is_capped_at_the_total() {
        let mut plan = StackPlan::build(&dates(), "2026-11-10", 1);
        assert_eq!(plan.visible(), 1);
        assert_eq!(plan.grow(1), 2);
        assert_eq!(plan.grow(10), 4);
        assert_eq!(plan.grow(5), 4);
    }

    #[test]
    fn load_indices_follow_the_preload_margin() {
        let plan = StackPlan::build(&dates(), "2026-11-10", 25);
        // The 400 px preload margin covers every placeholder band in this tiny
        // four-section stack, so the first frame materialises all of them.
        assert_eq!(plan.load_indices(0.0), vec![0, 1, 2, 3]);
        let offset = plan.offset_of(3).expect("section");
        assert!(plan.load_indices(offset).contains(&3));
    }

    #[test]
    fn out_of_range_offsets_still_load_a_section() {
        let plan = StackPlan::build(&dates(), "2026-11-10", 2);
        assert_eq!(plan.load_indices(-100.0), vec![0, 1]);
        // Today is always loaded, and a scroll past the end also materialises
        // the last visible section.
        let indices = plan.load_indices(10_000.0);
        assert_eq!(indices, vec![0, 1]);
        assert!(indices.iter().all(|index| *index < plan.visible()));
    }

    #[test]
    fn duplicate_and_today_dates_are_collapsed() {
        let plan = StackPlan::build(
            &[
                "2026-11-09".to_string(),
                "2026-11-09".to_string(),
                "2026-11-10".to_string(),
                "2026-11-07".to_string(),
            ],
            "2026-11-10",
            25,
        );
        let dates: Vec<_> = plan
            .sections()
            .iter()
            .map(|section| section.date.as_str())
            .collect();
        assert_eq!(dates, ["2026-11-10", "2026-11-09", "2026-11-07"]);
    }
}
