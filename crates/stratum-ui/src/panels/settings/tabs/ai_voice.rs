//! Voice sections of the AI tab: speech-to-text, text-to-speech and the
//! one-tap voice-capture toggle.

use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::{t_global, tr_global};

use super::super::controls::{self, ChipTone, Palette};
use super::super::SettingsView;

/// Render the STT, TTS and voice-capture sections.
pub(super) fn render(
    view: &SettingsView,
    palette: &Palette,
    cx: &mut Context<SettingsView>,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(12.0))
        .child(stt_section(view, palette, cx))
        .child(tts_section(view, palette, cx))
        .child(voice_section(view, palette, cx))
}

/// Speech-to-text configuration and probe.
fn stt_section(
    view: &SettingsView,
    palette: &Palette,
    cx: &mut Context<SettingsView>,
) -> impl IntoElement {
    let stt = &view.form.settings.stt;
    let mut section = controls::section(t_global("settings.voice_dictation_stt"), palette)
        .child(controls::labeled_input(
            view,
            "stt_endpoint",
            t_global("settings.transcription_endpoint_openai_compatible"),
            palette,
        ))
        .child(controls::labeled_input(
            view,
            "stt_api_key",
            t_global("settings.api_key_optional"),
            palette,
        ))
        .child(
            div()
                .flex()
                .items_end()
                .gap(px(6.0))
                .child(div().flex_1().child(controls::labeled_input(
                    view,
                    "stt_model",
                    t_global("settings.transcription_model"),
                    palette,
                )))
                .child(div().flex_1().child(controls::labeled_input(
                    view,
                    "stt_diarize_model",
                    t_global("settings.diarization_model"),
                    palette,
                ))),
        )
        .child(controls::labeled_input(
            view,
            "stt_language",
            t_global("settings.language_hint_optional"),
            palette,
        ))
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap(px(10.0))
                .child(controls::toggle_row(
                    "settings-stt-diarize",
                    t_global("dictation.diarize_speakers"),
                    None,
                    stt.diarize,
                    palette,
                    cx.listener(|view, checked: &bool, _, cx| {
                        view.set_flag("stt_diarize", *checked, cx);
                    }),
                ))
                .child(controls::toggle_row(
                    "settings-stt-summarize",
                    t_global("settings.auto_summarize"),
                    None,
                    stt.auto_summarize,
                    palette,
                    cx.listener(|view, checked: &bool, _, cx| {
                        view.set_flag("stt_auto_summarize", *checked, cx);
                    }),
                ))
                .child(controls::toggle_row(
                    "settings-stt-identify",
                    t_global("settings.auto_identify_voices"),
                    None,
                    stt.auto_identify,
                    palette,
                    cx.listener(|view, checked: &bool, _, cx| {
                        view.set_flag("stt_auto_identify", *checked, cx);
                    }),
                )),
        )
        .child(controls::toggle_row(
            "settings-stt-gateway",
            t_global("settings.use_gateway_and_auth_from_llm"),
            None,
            stt.use_llm_gateway_and_auth,
            palette,
            cx.listener(|view, checked: &bool, _, cx| {
                view.set_flag("stt_gateway", *checked, cx);
            }),
        ))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(controls::button(
                    "settings-stt-test",
                    if view.busy() == Some("stt") {
                        t_global("settings.testing")
                    } else {
                        t_global("settings.test_connection")
                    },
                    view.busy() == Some("stt"),
                    false,
                    palette,
                    cx.listener(|view, _, _, cx| view.test_stt(cx)),
                )),
        );
    if let Some(outcome) = view.stt_outcome() {
        let (label, tone) = if outcome.ok {
            (
                format!(
                    "{} {}",
                    t_global("settings.connected"),
                    tr_global(
                        "settings.models_count",
                        &[("count", outcome.models.len().to_string().as_str())],
                    )
                ),
                ChipTone::Success,
            )
        } else {
            (
                outcome
                    .error
                    .clone()
                    .unwrap_or_else(|| t_global("settings.ping_failed")),
                ChipTone::Danger,
            )
        };
        section = section.child(controls::status_chip(label, tone, palette));
    }
    section
}

/// Text-to-speech configuration and probe.
fn tts_section(
    view: &SettingsView,
    palette: &Palette,
    cx: &mut Context<SettingsView>,
) -> impl IntoElement {
    let tts = &view.form.settings.tts;
    let mut section = controls::section(t_global("settings.text_to_speech"), palette)
        .child(controls::labeled_input(
            view,
            "tts_endpoint",
            t_global("settings.tts_endpoint_optional"),
            palette,
        ))
        .child(controls::hint(
            t_global("settings.uses_the_ai_endpoint_when_empty"),
            palette,
        ))
        .child(controls::labeled_input(
            view,
            "tts_api_key",
            t_global("settings.api_key_optional"),
            palette,
        ))
        .child(
            div()
                .flex()
                .items_end()
                .gap(px(6.0))
                .child(div().flex_1().child(controls::labeled_input(
                    view,
                    "tts_voice",
                    t_global("settings.voice"),
                    palette,
                )))
                .child(div().flex_1().child(controls::labeled_input(
                    view,
                    "tts_format",
                    t_global("settings.format"),
                    palette,
                ))),
        )
        .child(div().w(px(160.0)).child(controls::labeled_input(
            view,
            super::super::form::KEY_TTS_SPEED,
            t_global("settings.speed"),
            palette,
        )))
        .child(controls::hint(
            t_global("settings.playback_speed_multiplier_0_25_4_0"),
            palette,
        ))
        .child(controls::toggle_row(
            "settings-tts-gateway",
            t_global("settings.use_gateway_and_auth_from_llm"),
            None,
            tts.use_llm_gateway_and_auth,
            palette,
            cx.listener(|view, checked: &bool, _, cx| {
                view.set_flag("tts_gateway", *checked, cx);
            }),
        ))
        .child(div().flex().items_center().child(controls::button(
            "settings-tts-test",
            if view.busy() == Some("tts") {
                t_global("settings.generating")
            } else {
                t_global("settings.test_play_voice")
            },
            view.busy() == Some("tts"),
            false,
            palette,
            cx.listener(|view, _, _, cx| view.test_tts(cx)),
        )));
    if let Some(outcome) = view.tts_outcome() {
        section = section.child(controls::status_chip(
            format!(
                "{} {:.1} KiB",
                t_global("settings.generating_played"),
                outcome.byte_len as f32 / 1024.0
            ),
            ChipTone::Success,
            palette,
        ));
    }
    section
}

/// One-tap voice-capture toggle.
fn voice_section(
    view: &SettingsView,
    palette: &Palette,
    cx: &mut Context<SettingsView>,
) -> impl IntoElement {
    controls::section(t_global("voice_capture.settings_toggle_label"), palette).child(
        controls::toggle_row(
            "settings-voice-enabled",
            t_global("voice_capture.enable"),
            Some(t_global("voice_capture.settings_target_today")),
            view.form.settings.voice.enabled,
            palette,
            cx.listener(|view, checked: &bool, _, cx| {
                view.set_flag("voice_enabled", *checked, cx);
            }),
        ),
    )
}
