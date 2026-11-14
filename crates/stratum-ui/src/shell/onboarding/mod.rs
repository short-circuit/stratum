//! First-run onboarding wizard.
//!
//! The shipped web app opened a four-step wizard on a brand-new vault: welcome
//! and vault selection, optional starter templates, an optional sample vault,
//! and a finish step that opens the Quick Start guide. The native shell ports the
//! same flow as a modal overlay driven by [`crate::provider::SettingsData`]:
//!
//! 1. Welcome — shows the active vault path and opens the native folder picker.
//! 2. Templates — optional starter templates (`install_starter_templates`).
//! 3. Sample vault — optional linked example notes (`install_starter_sample`).
//! 4. Finish — persists `onboarding_pending = false` and opens the Quick Start
//!    page (creating it when the vault does not have it yet).
//!
//! The wizard is only shown when the provider reports pending onboarding, so a
//! populated vault never sees it. "Settings → Developer → Reset onboarding"
//! re-arms it.
//!
//! [`ONBOARDING_I18N_KEYS`] is the release gate list for every string this
//! surface renders.

use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::{t_global, tr_global};

use super::Shell;

mod render;

pub use render::overlay;

/// The Quick Start page the finish step opens.
pub const QUICK_START_PAGE: &str = "pages/Quick Start.md";

/// Every translation key the onboarding wizard renders.
pub const ONBOARDING_I18N_KEYS: &[&str] = &[
    "onboarding.back",
    "onboarding.choose_vault_folder",
    "onboarding.error",
    "onboarding.finish",
    "onboarding.finish_hint",
    "onboarding.finish_title",
    "onboarding.installing",
    "onboarding.next",
    "onboarding.open_quick_start",
    "onboarding.sample_vault",
    "onboarding.sample_vault_hint",
    "onboarding.start_with_templates",
    "onboarding.step",
    "onboarding.templates_hint",
    "onboarding.welcome_subtitle",
    "onboarding.welcome_title",
    "settings.browse",
    "settings.vault_path",
];

/// One wizard step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// Welcome and vault selection.
    Welcome,
    /// Optional starter templates.
    Templates,
    /// Optional sample vault.
    Sample,
    /// Finish and open the guide.
    Finish,
}

impl Step {
    /// The number shown in the "Step n of 4" label.
    fn number(self) -> usize {
        match self {
            Step::Welcome => 1,
            Step::Templates => 2,
            Step::Sample => 3,
            Step::Finish => 4,
        }
    }

    /// The next step, if any.
    fn next(self) -> Option<Step> {
        match self {
            Step::Welcome => Some(Step::Templates),
            Step::Templates => Some(Step::Sample),
            Step::Sample => Some(Step::Finish),
            Step::Finish => None,
        }
    }

    /// The previous step, if any.
    fn previous(self) -> Option<Step> {
        match self {
            Step::Welcome => None,
            Step::Templates => Some(Step::Welcome),
            Step::Sample => Some(Step::Templates),
            Step::Finish => Some(Step::Sample),
        }
    }

    /// Parse a step from the capture flags.
    pub fn from_env(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "welcome" => Some(Step::Welcome),
            "templates" => Some(Step::Templates),
            "sample" => Some(Step::Sample),
            "finish" => Some(Step::Finish),
            _ => None,
        }
    }
}

/// Wizard state owned by the shell.
pub struct Onboarding {
    /// Whether the overlay is visible.
    pub(super) open: bool,
    /// The current step.
    pub(super) step: Step,
    /// The vault path shown in the welcome step.
    pub(super) vault_path: String,
    vault_input: Option<Entity<InputState>>,
    vault_subscription: Option<Subscription>,
    /// Whether starter templates are installed on finish.
    pub(super) install_templates: bool,
    /// Whether the sample vault is seeded on finish.
    pub(super) install_sample: bool,
    /// Whether a provider call is running.
    pub(super) busy: bool,
    /// The last provider error.
    pub(super) error: Option<String>,
    /// The last status line (installed templates/sample, finish).
    pub(super) status: Option<String>,
}

impl Default for Onboarding {
    fn default() -> Self {
        Self {
            open: false,
            step: Step::Welcome,
            vault_path: String::new(),
            vault_input: None,
            vault_subscription: None,
            install_templates: false,
            install_sample: false,
            busy: false,
            error: None,
            status: None,
        }
    }
}

impl Onboarding {
    /// Whether the overlay is visible.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// The current step.
    pub fn step(&self) -> Step {
        self.step
    }

    /// The vault path shown in the welcome step.
    pub fn vault_path(&self) -> &str {
        &self.vault_path
    }

    /// Whether starter templates will be installed on finish.
    pub fn install_templates(&self) -> bool {
        self.install_templates
    }

    /// Whether the sample vault will be seeded on finish.
    pub fn install_sample(&self) -> bool {
        self.install_sample
    }

    /// The last provider error.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// The last status line.
    pub fn status(&self) -> Option<&str> {
        self.status.as_deref()
    }

    /// The vault input entity, once the wizard has a window.
    pub fn vault_input(&self) -> Option<&Entity<InputState>> {
        self.vault_input.as_ref()
    }
}

impl Shell {
    /// Open the wizard for a vault that reports pending onboarding.
    ///
    /// Called from [`Shell::new`](crate::shell::Shell::new); the vault path is
    /// read from the provider's snapshot.
    pub(super) fn seed_onboarding(&mut self, vault_path: String, pending: bool) {
        self.onboarding.vault_path = vault_path;
        self.onboarding.open = pending;
        self.onboarding.step = Step::Welcome;
        self.onboarding.install_templates = false;
        self.onboarding.install_sample = false;
        self.onboarding.busy = false;
        self.onboarding.error = None;
        self.onboarding.status = None;
    }

    /// Whether the onboarding overlay is visible.
    pub fn onboarding_open(&self) -> bool {
        self.onboarding.is_open()
    }

    /// Set the wizard step (used by the `STRATUM_ONBOARDING_STEP` capture flag).
    pub fn set_onboarding_step(&mut self, step: Step) {
        self.onboarding.step = step;
    }

    /// Open or close the wizard (used by the capture harness and tests).
    pub fn set_onboarding_open(&mut self, open: bool) {
        self.onboarding.open = open;
    }

    /// The wizard state.
    pub fn onboarding(&self) -> &Onboarding {
        &self.onboarding
    }

    /// Create the vault-path input on first render.
    ///
    /// The wizard is seeded without a window (from [`Shell::new`]), so the input
    /// entity is created lazily by the first frame that shows the overlay.
    pub(super) fn sync_onboarding_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.onboarding.vault_input.is_some() {
            return;
        }
        let placeholder = t_global("settings.vault_path");
        let initial = self.onboarding.vault_path.clone();
        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(placeholder)
                .default_value(initial)
        });
        let subscription =
            cx.subscribe(&input, |this: &mut Self, input, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    this.onboarding.vault_path = input.read(cx).value().to_string();
                    cx.notify();
                }
            });
        self.onboarding.vault_input = Some(input);
        self.onboarding.vault_subscription = Some(subscription);
    }

    /// Advance to the next wizard step.
    pub fn onboarding_next(&mut self, cx: &mut Context<Self>) {
        if let Some(next) = self.onboarding.step.next() {
            self.onboarding.step = next;
            self.onboarding.error = None;
        }
        cx.notify();
    }

    /// Return to the previous wizard step.
    pub fn onboarding_back(&mut self, cx: &mut Context<Self>) {
        if let Some(previous) = self.onboarding.step.previous() {
            self.onboarding.step = previous;
            self.onboarding.error = None;
        }
        cx.notify();
    }

    /// Toggle the starter-template install.
    pub fn onboarding_toggle_templates(&mut self, cx: &mut Context<Self>) {
        self.onboarding.install_templates = !self.onboarding.install_templates;
        cx.notify();
    }

    /// Toggle the sample-vault seed.
    pub fn onboarding_toggle_sample(&mut self, cx: &mut Context<Self>) {
        self.onboarding.install_sample = !self.onboarding.install_sample;
        cx.notify();
    }

    /// Open the native folder picker and switch the vault on selection.
    pub fn onboarding_browse_vault(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some(t_global("onboarding.choose_vault_folder").into()),
        });
        cx.spawn(async move |this: WeakEntity<Shell>, cx: &mut AsyncApp| {
            if let Ok(Ok(Some(paths))) = receiver.await {
                if let Some(path) = paths.into_iter().next() {
                    let value = path.to_string_lossy().to_string();
                    let _ = this.update(cx, |shell, cx| {
                        match shell.provider.set_vault_path(&value) {
                            Ok(()) => {
                                shell.onboarding.vault_path = value.clone();
                                let input = shell.onboarding.vault_input.clone();
                                if let Some(input) = input {
                                    // The picker callback runs outside a window
                                    // borrow, so defer the field update to the
                                    // next effect cycle.
                                    cx.defer(move |cx| {
                                        let Some(handle) = cx.active_window() else {
                                            return;
                                        };
                                        let _ = cx.update_window(handle, move |_, window, cx| {
                                            input.update(cx, |input, cx| {
                                                input.set_value(value.clone(), window, cx);
                                            });
                                        });
                                    });
                                }
                                shell.snapshot = shell.provider.snapshot();
                                shell.onboarding.error = None;
                                cx.notify();
                            }
                            Err(error) => {
                                shell.onboarding.error = Some(tr_global(
                                    "onboarding.error",
                                    &[("message", error.as_str())],
                                ));
                                cx.notify();
                            }
                        }
                    });
                }
            }
        })
        .detach();
    }

    /// Finish the wizard: install the selected extras, mark onboarding done and
    /// open the Quick Start guide.
    pub fn onboarding_finish(&mut self, cx: &mut Context<Self>) {
        if self.onboarding.busy {
            return;
        }
        self.onboarding.busy = true;
        self.onboarding.error = None;
        let install_templates = self.onboarding.install_templates;
        let install_sample = self.onboarding.install_sample;
        let mut status = Vec::new();
        let outcome = (|| -> Result<(), String> {
            if install_templates {
                let names = self.provider.install_starter_templates()?;
                status.push(format!("{} templates", names.len()));
            }
            if install_sample {
                let names = self.provider.install_starter_sample()?;
                status.push(format!("{} sample pages", names.len()));
            }
            self.provider.mark_onboarding_done()
        })();
        self.onboarding.busy = false;
        match outcome {
            Ok(()) => {
                self.onboarding.status = Some(status.join(", "));
                self.onboarding.open = false;
                // An existing Quick Start page is fine: the open below loads it,
                // and `create_page` is a no-op when the file is already there.
                let _ = self
                    .provider
                    .create_page(QUICK_START_PAGE, Some("Quick Start"));
                self.open_page(QUICK_START_PAGE);
            }
            Err(error) => {
                self.onboarding.status = None;
                self.onboarding.error = Some(tr_global(
                    "onboarding.error",
                    &[("message", error.as_str())],
                ));
            }
        }
        cx.notify();
    }
}
