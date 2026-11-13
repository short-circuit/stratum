//! AI, voice and research probe actions for the native Settings panel.
//!
//! Split from [`super::actions`] so each file stays under the repository
//! length limit; the methods remain inherent [`SettingsView`] actions.

use gpui_kit::prelude::*;
use stratum_i18n::{t_global, tr_global};

use super::{PresetOutcome, SettingsView};

impl SettingsView {
    /// Fetch the models exposed by the configured AI endpoint.
    pub fn fetch_models(&mut self, cx: &mut Context<Self>) {
        self.busy = Some("fetch");
        match self.provider.fetch_models() {
            Ok(models) => {
                let count = models.len().to_string();
                self.available_models = models;
                self.busy = None;
                self.set_message(
                    tr_global("settings.found_models", &[("count", count.as_str())]),
                    cx,
                );
            }
            Err(error) => {
                self.busy = None;
                self.set_error(format!("{} {error}", t_global("settings.fetch_failed")), cx);
            }
        }
    }

    /// Scan the local network for OpenAI-compatible AI endpoints.
    pub fn discover_endpoints(&mut self, cx: &mut Context<Self>) {
        self.discovery_error = None;
        self.busy = Some("discover");
        match self.provider.discover_local_ai() {
            Ok(endpoints) => {
                self.form.settings.ai.last_discovered = endpoints.clone();
                self.discovered = endpoints;
                self.busy = None;
                cx.notify();
            }
            Err(error) => {
                self.busy = None;
                self.discovery_error = Some(tr_global(
                    "settings.scanning_failed",
                    &[("error", error.as_str())],
                ));
                cx.notify();
            }
        }
    }

    /// Adopt a discovered endpoint as the configured provider/endpoint pair.
    pub fn adopt_endpoint(&mut self, host: String, kind: String, cx: &mut Context<Self>) {
        self.form.settings.ai.provider = if kind == "ollama" {
            "ollama".to_string()
        } else {
            "custom".to_string()
        };
        self.form.settings.ai.endpoint = Some(host.clone());
        self.adopted_host = Some(host);
        cx.notify();
    }

    /// Toggle one capability flag on an available model.
    pub fn toggle_model_capability(
        &mut self,
        name: &str,
        capability: &str,
        cx: &mut Context<Self>,
    ) {
        let models = &mut self.form.settings.ai.models;
        match models.iter_mut().find(|model| model.name == name) {
            Some(model) => {
                if let Some(index) = model
                    .capabilities
                    .iter()
                    .position(|entry| entry == capability)
                {
                    model.capabilities.remove(index);
                } else {
                    model.capabilities.push(capability.to_string());
                }
            }
            None => models.push(super::AiModelDto {
                name: name.to_string(),
                capabilities: vec![capability.to_string()],
            }),
        }
        models.retain(|model| !model.capabilities.is_empty());
        cx.notify();
    }

    /// Probe the configured speech-to-text endpoint.
    pub fn test_stt(&mut self, cx: &mut Context<Self>) {
        self.busy = Some("stt");
        match self.provider.stt_test() {
            Ok(outcome) => {
                self.busy = None;
                self.stt_outcome = Some(outcome);
            }
            Err(error) => {
                self.busy = None;
                self.stt_outcome = Some(super::SttTestOutcome {
                    ok: false,
                    models: Vec::new(),
                    latency_ms: 0,
                    error: Some(error),
                });
            }
        }
        cx.notify();
    }

    /// Synthesize and play a short text-to-speech sample.
    pub fn test_tts(&mut self, cx: &mut Context<Self>) {
        self.busy = Some("tts");
        let text = t_global("settings.hello_this_is_a_test_of_stratum_text_to_speech");
        match self.provider.tts_test(&text) {
            Ok(outcome) => {
                self.busy = None;
                self.tts_outcome = Some(outcome);
            }
            Err(error) => {
                self.busy = None;
                self.set_error(error, cx);
            }
        }
        cx.notify();
    }

    /// Ping one SearXNG preset.
    pub fn test_preset(&mut self, url: String, cx: &mut Context<Self>) {
        self.preset_testing = Some(url.clone());
        let outcome = match self.provider.ping_search_instance(&url) {
            Ok(result) if result.ok => PresetOutcome {
                ok: true,
                detail: tr_global(
                    "settings.instance_reachable",
                    &[("latency_ms", result.latency_ms.to_string().as_str())],
                ),
            },
            Ok(result) => PresetOutcome {
                ok: false,
                detail: result
                    .error
                    .unwrap_or_else(|| t_global("settings.instance_unreachable")),
            },
            Err(error) => PresetOutcome {
                ok: false,
                detail: error,
            },
        };
        self.preset_outcomes.insert(url, outcome);
        self.preset_testing = None;
        cx.notify();
    }

    /// Adopt one SearXNG preset as the research endpoint.
    pub fn use_preset(&mut self, url: String, cx: &mut Context<Self>) {
        self.form.settings.research.searxng_endpoint = url;
        cx.notify();
    }
}
