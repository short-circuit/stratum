//! AI tab: provider, credentials, models, local discovery and RAG.

use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use super::super::controls::{self, ChipTone, Palette};
use super::super::SettingsView;

/// The shipped AI providers.
const PROVIDERS: [(&str, &str); 8] = [
    ("settings.ollama_local", "ollama"),
    ("settings.openai", "openai"),
    ("settings.anthropic", "anthropic"),
    ("settings.google_ai", "google"),
    ("settings.z_ai", "zai"),
    ("settings.custom_openai_compatible", "custom"),
    ("settings.custom_openai_api", "custom-openai"),
    ("settings.custom_anthropic_api", "custom-anthropic"),
];

/// Capabilities a model can be tagged with.
const CAPABILITIES: [&str; 3] = ["chat", "embedding", "tts"];

/// Render the AI tab.
pub(super) fn render(
    view: &SettingsView,
    palette: &Palette,
    cx: &mut Context<SettingsView>,
) -> impl IntoElement {
    let provider = view.form.settings.ai.provider.clone();
    let shows_endpoint = matches!(
        provider.as_str(),
        "ollama" | "custom" | "zai" | "custom-openai" | "custom-anthropic"
    );
    let mut section = controls::section(t_global("settings.ai"), palette).child(
        controls::chip_row(view, "ai_provider", &provider, &PROVIDERS, palette, cx),
    );
    if view.form.settings.ai.api_key_from_env {
        section = section.child(env_notice(view, palette));
    }
    if shows_endpoint {
        section = section.child(controls::labeled_input(
            view,
            "ai_endpoint",
            t_global("settings.api_endpoint_url"),
            palette,
        ));
    }
    section = section
        .child(controls::labeled_input(
            view,
            "ai_api_key",
            t_global("settings.api_key"),
            palette,
        ))
        .child(controls::labeled_input(
            view,
            "ai_model",
            t_global("settings.default_chat_model"),
            palette,
        ))
        .child(div().flex().items_center().child(controls::button(
            "settings-fetch-models",
            if view.busy() == Some("fetch") {
                t_global("settings.fetching")
            } else {
                t_global("settings.fetch_available_models")
            },
            view.busy() == Some("fetch"),
            false,
            palette,
            cx.listener(|view, _, _, cx| view.fetch_models(cx)),
        )));
    if !view.available_models().is_empty() {
        section = section.child(model_list(view, palette, cx));
    }
    section = section.child(super::ai_discovery::render(view, palette, cx));
    section = section
        .child(controls::toggle_row(
            "settings-rag-enabled",
            t_global("settings.enable_rag"),
            None,
            view.form.settings.ai.rag_enabled,
            palette,
            cx.listener(|view, checked: &bool, _, cx| {
                view.set_flag("rag_enabled", *checked, cx);
            }),
        ))
        .when(view.form.settings.ai.rag_enabled, |element| {
            element.child(div().w(px(120.0)).child(controls::labeled_input(
                view,
                super::super::form::KEY_AI_RAG_CHUNKS,
                t_global("settings.chunks"),
                palette,
            )))
        })
        .child(controls::toggle_row(
            "settings-ai-gateway",
            t_global("settings.use_gateway_and_auth_from_llm"),
            None,
            view.form.settings.ai.use_llm_gateway_and_auth,
            palette,
            cx.listener(|view, checked: &bool, _, cx| {
                view.set_flag("ai_gateway", *checked, cx);
            }),
        ))
        .child(div().w(px(220.0)).child(controls::labeled_input(
            view,
            super::super::form::KEY_AI_EMBEDDING_DIMENSIONS,
            t_global("settings.embedding_dimensions_0_auto"),
            palette,
        )))
        .child(controls::hint(
            t_global("settings.leave_0_to_infer_the_vector_size_from_your_embed"),
            palette,
        ))
        .child(super::ai_voice::render(view, palette, cx));
    section
}

/// The provider environment-variable notice.
fn env_notice(view: &SettingsView, palette: &Palette) -> impl IntoElement {
    let provider = view.form.settings.ai.provider.as_str();
    let env_var = match provider {
        "openai" | "custom-openai" => "OPENAI_API_KEY",
        "anthropic" | "custom-anthropic" => "ANTHROPIC_API_KEY",
        "google" => "GOOGLE_API_KEY",
        _ => "",
    };
    div()
        .flex()
        .flex_col()
        .gap(px(2.0))
        .p(px(8.0))
        .rounded(px(6.0))
        .border_1()
        .border_color(palette.warning)
        .bg(palette.warning.opacity(0.1))
        .text_size(px(9.0))
        .text_color(palette.warning)
        .child(format!(
            "{} {} {env_var} {} {} {}",
            t_global("settings.security_notice"),
            t_global("settings.api_key_is_set_via_the"),
            t_global("settings.environment_variable"),
            t_global("settings.environment_variables_are_more_secure_than_storing_keys_in_c"),
            t_global("settings.to_use_a_different_key_unset_the_environment_variable_or_upd")
        ))
        .child(format!(
            "{} {env_var} {}{}",
            t_global("settings.api_key_loaded_from"),
            t_global("settings.environment_variable"),
            if view.form.settings.ai.api_key.is_some() {
                format!(
                    " {}",
                    t_global("settings.config_file_key_is_ignored_while_the_env_var_is_set")
                )
            } else {
                String::new()
            }
        ))
}

/// The fetched model list with capability chips.
fn model_list(
    view: &SettingsView,
    palette: &Palette,
    cx: &mut Context<SettingsView>,
) -> impl IntoElement {
    let mut list = div()
        .flex()
        .flex_col()
        .gap(px(3.0))
        .child(controls::caption(
            t_global("settings.models_click_to_enable_capabilities"),
            palette,
        ));
    for name in view.available_models() {
        let mut row = div()
            .flex()
            .items_center()
            .gap(px(6.0))
            .p(px(4.0))
            .border_1()
            .border_color(palette.border)
            .rounded(px(4.0))
            .child(
                div()
                    .flex_1()
                    .text_size(px(10.0))
                    .text_color(palette.foreground)
                    .child(name.clone()),
            );
        let caps = view
            .form
            .settings
            .ai
            .models
            .iter()
            .find(|model| &model.name == name)
            .map(|model| model.capabilities.clone())
            .unwrap_or_default();
        for capability in CAPABILITIES {
            let model_name = name.clone();
            let cap = capability.to_string();
            let selected = caps.iter().any(|entry| entry == capability);
            row = row.child(controls::chip(
                format!("settings-cap-{name}-{capability}"),
                capability.to_string(),
                selected,
                palette,
                cx.listener(move |view, _, _, cx| {
                    view.toggle_model_capability(&model_name, &cap, cx);
                }),
            ));
        }
        list = list.child(row);
    }
    list.child(controls::status_chip(
        t_global("settings.models_click_to_enable_capabilities"),
        ChipTone::Info,
        palette,
    ))
}
