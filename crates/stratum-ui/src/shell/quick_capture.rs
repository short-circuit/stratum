//! Quick Capture dialog: tray/global-shortcut capture into the journal or inbox.
//!
//! The shipped webview opened this dialog from a tray menu item, the global
//! shortcut or the `quick-capture:open` event. The native shell does the same
//! through [`Shell::request_quick_capture`]: the platform event queue sets a
//! pending request and the next render opens the dialog with the right mode.
//!
//! Text capture mirrors the shipped dialog: Enter saves, Escape discards, an
//! empty capture closes without writing. Voice mode starts a recording against
//! today's journal, then stops and transcribes it.
//!
//! The dialog keeps its text in [`QuickCapture::text`] (updated from the input's
//! change events) so every action stays a plain `&mut Shell` method that the
//! tests can drive without a window.

use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::ActiveTheme;
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::{t_global, tr_global};

use crate::shell::{Shell, Toast};

/// The Quick Capture dialog state.
#[derive(Default)]
pub struct QuickCapture {
    input: Option<Entity<InputState>>,
    subscription: Option<Subscription>,
    /// Whether the dialog is visible. `pub(super)` so the shell tests can seed
    /// the dialog without a window.
    pub(super) open: bool,
    /// Whether the dialog was opened in voice mode (seeded by the tests).
    pub(super) voice: bool,
    recording: bool,
    /// The typed capture text, updated from the input's change events and
    /// seeded directly by the shell tests.
    pub(super) text: String,
    status: Option<String>,
    error: Option<String>,
}

impl QuickCapture {
    /// Whether the dialog is visible.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Whether the dialog was opened in voice mode.
    pub fn is_voice(&self) -> bool {
        self.voice
    }

    /// Whether a recording is running.
    pub fn is_recording(&self) -> bool {
        self.recording
    }

    /// The text currently typed into the dialog.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The last status line (a saved/voice confirmation).
    pub fn status(&self) -> Option<&str> {
        self.status.as_deref()
    }

    /// The last error line.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    fn reset(&mut self) {
        self.open = false;
        self.voice = false;
        self.recording = false;
        self.text.clear();
        self.error = None;
    }
}

impl Shell {
    /// Request the Quick Capture dialog from a tray item, the global shortcut or
    /// a start flag.
    ///
    /// The dialog is opened on the next render because creating an input entity
    /// needs a window; the request survives until then.
    pub fn request_quick_capture(&mut self, voice: bool) {
        self.pending_quick_capture = Some(voice);
    }

    /// Open the dialog now.
    ///
    /// `voice` starts a recording immediately (shipped "Voice Capture" menu
    /// item); text mode focuses the input.
    pub(super) fn open_quick_capture(
        &mut self,
        voice: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.pending_quick_capture = None;
        self.quick_capture.open = true;
        self.quick_capture.voice = voice;
        self.quick_capture.recording = false;
        self.quick_capture.error = None;
        self.quick_capture.status = None;
        self.quick_capture.text.clear();

        if self.quick_capture.input.is_none() {
            let input = cx.new(|cx| InputState::new(window, cx));
            let subscription = cx.subscribe(
                &input,
                move |this: &mut Shell, input, event: &InputEvent, cx| match event {
                    InputEvent::Change => {
                        this.quick_capture.text = input.read(cx).value().to_string();
                        cx.notify();
                    }
                    InputEvent::PressEnter { shift, .. } if !shift => {
                        this.quick_capture.text = input.read(cx).value().to_string();
                        this.save_quick_capture();
                        cx.notify();
                    }
                    _ => {}
                },
            );
            self.quick_capture.input = Some(input);
            self.quick_capture.subscription = Some(subscription);
        }
        if let Some(input) = self.quick_capture.input.clone() {
            input.update(cx, |input, cx| input.set_value("", window, cx));
            let handle = input.read(cx).focus_handle(cx);
            window.focus(&handle, cx);
        }

        if voice {
            self.start_voice_capture();
        }
        cx.notify();
    }

    /// Close the dialog without saving.
    ///
    /// A running recording is stopped first so the audio device is released.
    pub fn close_quick_capture(&mut self) {
        if self.quick_capture.recording {
            let _ = self.provider.voice_stop();
        }
        self.quick_capture.reset();
    }

    /// Close the dialog if it is open. Returns whether it was open.
    pub fn dismiss_quick_capture(&mut self) -> bool {
        if !self.quick_capture.is_open() {
            return false;
        }
        self.close_quick_capture();
        true
    }

    /// Save the typed capture.
    ///
    /// An empty capture closes the dialog without writing, exactly like the
    /// shipped dialog. A successful save closes the dialog and shows the
    /// localized "captured to <page>" toast.
    pub fn save_quick_capture(&mut self) {
        let text = self.quick_capture.text.trim().to_string();
        if text.is_empty() {
            self.close_quick_capture();
            return;
        }
        match self.provider.quick_capture_save(text) {
            Ok(saved) => {
                let message =
                    tr_global("quickcapture.saved", &[("page", saved.page_path.as_str())]);
                self.close_quick_capture();
                self.toaster.toast = Some(Toast::message(message));
            }
            Err(error) => {
                self.quick_capture.error = Some(error);
                self.quick_capture.recording = false;
            }
        }
    }

    /// Start the voice capture flow.
    pub(super) fn start_voice_capture(&mut self) {
        if !self.provider.stt_configured() {
            self.quick_capture.error = Some(t_global("voice_capture.configure_stt_prompt"));
            self.quick_capture.recording = false;
            return;
        }
        let page_path = match self.provider.ensure_today_journal() {
            Ok(path) => path,
            Err(error) => {
                self.quick_capture.error = Some(error);
                return;
            }
        };
        match self.provider.voice_start(&page_path) {
            Ok(_) => {
                self.quick_capture.recording = true;
                self.quick_capture.error = None;
            }
            Err(error) => {
                self.quick_capture.error = Some(error);
                self.quick_capture.recording = false;
            }
        }
    }

    /// Stop the recording and transcribe it into today's journal.
    pub(super) fn stop_voice_capture(&mut self) {
        let page_path = match self.provider.ensure_today_journal() {
            Ok(path) => path,
            Err(error) => {
                self.quick_capture.error = Some(error);
                return;
            }
        };
        let (recording_path, _duration) = match self.provider.voice_stop() {
            Ok(stop) => stop,
            Err(error) => {
                self.quick_capture.error = Some(error);
                self.quick_capture.recording = false;
                return;
            }
        };
        match self.provider.voice_transcribe(&recording_path, &page_path) {
            Ok(_) => {
                let message = t_global("quickcapture.voice_done");
                self.close_quick_capture();
                self.toaster.toast = Some(Toast::message(message));
            }
            Err(error) => {
                self.quick_capture.error = Some(error);
                self.quick_capture.recording = false;
            }
        }
    }

    /// Render the dialog overlay.
    pub(super) fn render_quick_capture(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let capture = &self.quick_capture;
        let target = self.quick_capture_target();
        let status = capture.status.clone();
        let error = capture.error.clone();
        let recording = capture.is_recording();

        div()
            .id("quick-capture-backdrop")
            .debug_selector(|| "quick-capture-backdrop".to_string())
            .absolute()
            .inset_0()
            .flex()
            .items_start()
            .justify_center()
            .pt(px(96.0))
            .bg(gpui::hsla(0.0, 0.0, 0.0, 0.45))
            .on_click(cx.listener(|this, _, _, cx| {
                if !this.quick_capture.is_recording() {
                    this.close_quick_capture();
                    cx.notify();
                }
            }))
            .child(
                div()
                    .id("quick-capture-dialog")
                    .debug_selector(|| "quick-capture-dialog".to_string())
                    .flex()
                    .flex_col()
                    .gap(px(10.0))
                    .w(px(460.0))
                    .p(px(18.0))
                    .rounded(px(10.0))
                    .bg(theme.colors.background)
                    .border_1()
                    .border_color(theme.colors.border)
                    .on_click(cx.listener(|_, _, _, cx| cx.stop_propagation()))
                    .child(
                        div()
                            .text_size(px(15.0))
                            .text_color(theme.colors.foreground)
                            .child(t_global("quickcapture.title")),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(theme.colors.muted_foreground)
                            .child(format!(
                                "{} {}",
                                t_global("quickcapture.target_label"),
                                target
                            )),
                    )
                    .when_some(status, |element, status| {
                        element.child(
                            div()
                                .text_size(px(12.0))
                                .text_color(theme.colors.foreground)
                                .child(status),
                        )
                    })
                    .when_some(error, |element, error| {
                        element.child(
                            div()
                                .text_size(px(12.0))
                                .text_color(theme.colors.danger)
                                .child(error),
                        )
                    })
                    .when(recording, |element| {
                        element.child(
                            div()
                                .text_size(px(13.0))
                                .text_color(theme.colors.foreground)
                                .child(t_global("quickcapture.recording")),
                        )
                    })
                    .when(!recording, |element| {
                        element.child(div().h(px(88.0)).child(
                            match self.quick_capture.input.as_ref() {
                                Some(input) => {
                                    Input::new(input).appearance(false).into_any_element()
                                }
                                None => div().into_any_element(),
                            },
                        ))
                    })
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(theme.colors.muted_foreground)
                                    .child(t_global("quickcapture.hint_save")),
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap(px(8.0))
                                    .child(
                                        div()
                                            .id("quick-capture-cancel")
                                            .debug_selector(|| "quick-capture-cancel".to_string())
                                            .px(px(10.0))
                                            .py(px(4.0))
                                            .rounded(px(5.0))
                                            .text_size(px(12.0))
                                            .text_color(theme.colors.muted_foreground)
                                            .cursor_pointer()
                                            .child(t_global("common.cancel"))
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.close_quick_capture();
                                                cx.notify();
                                            })),
                                    )
                                    .child(
                                        div()
                                            .id("quick-capture-save")
                                            .debug_selector(|| "quick-capture-save".to_string())
                                            .px(px(12.0))
                                            .py(px(4.0))
                                            .rounded(px(5.0))
                                            .bg(theme.colors.accent)
                                            .text_color(theme.colors.accent_foreground)
                                            .text_size(px(12.0))
                                            .cursor_pointer()
                                            .child(t_global("common.save"))
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.save_quick_capture();
                                                cx.notify();
                                            })),
                                    )
                                    .when(recording, |element| {
                                        element.child(
                                            div()
                                                .id("quick-capture-stop")
                                                .debug_selector(|| "quick-capture-stop".to_string())
                                                .px(px(12.0))
                                                .py(px(4.0))
                                                .rounded(px(5.0))
                                                .bg(theme.colors.accent)
                                                .text_color(theme.colors.accent_foreground)
                                                .text_size(px(12.0))
                                                .cursor_pointer()
                                                .child(t_global("quickcapture.stop"))
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.stop_voice_capture();
                                                    cx.notify();
                                                })),
                                        )
                                    }),
                            ),
                    ),
            )
    }

    /// The localized capture-target label.
    fn quick_capture_target(&self) -> String {
        let target = self
            .provider
            .settings()
            .map(|settings| settings.tray.capture_target)
            .unwrap_or_else(|_| "journal".to_string());
        if target == "inbox" {
            t_global("quickcapture.target_inbox")
        } else {
            t_global("quickcapture.target_journal")
        }
    }
}
