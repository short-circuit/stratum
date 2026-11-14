//! Desktop shell layout: sidebar, header, routed panel, error bar and overlays.
//!
//! [`Shell::render`](crate::shell::Shell) dispatches here when the mobile layout
//! is off. The mobile counterpart lives in [`crate::mobile::render`]; both share
//! the per-frame lifecycle (`Shell::begin_frame`), the panel entities and the
//! provider seam.

use gpui_kit::component::ActiveTheme;
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::panels::{panel_owns_header, panel_view, PanelEntities};
use crate::router::Panel;
use crate::shell::chrome::{error_bar, header, toast_overlay};
use crate::shell::{sidebar, Shell};

impl Shell {
    /// Render the desktop shell chrome.
    pub(crate) fn render_desktop(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let background = theme.colors.background;
        let foreground = theme.colors.foreground;
        // The page, journal, search and query panels render their own header
        // (title, path chip, mode controls), matching the shipped app where the
        // routed panel owns the whole content column. The chrome header stays
        // for panels that are still placeholders so navigation remains legible.
        let panel_owns_header = panel_owns_header(self.active)
            && match self.active {
                Panel::Page => self.page.is_some(),
                Panel::Journal => self.journal.is_some(),
                Panel::Search => self.search.is_some(),
                Panel::Query => self.query.is_some(),
                Panel::AskNotes => self.ask.is_some(),
                Panel::Templates => self.templates.is_some(),
                Panel::Flashcards => self.flashcards.is_some(),
                Panel::Reminders => self.reminders.is_some(),
                Panel::Plugins => self.plugins.is_some(),
                Panel::Graph => self.graph.is_some(),
                Panel::Whiteboards => self.whiteboards.is_some(),
                Panel::Kanban => self.kanban.is_some(),
                _ => false,
            };

        div()
            .relative()
            .flex()
            .size_full()
            .bg(background)
            .text_color(foreground)
            .on_action(cx.listener(|this, _: &crate::ToggleReadMode, _window, cx| {
                this.toggle_read_mode();
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &crate::DismissOverlay, _window, cx| {
                this.dismiss_overlay();
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &crate::ToggleSidebar, _window, cx| {
                this.toggle_sidebar();
                cx.notify();
            }))
            .on_drop::<ExternalPaths>(cx.listener(|this, paths: &ExternalPaths, _window, cx| {
                // A note dropped anywhere outside a block opens that note;
                // drops on a block stay with the editor's attachment import.
                this.handle_drop(&paths.0, cx);
            }))
            .child(sidebar::sidebar(self, cx))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .h_full()
                    .overflow_hidden()
                    .when(!panel_owns_header, |element| {
                        element.child(header(self, cx))
                    })
                    .when(self.error.is_some(), |element| {
                        element.child(error_bar(self, cx))
                    })
                    .child(div().flex_1().overflow_hidden().child(panel_view(
                        self.active,
                        self.current_page.as_deref(),
                        PanelEntities {
                            page: self.page.clone(),
                            journal: self.journal.clone(),
                            search: self.search.clone(),
                            query: self.query.clone(),
                            ask: self.ask.clone(),
                            templates: self.templates.clone(),
                            flashcards: self.flashcards.clone(),
                            reminders: self.reminders.clone(),
                            plugins: self.plugins.clone(),
                            graph: self.graph.clone(),
                            whiteboards: self.whiteboards.clone(),
                            settings: self.settings.clone(),
                            kanban: self.kanban.clone(),
                        },
                        cx,
                    ))),
            )
            .when(self.toaster.toast.is_some(), |element| {
                element.child(toast_overlay(self, cx))
            })
            .when(self.quick_capture.is_open(), |element| {
                element.child(self.render_quick_capture(cx))
            })
            .when(self.onboarding.open, |element| {
                element.child(super::onboarding_overlay(self, cx))
            })
            .into_any_element()
    }
}
