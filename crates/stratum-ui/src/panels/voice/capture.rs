//! The gpui voice-capture entity.
//!
//! [`VoiceCapture`] drives the [`crate::provider::ShellData`] voice seam and
//! renders the shared header control. Stage transitions live in
//! [`super::stage`]; this module owns the async start/stop/transcribe flows and
//! the button chrome.

use std::sync::Arc;
use std::time::Duration;

use gpui_kit::assets::IconName;
use gpui_kit::component::{ActiveTheme, Icon, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use super::stage::{format_timer, VoiceStage};
use crate::provider::ShellData;

enum VoiceOutcome {
    Inserted(usize),
    NeedsConfig,
    Fallback(String),
}

/// The voice-capture button entity.
pub struct VoiceCapture {
    provider: Arc<dyn ShellData>,
    page_path: String,
    stage: VoiceStage,
    recording_path: Option<String>,
    elapsed: u64,
    error: Option<String>,
    inserted: usize,
    reload_requested: bool,
    generation: u64,
}

impl VoiceCapture {
    /// Create a capture control targeting `page_path`.
    pub fn new(
        page_path: impl Into<String>,
        provider: Arc<dyn ShellData>,
        _cx: &mut Context<Self>,
    ) -> Self {
        Self {
            provider,
            page_path: page_path.into(),
            stage: VoiceStage::Idle,
            recording_path: None,
            elapsed: 0,
            error: None,
            inserted: 0,
            reload_requested: false,
            generation: 0,
        }
    }

    /// Retarget the capture at another page (for example after a day change).
    pub fn set_page_path(&mut self, page_path: impl Into<String>) {
        self.page_path = page_path.into();
    }

    /// The current pipeline stage.
    pub fn stage(&self) -> VoiceStage {
        self.stage
    }

    /// Elapsed recording seconds.
    pub fn elapsed(&self) -> u64 {
        self.elapsed
    }

    /// The localized stage label.
    pub fn stage_label(&self) -> String {
        t_global(self.stage.label_key())
    }

    /// The last error message, if any.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Whether the parent should reload the editor to show the inserted memo.
    ///
    /// The flag is consumed so one insert triggers exactly one reload.
    pub fn take_reload_request(&mut self) -> bool {
        let requested = self.reload_requested;
        self.reload_requested = false;
        requested
    }

    /// Start recording through the provider.
    pub fn start(&mut self, cx: &mut Context<Self>) {
        if !self.stage.can_start() {
            return;
        }
        self.generation = self.generation.wrapping_add(1);
        let generation = self.generation;
        self.stage = VoiceStage::Recording;
        self.elapsed = 0;
        self.error = None;
        self.recording_path = None;
        self.inserted = 0;
        let provider = self.provider.clone();
        let page_path = self.page_path.clone();
        cx.spawn(
            async move |this: WeakEntity<VoiceCapture>, cx: &mut AsyncApp| {
                let started = cx
                    .background_executor()
                    .spawn(async move { provider.voice_start(&page_path) })
                    .await;
                let _ = this.update(cx, |view, cx| {
                    if view.generation != generation {
                        return;
                    }
                    match started {
                        Ok(path) => {
                            view.recording_path = Some(path);
                            view.stage = VoiceStage::Recording;
                        }
                        Err(error) => {
                            view.stage = VoiceStage::Error;
                            view.error = Some(error);
                        }
                    }
                    cx.notify();
                });
            },
        )
        .detach();
        self.spawn_ticker(cx);
    }

    /// Stop recording, then transcribe or insert a clip placeholder.
    pub fn stop(&mut self, cx: &mut Context<Self>) {
        if !self.stage.can_stop() {
            return;
        }
        self.generation = self.generation.wrapping_add(1);
        let generation = self.generation;
        self.stage = VoiceStage::Transcribing;
        self.elapsed = 0;
        let provider = self.provider.clone();
        let page_path = self.page_path.clone();
        cx.spawn(
            async move |this: WeakEntity<VoiceCapture>, cx: &mut AsyncApp| {
                let outcome = cx
                    .background_executor()
                    .spawn(async move { run_stop_pipeline(provider, page_path) })
                    .await;
                let _ = this.update(cx, |view, cx| {
                    if view.generation != generation {
                        return;
                    }
                    view.apply_outcome(outcome);
                    cx.notify();
                });
            },
        )
        .detach();
    }

    /// Retry transcription for the last recording.
    pub fn retry(&mut self, cx: &mut Context<Self>) {
        let Some(recording_path) = self.recording_path.clone() else {
            return;
        };
        self.generation = self.generation.wrapping_add(1);
        let generation = self.generation;
        self.stage = VoiceStage::Transcribing;
        self.error = None;
        let provider = self.provider.clone();
        let page_path = self.page_path.clone();
        cx.spawn(
            async move |this: WeakEntity<VoiceCapture>, cx: &mut AsyncApp| {
                let result = cx
                    .background_executor()
                    .spawn(async move { provider.voice_transcribe(&recording_path, &page_path) })
                    .await;
                let _ = this.update(cx, |view, cx| {
                    if view.generation != generation {
                        return;
                    }
                    match result {
                        Ok(count) => {
                            view.inserted = count;
                            view.stage = VoiceStage::Inserted;
                            view.reload_requested = true;
                        }
                        Err(error) => {
                            view.stage = VoiceStage::Fallback;
                            view.error = Some(error);
                        }
                    }
                    cx.notify();
                });
            },
        )
        .detach();
    }

    /// Dismiss a terminal stage and return to idle.
    pub fn dismiss(&mut self, cx: &mut Context<Self>) {
        self.generation = self.generation.wrapping_add(1);
        self.stage = VoiceStage::Idle;
        self.recording_path = None;
        self.elapsed = 0;
        self.error = None;
        self.inserted = 0;
        cx.notify();
    }

    /// Apply the background stop/transcribe result.
    fn apply_outcome(&mut self, outcome: VoiceOutcome) {
        match outcome {
            VoiceOutcome::Inserted(count) => {
                self.inserted = count;
                self.stage = VoiceStage::Inserted;
                self.reload_requested = true;
            }
            VoiceOutcome::NeedsConfig => {
                self.stage = VoiceStage::NeedsConfig;
                self.reload_requested = true;
            }
            VoiceOutcome::Fallback(error) => {
                self.stage = VoiceStage::Fallback;
                self.error = Some(error);
                self.reload_requested = true;
            }
        }
    }

    /// Spawn the one-second elapsed ticker for a recording.
    fn spawn_ticker(&mut self, cx: &mut Context<Self>) {
        let generation = self.generation;
        cx.spawn(
            async move |this: WeakEntity<VoiceCapture>, cx: &mut AsyncApp| loop {
                cx.background_executor().timer(Duration::from_secs(1)).await;
                let keep = this
                    .update(cx, |view, cx| {
                        if view.generation != generation || view.stage != VoiceStage::Recording {
                            return false;
                        }
                        view.elapsed = view.elapsed.saturating_add(1);
                        cx.notify();
                        true
                    })
                    .unwrap_or(false);
                if !keep {
                    break;
                }
            },
        )
        .detach();
    }
}

/// Stop the recording and run the shipped fallback/transcription policy.
fn run_stop_pipeline(provider: Arc<dyn ShellData>, page_path: String) -> VoiceOutcome {
    let Ok((recording_path, seconds)) = provider.voice_stop() else {
        return VoiceOutcome::Fallback("recording stop failed".to_string());
    };
    if !provider.stt_configured() {
        let _ = provider.voice_insert_clip(&recording_path, &page_path, seconds);
        return VoiceOutcome::NeedsConfig;
    }
    match provider.voice_transcribe(&recording_path, &page_path) {
        Ok(count) => VoiceOutcome::Inserted(count),
        Err(error) => {
            let _ = provider.voice_insert_clip(&recording_path, &page_path, seconds);
            VoiceOutcome::Fallback(error)
        }
    }
}

impl Render for VoiceCapture {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let muted = theme.colors.muted_foreground;
        let accent = theme.colors.accent;
        let accent_foreground = theme.colors.accent_foreground;
        let danger = theme.colors.danger;
        let label = self.stage_label();
        let icon = match self.stage {
            VoiceStage::Recording => IconName::Square,
            VoiceStage::Transcribing => IconName::LoaderCircle,
            VoiceStage::Inserted => IconName::CircleCheckBig,
            VoiceStage::NeedsConfig | VoiceStage::Fallback | VoiceStage::Error => {
                IconName::CircleAlert
            }
            VoiceStage::Idle => IconName::Mic,
        };
        let recording = self.stage == VoiceStage::Recording;
        let terminal = self.stage.can_start() && self.stage != VoiceStage::Idle;
        let can_retry = self.stage.can_retry();

        div()
            .id("voice-capture")
            .flex()
            .items_center()
            .gap(px(6.0))
            .px(px(8.0))
            .py(px(4.0))
            .rounded(px(6.0))
            .bg(if recording { danger } else { accent })
            .text_color(accent_foreground)
            .text_size(px(12.0))
            .cursor_pointer()
            .on_click(cx.listener(|this, _, _, cx| {
                if this.stage.can_stop() {
                    this.stop(cx);
                } else if this.stage.can_start() {
                    this.start(cx);
                }
            }))
            .child(Icon::new(icon).with_size(px(14.0)))
            .child(div().child(label))
            .when(recording, |element| {
                element.child(div().text_color(muted).child(format_timer(self.elapsed)))
            })
            .when(can_retry, |element| {
                element.child(
                    div()
                        .id("voice-retry")
                        .px(px(4.0))
                        .rounded(px(4.0))
                        .bg(accent)
                        .text_color(accent_foreground)
                        .child(t_global("voice_capture.retry_transcription"))
                        .on_click(cx.listener(|this, _, _, cx| this.retry(cx))),
                )
            })
            .when(terminal, |element| {
                element.child(
                    div()
                        .id("voice-dismiss")
                        .px(px(4.0))
                        .rounded(px(4.0))
                        .text_color(muted)
                        .child(t_global("voice_capture.cancel"))
                        .on_click(cx.listener(|this, _, _, cx| this.dismiss(cx))),
                )
            })
    }
}
