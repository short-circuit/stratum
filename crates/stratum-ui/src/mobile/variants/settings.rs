//! Mobile Settings variant.
//!
//! Renders a collapsible tab accordion above the live
//! [`SettingsView`](crate::panels::settings::SettingsView) and hides the desktop
//! tab rail. Selecting a row switches the tab through the panel's public
//! `set_tab` action, so the accordion and the panel state stay in sync.

use gpui_kit::assets::IconName;
use gpui_kit::component::{ActiveTheme, Icon, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use crate::mobile::insets::MobileFrame;
use crate::mobile::nav::MIN_TOUCH_TARGET;
use crate::panels::settings::SettingsTab;
use crate::router::Panel;
use crate::shell::Shell;

/// Render the Settings surface for the mobile shell.
pub fn render(
    shell: &Shell,
    _window: &mut Window,
    _frame: &MobileFrame,
    cx: &mut Context<Shell>,
) -> AnyElement {
    let theme = cx.theme();
    let background = theme.colors.background;
    let foreground = theme.colors.foreground;
    let border = theme.colors.border;
    let panel = theme.colors.secondary;
    let accent = theme.colors.accent;
    let muted = theme.colors.muted_foreground;

    let Some(settings) = shell.settings_view().cloned() else {
        return super::placeholder(Panel::Settings, cx);
    };
    // The mobile variant owns the tab selector, so suppress the desktop rail.
    settings.update(cx, |view, _| view.set_mobile(true));

    let open = shell.mobile_settings_open();
    let header = div()
        .id("mobile-settings-accordion")
        .debug_selector(|| "mobile-settings-accordion".to_string())
        .flex()
        .items_center()
        .justify_between()
        .h(px(48.0))
        .min_h(px(MIN_TOUCH_TARGET))
        .px(px(16.0))
        .bg(panel)
        .border_b_1()
        .border_color(border)
        .cursor_pointer()
        .child(
            div()
                .text_size(px(14.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(foreground)
                .child(t_global("mobile_nav.settings")),
        )
        .child(
            Icon::new(if open {
                IconName::ChevronUp
            } else {
                IconName::ChevronDown
            })
            .with_size(px(18.0))
            .text_color(muted),
        )
        .on_click(cx.listener(|shell, _, _, cx| {
            shell.mobile_toggle_settings_accordion();
            cx.notify();
        }));

    let mut root = div()
        .flex()
        .flex_col()
        .size_full()
        .bg(background)
        .text_color(foreground)
        .child(header);

    if open {
        let mut rows = div()
            .flex()
            .flex_col()
            .bg(panel)
            .border_b_1()
            .border_color(border);
        for tab in SettingsTab::ALL {
            let active = settings.read(cx).tab() == tab;
            let color = if active { accent } else { foreground };
            rows = rows.child(
                div()
                    .id(format!("mobile-settings-tab-{}", tab.id()))
                    .debug_selector(move || format!("mobile-settings-tab-{}", tab.id()))
                    .flex()
                    .items_center()
                    .h(px(48.0))
                    .min_h(px(MIN_TOUCH_TARGET))
                    .px(px(16.0))
                    .cursor_pointer()
                    .text_color(color)
                    .child(div().text_size(px(14.0)).child(t_global(tab.label_key())))
                    .on_click(cx.listener(move |shell, _, window, cx| {
                        if let Some(settings) = shell.settings_view().cloned() {
                            settings.update(cx, |view, cx| view.set_tab(tab, window, cx));
                        }
                        shell.mobile_close_settings_accordion();
                        cx.notify();
                    })),
            );
        }
        root = root.child(rows);
    }

    root.child(div().flex_1().overflow_hidden().child(settings))
        .into_any_element()
}
