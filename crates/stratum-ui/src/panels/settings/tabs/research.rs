//! Research tab: SearXNG endpoint, result cap, depth and the public-instance
//! presets with inline reachability probes.

use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use super::super::controls::{self, ChipTone, Palette};
use super::super::SettingsView;

/// Shipped public SearXNG presets.
const PRESETS: [&str; 4] = [
    "https://searx.be",
    "https://search.bus-hit.me",
    "https://priv.au",
    "https://searx.tiekoetter.com",
];

/// Render the research tab.
pub(super) fn render(
    view: &SettingsView,
    palette: &Palette,
    cx: &mut Context<SettingsView>,
) -> impl IntoElement {
    let mut presets = div().flex().flex_col().gap(px(4.0));
    for url in PRESETS {
        presets = presets.child(preset_row(view, url, palette, cx));
    }

    controls::section(t_global("settings.research"), palette)
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(10.0))
                .w(px(480.0))
                .child(controls::labeled_input(
                    view,
                    "research_endpoint",
                    t_global("settings.searxng_endpoint_url"),
                    palette,
                ))
                .child(controls::hint(
                    t_global("settings.the_url_of_your_searxng_instance_e_g_http_localh"),
                    palette,
                ))
                .child(controls::labeled_input(
                    view,
                    "research_max_results",
                    t_global("settings.max_results_per_search"),
                    palette,
                ))
                .child(controls::labeled_input(
                    view,
                    "research_max_depth",
                    t_global("settings.research_depth_search_read_cycles"),
                    palette,
                )),
        )
        .child(controls::section(
            t_global("settings.search_presets"),
            palette,
        ))
        .child(controls::hint(
            t_global("settings.search_presets_help"),
            palette,
        ))
        .child(presets)
}

/// One preset row with test and adopt actions.
fn preset_row(
    view: &SettingsView,
    url: &'static str,
    palette: &Palette,
    cx: &mut Context<SettingsView>,
) -> impl IntoElement {
    let selected = view.form.settings.research.searxng_endpoint == url;
    let testing = view.preset_testing() == Some(url);
    let outcome = view.preset_outcome(url);
    let mut row = div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap(px(6.0))
        .p(px(6.0))
        .rounded(px(6.0))
        .border_1()
        .border_color(if selected {
            palette.accent
        } else {
            palette.border
        })
        .child(
            div()
                .flex_1()
                .text_size(px(10.0))
                .text_color(if selected {
                    palette.foreground
                } else {
                    palette.muted
                })
                .child(url),
        );
    if let Some(outcome) = outcome {
        let tone = if outcome.ok {
            ChipTone::Success
        } else {
            ChipTone::Danger
        };
        row = row.child(controls::status_chip(outcome.detail.clone(), tone, palette));
    }
    let url_owned = url.to_string();
    let use_url = url.to_string();
    row.child(controls::button(
        preset_test_id(url),
        if testing {
            t_global("settings.testing")
        } else {
            t_global("settings.test_instance")
        },
        testing,
        false,
        palette,
        cx.listener(move |view, _, _, cx| view.test_preset(url_owned.clone(), cx)),
    ))
    .child(controls::button(
        preset_use_id(url),
        t_global("settings.use_this_instance"),
        false,
        true,
        palette,
        cx.listener(move |view, _, _, cx| view.use_preset(use_url.clone(), cx)),
    ))
}

/// Stable debug id for a preset test button.
fn preset_test_id(url: &str) -> &'static str {
    match url {
        "https://searx.be" => "settings-preset-test-searx-be",
        "https://search.bus-hit.me" => "settings-preset-test-bus-hit",
        "https://priv.au" => "settings-preset-test-priv-au",
        _ => "settings-preset-test-tiekoetter",
    }
}

/// Stable debug id for a preset adopt button.
fn preset_use_id(url: &str) -> &'static str {
    match url {
        "https://searx.be" => "settings-preset-use-searx-be",
        "https://search.bus-hit.me" => "settings-preset-use-bus-hit",
        "https://priv.au" => "settings-preset-use-priv-au",
        _ => "settings-preset-use-tiekoetter",
    }
}
