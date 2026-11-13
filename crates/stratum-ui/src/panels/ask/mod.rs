//! Native Ask Notes panel.
//!
//! [`AskView`] ports the shipped desktop `AskNotesPanel`: a multi-line question
//! field with Enter submission, an Ask action, a Clear action and the answer
//! card with read-aloud plus cited sources. Provider calls stay behind the
//! [`crate::provider::PanelData`] seam; the panel only owns presentation and the
//! pending requests the shell consumes.
//!
//! Citations are informational in the shipped UI — clicking one does not
//! navigate — so [`AskView`] never sets a pending navigation. Read-aloud goes
//! through [`crate::provider::ShellData::speak`], the same method the editor
//! uses, so the shell has one playback path.
//!
//! [`ASK_I18N_KEYS`] is the release gate list for every string this surface
//! renders.

use std::sync::Arc;

use gpui_kit::component::input::{InputEvent, Textarea, TextareaState};
use gpui_kit::component::ActiveTheme;
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use crate::panels::PanelRequests;
use crate::provider::{AskAnswer, ShellData};

pub mod answer;

/// Every translation key the Ask Notes surface renders.
///
/// The contract test walks this list against all eight catalogs, so adding a
/// visible string without a catalog entry fails the build instead of leaking a
/// raw key into the UI.
pub const ASK_I18N_KEYS: &[&str] = &[
    "ask_notes.ask",
    "ask_notes.ask_your_notes",
    "ask_notes.ask_your_notes_e_g_what_did_i_decide_about_the_h",
    "ask_notes.clear",
    "ask_notes.get_answers_grounded_in_your_vault_with_cited_sources",
    "ask_notes.no_matching_notes_found_the_answer_is_based_on_general_knowl",
    "ask_notes.playing",
    "ask_notes.read_aloud_failed",
    "ask_notes.searching_your_notes",
    "ask_notes.sources",
    "editor.read_aloud",
];

/// Deterministic start state for the Ask Notes panel.
///
/// `STRATUM_ASK=<question>` uses this to submit a question during construction
/// so the capture harness renders a real answer without synthesising keystrokes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AskStart {
    /// Question to ask immediately after construction.
    pub question: Option<String>,
}

/// The Ask Notes panel entity.
pub struct AskView {
    provider: Arc<dyn ShellData>,
    input: Entity<TextareaState>,
    _input_subscription: Subscription,
    question: String,
    loading: bool,
    answer: Option<AskAnswer>,
    error: Option<String>,
    speak_error: Option<String>,
    speaking: bool,
    pending_navigation: Option<String>,
    pending_toast: Option<String>,
}

impl AskView {
    /// Build the panel and ask `start.question` when present.
    pub fn new(
        provider: Arc<dyn ShellData>,
        start: AskStart,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let placeholder = t_global("ask_notes.ask_your_notes_e_g_what_did_i_decide_about_the_h");
        let initial = start.question.clone().unwrap_or_default();
        let input = cx.new(move |cx| {
            TextareaState::new(window, cx)
                .placeholder(placeholder)
                .default_value(initial)
        });
        let subscription = cx.subscribe(
            &input,
            |this: &mut Self, input, event: &InputEvent, cx| match event {
                InputEvent::Change => {
                    this.question = input.read(cx).value().to_string();
                    cx.notify();
                }
                InputEvent::PressEnter { shift, .. } if !shift => this.ask(cx),
                _ => {}
            },
        );
        let mut view = Self {
            provider,
            input,
            _input_subscription: subscription,
            question: start.question.clone().unwrap_or_default(),
            loading: false,
            answer: None,
            error: None,
            speak_error: None,
            speaking: false,
            pending_navigation: None,
            pending_toast: None,
        };
        if start.question.is_some() {
            view.ask(cx);
        }
        view
    }

    /// The current question text.
    pub fn question(&self) -> &str {
        &self.question
    }

    /// The current answer, if any.
    pub fn answer(&self) -> Option<&AskAnswer> {
        self.answer.as_ref()
    }

    /// Whether the provider query is in flight.
    pub fn loading(&self) -> bool {
        self.loading
    }

    /// The provider error, if any.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// The read-aloud error, if any.
    pub fn speak_error(&self) -> Option<&str> {
        self.speak_error.as_deref()
    }

    /// Whether the answer is currently being spoken.
    pub fn speaking(&self) -> bool {
        self.speaking
    }

    /// Whether the Ask button should accept a click.
    pub fn can_ask(&self) -> bool {
        !self.loading && !self.question.trim().is_empty()
    }

    /// Ask the provider the current question.
    pub fn ask(&mut self, cx: &mut Context<Self>) {
        let question = self.question.trim().to_string();
        if question.is_empty() || self.loading {
            return;
        }
        self.loading = true;
        self.error = None;
        self.speak_error = None;
        self.answer = None;
        match self.provider.ask_notes(&question) {
            Ok(answer) => self.answer = Some(answer),
            Err(error) => self.error = Some(error),
        }
        self.loading = false;
        cx.notify();
    }

    /// Clear the answer, errors and read-aloud failure (shipped `reset`).
    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.answer = None;
        self.error = None;
        self.speak_error = None;
        cx.notify();
    }

    /// Speak the current answer through the provider.
    pub fn speak_answer(&mut self, cx: &mut Context<Self>) {
        let Some(answer) = self.answer.as_ref() else {
            return;
        };
        if answer.answer.trim().is_empty() || self.speaking {
            return;
        }
        self.speaking = true;
        self.speak_error = None;
        let text = answer.answer.clone();
        match self.provider.speak(text) {
            Ok(()) => {}
            Err(error) => self.speak_error = Some(error),
        }
        self.speaking = false;
        cx.notify();
    }

    /// The question input entity.
    pub fn input(&self) -> &Entity<TextareaState> {
        &self.input
    }
}

impl PanelRequests for AskView {
    fn pending_navigation(&self) -> Option<&str> {
        self.pending_navigation.as_deref()
    }

    fn clear_pending_navigation(&mut self) {
        self.pending_navigation = None;
    }

    fn pending_toast(&self) -> Option<&str> {
        self.pending_toast.as_deref()
    }

    fn take_pending_toast(&mut self) -> Option<String> {
        self.pending_toast.take()
    }
}

impl Render for AskView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.colors.background)
            .text_color(theme.colors.foreground)
            .child(header(self, cx))
            .child(
                div()
                    .id("ask-body")
                    .flex()
                    .flex_col()
                    .flex_1()
                    .overflow_y_scroll()
                    .p(px(24.0))
                    .gap(px(10.0))
                    .child(question_row(self, cx))
                    .child(answer::body(self, cx)),
            )
    }
}

/// The title row with the Clear action.
fn header(view: &AskView, cx: &mut Context<AskView>) -> impl IntoElement {
    let theme = cx.theme();
    let muted = theme.colors.muted_foreground;
    let foreground = theme.colors.foreground;
    let border = theme.colors.border;
    div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .px(px(24.0))
        .py(px(12.0))
        .border_b_1()
        .border_color(border)
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_size(px(18.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(foreground)
                        .child(t_global("ask_notes.ask_your_notes")),
                )
                .when(view.answer().is_some(), |element| {
                    element.child(
                        div()
                            .id("ask-clear")
                            .debug_selector(|| "ask-clear".to_string())
                            .px(px(10.0))
                            .py(px(4.0))
                            .rounded(px(6.0))
                            .text_size(px(12.0))
                            .text_color(muted)
                            .cursor_pointer()
                            .child(t_global("ask_notes.clear"))
                            .on_click(cx.listener(|view, _, _, cx| view.clear(cx))),
                    )
                }),
        )
        .child(div().text_size(px(12.0)).text_color(muted).child(t_global(
            "ask_notes.get_answers_grounded_in_your_vault_with_cited_sources",
        )))
}

/// The question field and Ask action.
fn question_row(view: &AskView, cx: &mut Context<AskView>) -> impl IntoElement {
    let theme = cx.theme();
    let accent = theme.colors.accent;
    let accent_foreground = theme.colors.accent_foreground;
    let enabled = view.can_ask();
    div()
        .flex()
        .items_start()
        .gap(px(8.0))
        .child(
            div()
                .id("ask-input")
                .debug_selector(|| "ask-input".to_string())
                .flex_1()
                .child(Textarea::new(&view.input).h(px(64.0)).w_full()),
        )
        .child(
            div()
                .id("ask-submit")
                .debug_selector(|| "ask-submit".to_string())
                .px(px(14.0))
                .py(px(6.0))
                .rounded(px(6.0))
                .bg(if enabled { accent } else { accent.opacity(0.4) })
                .text_color(accent_foreground)
                .text_size(px(12.0))
                .when(enabled, |element| {
                    element
                        .cursor_pointer()
                        .on_click(cx.listener(|view, _, _, cx| view.ask(cx)))
                })
                .child(t_global("ask_notes.ask")),
        )
}
