//! Bottom navigation, More sheet and the mobile back stack.
//!
//! The table is plain data so the coverage contract is testable without a
//! window: every [`Panel`] appears exactly once across the four tabs and the
//! ten More-sheet rows. Rendering lives here too so [`super::mod`] stays a thin
//! frame composer.
//!
//! Touch targets follow the platform minimum: every tab and every sheet row is
//! at least [`MIN_TOUCH_TARGET`] logical pixels tall. The bar reserves the
//! bottom safe-area inset (home indicator/gesture bar) so it never overlaps the
//! content column.

use gpui_kit::assets::IconName;
use gpui_kit::component::{ActiveTheme, Icon, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use crate::router::Panel;
use crate::shell::Shell;

use super::insets::MobileFrame;

/// Minimum touch target in logical pixels.
pub const MIN_TOUCH_TARGET: f32 = 44.0;
/// Height of the bottom navigation row (excludes the safe-area padding).
pub const NAV_ITEM_HEIGHT: f32 = 56.0;
/// Height of one More-sheet row.
pub const SHEET_ROW_HEIGHT: f32 = 48.0;
/// Maximum number of back-stack entries kept (oldest dropped first).
pub const MAX_BACK_STACK: usize = 32;

/// One bottom-navigation tab.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MobileTabDef {
    /// Stable tab id.
    pub id: &'static str,
    /// Localized label key.
    pub label_key: &'static str,
    /// Shipped icon.
    pub icon: IconName,
    /// Panel opened by the tab; `None` for the More sheet.
    pub target: Option<Panel>,
}

/// The four bottom-navigation tabs, in the shipped mobile order.
pub const MOBILE_TABS: [MobileTabDef; 4] = [
    MobileTabDef {
        id: "journal",
        label_key: "mobile_nav.journal",
        icon: IconName::Calendar,
        target: Some(Panel::Journal),
    },
    MobileTabDef {
        id: "search",
        label_key: "mobile_nav.search",
        icon: IconName::Search,
        target: Some(Panel::Search),
    },
    MobileTabDef {
        id: "graph",
        label_key: "mobile_nav.graph",
        icon: IconName::Network,
        target: Some(Panel::Graph),
    },
    MobileTabDef {
        id: "more",
        label_key: "mobile_nav.more",
        icon: IconName::Ellipsis,
        target: None,
    },
];

/// One More-sheet row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MoreEntryDef {
    /// Panel opened by the row.
    pub panel: Panel,
    /// Localized label key.
    pub label_key: &'static str,
    /// Shipped icon.
    pub icon: IconName,
}

/// Every panel that is reachable from the More sheet.
///
/// The Page row uses `mobile_layout.pages`, matching the shipped mobile layout
/// label. The remaining rows reuse the shipped `mobile_nav.*` keys.
pub const MORE_ENTRIES: [MoreEntryDef; 10] = [
    MoreEntryDef {
        panel: Panel::Page,
        label_key: "mobile_layout.pages",
        icon: IconName::FileText,
    },
    MoreEntryDef {
        panel: Panel::Query,
        label_key: "mobile_nav.query",
        icon: IconName::Code,
    },
    MoreEntryDef {
        panel: Panel::AskNotes,
        label_key: "mobile_nav.ask_notes",
        icon: IconName::Sparkles,
    },
    MoreEntryDef {
        panel: Panel::Templates,
        label_key: "mobile_nav.templates",
        icon: IconName::FileText,
    },
    MoreEntryDef {
        panel: Panel::Flashcards,
        label_key: "mobile_nav.flashcards",
        icon: IconName::CircleQuestionMark,
    },
    MoreEntryDef {
        panel: Panel::Kanban,
        label_key: "mobile_nav.kanban",
        icon: IconName::ListChecks,
    },
    MoreEntryDef {
        panel: Panel::Reminders,
        label_key: "mobile_nav.reminders",
        icon: IconName::AlarmClock,
    },
    MoreEntryDef {
        panel: Panel::Whiteboards,
        label_key: "mobile_nav.whiteboards",
        icon: IconName::PenTool,
    },
    MoreEntryDef {
        panel: Panel::Plugins,
        label_key: "mobile_nav.plugins",
        icon: IconName::Puzzle,
    },
    MoreEntryDef {
        panel: Panel::Settings,
        label_key: "mobile_nav.settings",
        icon: IconName::Settings,
    },
];

/// One back-stack entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NavEntry {
    /// Panel the user visited.
    pub panel: Panel,
}

/// The mobile navigation state: the back stack and the More-sheet flag.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MobileNav {
    stack: Vec<NavEntry>,
    more_open: bool,
}

impl MobileNav {
    /// Whether the More sheet is open.
    pub fn more_open(&self) -> bool {
        self.more_open
    }

    /// Open the More sheet.
    pub fn open_more(&mut self) {
        self.more_open = true;
    }

    /// Close the More sheet.
    pub fn close_more(&mut self) {
        self.more_open = false;
    }

    /// The back stack, oldest first.
    pub fn stack(&self) -> &[NavEntry] {
        &self.stack
    }

    /// The top of the back stack.
    pub fn top(&self) -> Option<NavEntry> {
        self.stack.last().copied()
    }

    /// Push a visited panel onto the back stack.
    pub fn push(&mut self, panel: Panel) {
        if self.stack.len() >= MAX_BACK_STACK {
            self.stack.remove(0);
        }
        self.stack.push(NavEntry { panel });
    }

    /// Pop the top of the back stack.
    pub fn pop(&mut self) -> Option<NavEntry> {
        self.stack.pop()
    }

    /// Clear the back stack.
    pub fn clear(&mut self) {
        self.stack.clear();
    }
}

/// The tab definition for `panel`, if a bottom tab opens it.
pub fn tab_for_panel(panel: Panel) -> Option<MobileTabDef> {
    MOBILE_TABS
        .into_iter()
        .find(|tab| tab.target == Some(panel))
}

/// Render the bottom navigation bar.
///
/// The returned element reserves `frame.bottom` below the 56 px row so the bar
/// sits above the home indicator and the content column never overlaps it.
pub fn bottom_nav(shell: &Shell, frame: &MobileFrame, cx: &mut Context<Shell>) -> AnyElement {
    let theme = cx.theme();
    let background = theme.colors.secondary;
    let border = theme.colors.border;
    let foreground = theme.colors.foreground;
    let muted = theme.colors.muted_foreground;
    let accent = theme.colors.accent;
    let active = shell.active_panel();
    let more_open = shell.mobile_nav().more_open();

    let tabs = MOBILE_TABS.into_iter().map(|tab| {
        let selected = match tab.target {
            Some(panel) => panel == active,
            None => more_open,
        };
        let color = if selected { accent } else { muted };
        let label = t_global(tab.label_key);
        div()
            .id(SharedString::from(format!("mobile-tab-{}", tab.id)))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(3.0))
            .flex_1()
            .h(px(NAV_ITEM_HEIGHT))
            .min_h(px(MIN_TOUCH_TARGET))
            .cursor_pointer()
            .text_color(color)
            .child(Icon::new(tab.icon).with_size(px(22.0)).text_color(color))
            .child(div().text_size(px(11.0)).child(label))
            .on_click(cx.listener(move |shell, _, _, cx| {
                shell.mobile_activate_tab(tab);
                cx.notify();
            }))
    });

    div()
        .flex()
        .flex_col()
        .flex_shrink_0()
        .bg(background)
        .border_t_1()
        .border_color(border)
        .pb(px(frame.bottom))
        .text_color(foreground)
        .child(
            div()
                .flex()
                .items_center()
                .justify_around()
                .h(px(NAV_ITEM_HEIGHT))
                .children(tabs),
        )
        .into_any_element()
}

/// Render the More sheet when it is open.
///
/// The sheet is anchored above the bottom navigation bar and lists every panel
/// that is not one of the three primary tabs. A scrim closes it on tap.
pub fn more_sheet(
    shell: &Shell,
    frame: &MobileFrame,
    cx: &mut Context<Shell>,
) -> Option<AnyElement> {
    if !shell.mobile_nav().more_open() {
        return None;
    }
    let theme = cx.theme();
    let background = theme.colors.background;
    let panel = theme.colors.secondary;
    let border = theme.colors.border;
    let foreground = theme.colors.foreground;
    let muted = theme.colors.muted_foreground;
    let accent = theme.colors.accent;

    let rows = MORE_ENTRIES.into_iter().map(|entry| {
        let label = t_global(entry.label_key);
        let selected = shell.active_panel() == entry.panel;
        div()
            .id(SharedString::from(format!(
                "mobile-more-{}",
                entry.panel.id()
            )))
            .flex()
            .items_center()
            .gap(px(12.0))
            .h(px(SHEET_ROW_HEIGHT))
            .min_h(px(MIN_TOUCH_TARGET))
            .px(px(16.0))
            .cursor_pointer()
            .text_color(if selected { accent } else { foreground })
            .child(Icon::new(entry.icon).with_size(px(20.0)))
            .child(div().text_size(px(15.0)).child(label))
            .on_click(cx.listener(move |shell, _, _, cx| {
                shell.mobile_activate_more(entry.panel);
                cx.notify();
            }))
    });

    let sheet = div()
        .absolute()
        .left(px(frame.left))
        .right(px(frame.right))
        .bottom(px(frame.bottom + NAV_ITEM_HEIGHT))
        .max_h(px(560.0))
        .flex()
        .flex_col()
        .rounded_t(px(14.0))
        .bg(panel)
        .border_1()
        .border_color(border)
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .px(px(16.0))
                .py(px(12.0))
                .border_b_1()
                .border_color(border)
                .child(
                    div()
                        .text_size(px(16.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(foreground)
                        .child(t_global("mobile_nav.more")),
                )
                .child(
                    div()
                        .id("mobile-more-close")
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(px(MIN_TOUCH_TARGET))
                        .cursor_pointer()
                        .text_color(muted)
                        .child(Icon::new(IconName::X).with_size(px(18.0)))
                        .on_click(cx.listener(|shell, _, _, cx| {
                            shell.mobile_close_more();
                            cx.notify();
                        })),
                ),
        )
        .children(rows);

    let scrim = div()
        .id("mobile-more-scrim")
        .absolute()
        .top(px(0.0))
        .left(px(0.0))
        .size_full()
        .bg(background.opacity(0.55))
        .on_click(cx.listener(|shell, _, _, cx| {
            shell.mobile_close_more();
            cx.notify();
        }));

    Some(
        div()
            .absolute()
            .top(px(0.0))
            .left(px(0.0))
            .size_full()
            .child(scrim)
            .child(sheet)
            .into_any_element(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::core::prelude::v1::test;

    #[test]
    fn every_panel_is_reachable_exactly_once() {
        let mut seen = std::collections::HashSet::new();
        for tab in MOBILE_TABS {
            if let Some(panel) = tab.target {
                assert!(seen.insert(panel), "duplicate tab target: {}", panel.id());
            }
        }
        for entry in MORE_ENTRIES {
            assert!(
                seen.insert(entry.panel),
                "duplicate More entry: {}",
                entry.panel.id()
            );
        }
        for panel in Panel::ALL {
            assert!(seen.contains(&panel), "unreachable panel: {}", panel.id());
        }
        assert_eq!(seen.len(), Panel::ALL.len());
    }

    #[test]
    fn every_touch_target_meets_the_minimum() {
        const { assert!(NAV_ITEM_HEIGHT >= MIN_TOUCH_TARGET) };
        const { assert!(SHEET_ROW_HEIGHT >= MIN_TOUCH_TARGET) };
    }

    #[test]
    fn tabs_and_sheet_labels_are_catalog_keys() {
        for tab in MOBILE_TABS {
            assert!(tab.label_key.starts_with("mobile_nav."));
        }
        for entry in MORE_ENTRIES {
            assert!(
                entry.label_key.starts_with("mobile_nav.")
                    || entry.label_key.starts_with("mobile_layout.")
            );
        }
    }

    #[test]
    fn back_stack_pushes_and_pops_in_order() {
        let mut nav = MobileNav::default();
        assert_eq!(nav.top(), None);
        nav.push(Panel::Journal);
        nav.push(Panel::Search);
        assert_eq!(
            nav.top(),
            Some(NavEntry {
                panel: Panel::Search
            })
        );
        assert_eq!(nav.stack().len(), 2);
        assert_eq!(
            nav.pop(),
            Some(NavEntry {
                panel: Panel::Search
            })
        );
        assert_eq!(
            nav.top(),
            Some(NavEntry {
                panel: Panel::Journal
            })
        );
        assert_eq!(
            nav.pop(),
            Some(NavEntry {
                panel: Panel::Journal
            })
        );
        assert_eq!(nav.pop(), None);
    }

    #[test]
    fn back_stack_drops_the_oldest_entry_at_the_cap() {
        let mut nav = MobileNav::default();
        for index in 0..(MAX_BACK_STACK + 4) {
            nav.push(if index % 2 == 0 {
                Panel::Journal
            } else {
                Panel::Search
            });
        }
        assert_eq!(nav.stack().len(), MAX_BACK_STACK);
    }

    #[test]
    fn more_sheet_opens_and_closes() {
        let mut nav = MobileNav::default();
        assert!(!nav.more_open());
        nav.open_more();
        assert!(nav.more_open());
        nav.close_more();
        assert!(!nav.more_open());
    }

    #[test]
    fn tab_lookup_finds_the_primary_panels_only() {
        assert_eq!(
            tab_for_panel(Panel::Journal).map(|tab| tab.id),
            Some("journal")
        );
        assert_eq!(
            tab_for_panel(Panel::Search).map(|tab| tab.id),
            Some("search")
        );
        assert_eq!(tab_for_panel(Panel::Graph).map(|tab| tab.id), Some("graph"));
        assert_eq!(tab_for_panel(Panel::Settings), None);
    }
}
