//! Local AI endpoint discovery inside the AI tab.
//!
//! Mirrors the shipped `LocalDiscoverySection`: scan, show available endpoints
//! with their models, adopt one as the configured provider/endpoint, and edit
//! the extra-host probe list.

use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::{t_global, tr_global};

use super::super::controls::{self, ChipTone, Palette};
use super::super::SettingsView;

/// Render the local-discovery section.
pub(super) fn render(
    view: &SettingsView,
    palette: &Palette,
    cx: &mut Context<SettingsView>,
) -> impl IntoElement {
    let mut section = controls::section(t_global("settings.local_endpoints"), palette).child(
        div().flex().items_center().child(controls::button(
            "settings-discover-endpoints",
            if view.busy() == Some("discover") {
                t_global("settings.discovering")
            } else {
                t_global("settings.discover_local_models")
            },
            view.busy() == Some("discover"),
            false,
            palette,
            cx.listener(|view, _, _, cx| view.discover_endpoints(cx)),
        )),
    );
    if let Some(error) = view.discovery_error() {
        section = section.child(
            div()
                .text_size(px(9.0))
                .text_color(palette.danger)
                .child(error.to_string()),
        );
    }
    if view.discovered().is_empty() {
        section = section.child(controls::hint(
            t_global("settings.no_local_endpoints"),
            palette,
        ));
    } else {
        for endpoint in view.discovered() {
            section = section.child(endpoint_card(view, endpoint, palette, cx));
        }
    }
    section
        .child(controls::labeled_textarea(
            view,
            super::super::form::KEY_AI_EXTRA_HOSTS,
            t_global("settings.extra_local_hosts"),
            palette,
        ))
        .child(controls::hint(
            t_global("settings.extra_local_hosts_help"),
            palette,
        ))
}

/// One discovered endpoint card.
fn endpoint_card(
    view: &SettingsView,
    endpoint: &crate::provider::DiscoveredEndpoint,
    palette: &Palette,
    cx: &mut Context<SettingsView>,
) -> impl IntoElement {
    let available = endpoint.status == "available";
    let current = view.form.settings.ai.endpoint.as_deref() == Some(endpoint.host.as_str());
    let is_hint = available && !current;
    let status_label = if available {
        tr_global(
            "settings.models_count",
            &[("count", endpoint.models.len().to_string().as_str())],
        )
    } else {
        t_global("settings.endpoint_unreachable")
    };
    let mut card = div()
        .flex()
        .flex_col()
        .gap(px(2.0))
        .p(px(8.0))
        .rounded(px(6.0))
        .border_1()
        .border_color(palette.border)
        .child(
            div()
                .text_size(px(11.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(palette.foreground)
                .child(endpoint.name.clone()),
        )
        .child(
            div()
                .text_size(px(9.0))
                .text_color(palette.muted)
                .child(endpoint.host.clone()),
        )
        .child(controls::status_chip(
            status_label,
            if available {
                ChipTone::Success
            } else {
                ChipTone::Danger
            },
            palette,
        ));
    if available && !endpoint.models.is_empty() {
        card = card.child(
            div()
                .text_size(px(9.0))
                .text_color(palette.muted)
                .child(endpoint.models.join(", ")),
        );
    }
    if !available {
        if let Some(error) = endpoint.error.clone() {
            card = card.child(
                div()
                    .text_size(px(9.0))
                    .text_color(palette.danger)
                    .child(error),
            );
        }
    }
    if is_hint {
        card = card.child(controls::hint(
            t_global("settings.endpoints_are_hints"),
            palette,
        ));
    }
    if available {
        let host = endpoint.host.clone();
        let kind = endpoint.kind.clone();
        card = card.child(div().flex().items_center().child(controls::button(
            endpoint_button_id(endpoint),
            t_global("settings.use_this_endpoint"),
            false,
            false,
            palette,
            cx.listener(move |view, _, _, cx| {
                view.adopt_endpoint(host.clone(), kind.clone(), cx);
            }),
        )));
        if view.adopted_host() == Some(endpoint.host.as_str()) {
            card = card.child(controls::hint(
                t_global("settings.remember_to_save_endpoint"),
                palette,
            ));
        }
    }
    card
}

/// Stable debug id for an endpoint adopt button.
fn endpoint_button_id(endpoint: &crate::provider::DiscoveredEndpoint) -> &'static str {
    match endpoint.host.as_str() {
        "http://localhost:11434" => "settings-adopt-ollama",
        "http://127.0.0.1:1234" => "settings-adopt-lmstudio",
        _ => "settings-adopt-endpoint",
    }
}
