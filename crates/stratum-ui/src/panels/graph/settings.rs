//! Graph settings panel: visibility toggles, force sliders, 3D switch and the
//! node cap selector.
//!
//! The controls mirror the shipped `GraphSettings` panel. Slider values are
//! bound to [`SliderState`] entities created by [`SettingsControls::new`];
//! changing any value updates [`GraphView`] settings and rebuilds the cached
//! frame. Persisted settings are loaded from and saved through
//! [`crate::provider::PanelData::graph_settings`] /
//! [`crate::provider::PanelData::save_graph_settings`]; the node cap stays in
//! memory exactly like the shipped panel.

use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::slider::{Slider, SliderEvent, SliderState, SliderValue};
use gpui_kit::component::ActiveTheme;
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use super::model::GraphSettings;
use super::GraphView;

/// Slider entities bound to the force settings.
pub(super) struct SettingsControls {
    /// Charge strength slider.
    pub charge: Entity<SliderState>,
    /// Link distance slider.
    pub distance: Entity<SliderState>,
    /// Alpha decay slider.
    pub alpha: Entity<SliderState>,
    /// Velocity decay slider.
    pub friction: Entity<SliderState>,
    /// Link curvature slider.
    pub curvature: Entity<SliderState>,
    /// Slider event subscriptions (kept alive for the entity lifetime).
    pub _subscriptions: Vec<Subscription>,
}

impl SettingsControls {
    /// Create the slider entities and subscribe their change events.
    pub(super) fn new(settings: &GraphSettings, cx: &mut Context<GraphView>) -> Self {
        let charge = cx.new(|_| {
            SliderState::new()
                .min(-30.0)
                .max(0.0)
                .step(1.0)
                .default_value(settings.charge_strength as f32)
        });
        let distance = cx.new(|_| {
            SliderState::new()
                .min(10.0)
                .max(120.0)
                .step(5.0)
                .default_value(settings.link_distance as f32)
        });
        let alpha = cx.new(|_| {
            SliderState::new()
                .min(0.01)
                .max(0.3)
                .step(0.01)
                .default_value(settings.alpha_decay as f32)
        });
        let friction = cx.new(|_| {
            SliderState::new()
                .min(0.05)
                .max(0.95)
                .step(0.05)
                .default_value(settings.velocity_decay as f32)
        });
        let curvature = cx.new(|_| {
            SliderState::new()
                .min(0.0)
                .max(0.5)
                .step(0.05)
                .default_value(settings.link_curvature as f32)
        });
        let subscriptions = vec![
            cx.subscribe(&charge, |view, _, event: &SliderEvent, cx| {
                if let SliderEvent::Change(SliderValue::Single(value)) = event {
                    view.set_charge(*value, cx);
                }
            }),
            cx.subscribe(&distance, |view, _, event: &SliderEvent, cx| {
                if let SliderEvent::Change(SliderValue::Single(value)) = event {
                    view.set_link_distance(*value, cx);
                }
            }),
            cx.subscribe(&alpha, |view, _, event: &SliderEvent, cx| {
                if let SliderEvent::Change(SliderValue::Single(value)) = event {
                    view.set_alpha_decay(*value, cx);
                }
            }),
            cx.subscribe(&friction, |view, _, event: &SliderEvent, cx| {
                if let SliderEvent::Change(SliderValue::Single(value)) = event {
                    view.set_friction(*value, cx);
                }
            }),
            cx.subscribe(&curvature, |view, _, event: &SliderEvent, cx| {
                if let SliderEvent::Change(SliderValue::Single(value)) = event {
                    view.set_curvature(*value, cx);
                }
            }),
        ];
        Self {
            charge,
            distance,
            alpha,
            friction,
            curvature,
            _subscriptions: subscriptions,
        }
    }
}

/// Render the settings panel.
pub(super) fn settings(view: &GraphView, cx: &mut Context<GraphView>) -> impl IntoElement {
    let theme = cx.theme();
    div()
        .flex()
        .flex_col()
        .gap(px(10.0))
        .px(px(16.0))
        .py(px(12.0))
        .border_b_1()
        .border_color(theme.colors.border)
        .bg(theme.colors.background)
        .child(toggle_row(view, cx))
        .child(slider_grid(view, cx))
        .child(cap_row(view, cx))
}

/// Visibility and 3D toggles.
fn toggle_row(view: &GraphView, cx: &mut Context<GraphView>) -> impl IntoElement {
    let theme = cx.theme();
    let muted = theme.colors.muted_foreground;
    div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap(px(16.0))
        .child(toggle(
            "graph-show-connected",
            t_global("graph.connected_notes"),
            view.settings.show_connected,
            cx.listener(|view, checked: &bool, _, cx| {
                view.set_show_connected(*checked, cx);
            }),
        ))
        .child(toggle(
            "graph-show-orphaned",
            t_global("graph.orphaned_notes"),
            view.settings.show_orphaned,
            cx.listener(|view, checked: &bool, _, cx| {
                view.set_show_orphaned(*checked, cx);
            }),
        ))
        .child(toggle(
            "graph-show-tags",
            t_global("graph.tags_on_hover"),
            view.settings.show_tags,
            cx.listener(|view, checked: &bool, _, cx| {
                view.set_show_tags(*checked, cx);
            }),
        ))
        .child(toggle(
            "graph-use-3d",
            t_global("graph.enable_3d_view"),
            view.settings.use_3d,
            cx.listener(|view, checked: &bool, _, cx| {
                view.set_use_3d(*checked, cx);
            }),
        ))
        .child(
            div()
                .text_size(px(10.0))
                .text_color(muted)
                .child(t_global("graph.graph_settings")),
        )
}

/// A labelled checkbox.
fn toggle(
    id: &'static str,
    label: String,
    checked: bool,
    listener: impl Fn(&bool, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    Checkbox::new(id)
        .checked(checked)
        .label(label)
        .on_click(listener)
}

/// The force slider grid.
fn slider_grid(view: &GraphView, cx: &mut Context<GraphView>) -> impl IntoElement {
    let settings = &view.settings;
    div()
        .flex()
        .flex_wrap()
        .gap(px(12.0))
        .child(slider_row(
            "graph-charge",
            t_global("graph.repulsion"),
            format!("{:.0}", settings.charge_strength),
            &view.controls.charge,
            cx,
        ))
        .child(slider_row(
            "graph-distance",
            t_global("graph.link_distance"),
            format!("{:.0}px", settings.link_distance),
            &view.controls.distance,
            cx,
        ))
        .child(slider_row(
            "graph-alpha",
            t_global("graph.alpha_decay"),
            format!("{:.2}", settings.alpha_decay),
            &view.controls.alpha,
            cx,
        ))
        .child(slider_row(
            "graph-friction",
            t_global("graph.friction"),
            format!("{:.2}", settings.velocity_decay),
            &view.controls.friction,
            cx,
        ))
        .child(slider_row(
            "graph-curvature",
            t_global("graph.curvature"),
            format!("{:.2}", settings.link_curvature),
            &view.controls.curvature,
            cx,
        ))
}

/// One labelled slider row.
fn slider_row(
    id: &'static str,
    label: String,
    display: String,
    state: &Entity<SliderState>,
    cx: &mut Context<GraphView>,
) -> impl IntoElement {
    let theme = cx.theme();
    div()
        .id(id)
        .debug_selector(move || id.to_string())
        .flex()
        .flex_col()
        .gap(px(4.0))
        .w(px(220.0))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_size(px(10.0))
                        .text_color(theme.colors.muted_foreground)
                        .child(label),
                )
                .child(
                    div()
                        .text_size(px(10.0))
                        .text_color(theme.colors.foreground)
                        .child(display),
                ),
        )
        .child(Slider::new(state).horizontal().w_full())
}

/// The node cap selector chips.
fn cap_row(view: &GraphView, cx: &mut Context<GraphView>) -> impl IntoElement {
    let (accent, accent_foreground, background, muted, border) = {
        let theme = cx.theme();
        (
            theme.colors.accent,
            theme.colors.accent_foreground,
            theme.colors.background,
            theme.colors.muted_foreground,
            theme.colors.border,
        )
    };
    let mut row = div().flex().flex_wrap().items_center().gap(px(6.0)).child(
        div()
            .text_size(px(10.0))
            .text_color(muted)
            .child(t_global("graph.node_cap")),
    );
    for cap in [500_usize, 1000, 2000, 5000, 10_000, 0] {
        let label = if cap == 0 {
            t_global("graph.unlimited_auto_cap_for_large_vaults")
        } else {
            cap.to_string()
        };
        row = row.child(cap_chip(
            cap,
            label,
            view.settings.node_cap == cap,
            accent,
            accent_foreground,
            background,
            muted,
            border,
            cx,
        ));
    }
    row
}

/// One node-cap chip.
#[allow(clippy::too_many_arguments)]
fn cap_chip(
    cap: usize,
    label: String,
    selected: bool,
    accent: Hsla,
    accent_foreground: Hsla,
    background: Hsla,
    muted: Hsla,
    border: Hsla,
    cx: &mut Context<GraphView>,
) -> impl IntoElement {
    let (chip_bg, chip_fg) = if selected {
        (accent, accent_foreground)
    } else {
        (background, muted)
    };
    div()
        .id(("graph-cap", cap))
        .debug_selector(move || format!("graph-cap-{cap}"))
        .px(px(8.0))
        .py(px(3.0))
        .rounded(px(6.0))
        .border_1()
        .border_color(border)
        .bg(chip_bg)
        .text_color(chip_fg)
        .text_size(px(10.0))
        .cursor_pointer()
        .child(label)
        .on_click(cx.listener(move |view, _, _, cx| view.set_node_cap(cap, cx)))
}
