//! Native Templates panel.
//!
//! [`TemplatesView`] ports the shipped desktop `TemplatesPanel`: a target-page
//! field, an editable variable list (blank keys are filtered before apply), the
//! template cards with placeholder chips and Apply actions, and the starter
//! template installer. Provider calls stay behind the
//! [`crate::provider::PanelData`] seam; this module owns presentation and the
//! pending toast the shell consumes.
//!
//! Applying with an empty target is a no-op, matching the shipped panel. The
//! template content itself is never edited here — only the target page is
//! written by the backend.
//!
//! [`TEMPLATES_I18N_KEYS`] is the release gate list for every string this
//! surface renders.

use std::sync::Arc;

use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::ActiveTheme;
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use crate::panels::PanelRequests;
use crate::provider::{ShellData, TemplateRow};

pub mod cards;
pub mod model;

use model::VariableRow;

/// Every translation key the Templates surface renders.
///
/// The contract test walks this list against all eight catalogs, so adding a
/// visible string without a catalog entry fails the build instead of leaking a
/// raw key into the UI.
pub const TEMPLATES_I18N_KEYS: &[&str] = &[
    "query.name",
    "query.value",
    "settings.install_starter_templates",
    "templates.add_variable",
    "templates.apply",
    "templates.created",
    "templates.no_templates_yet_save_a_page_as_template_to_get_started",
    "templates.target_page_path",
    "templates.templates",
    "templates.variables",
];

/// One variable row's live inputs plus their subscriptions.
struct VariableInputs {
    key: Entity<InputState>,
    value: Entity<InputState>,
    _subscriptions: Vec<Subscription>,
}

/// The templates panel entity.
pub struct TemplatesView {
    provider: Arc<dyn ShellData>,
    target: Entity<InputState>,
    _target_subscription: Subscription,
    target_value: String,
    variables: Vec<VariableRow>,
    variable_inputs: Vec<VariableInputs>,
    templates: Vec<TemplateRow>,
    load_error: Option<String>,
    message: Option<String>,
    pending_navigation: Option<String>,
    pending_toast: Option<String>,
}

impl TemplatesView {
    /// Build the panel and load the template list.
    pub fn new(provider: Arc<dyn ShellData>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let target = cx.new(|cx| InputState::new(window, cx).placeholder("pages/my-new-page.md"));
        let subscription =
            cx.subscribe(&target, |this: &mut Self, input, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    this.target_value = input.read(cx).value().to_string();
                    cx.notify();
                }
            });
        let mut view = Self {
            provider,
            target,
            _target_subscription: subscription,
            target_value: String::new(),
            variables: vec![VariableRow::default()],
            variable_inputs: Vec::new(),
            templates: Vec::new(),
            load_error: None,
            message: None,
            pending_navigation: None,
            pending_toast: None,
        };
        view.refresh_templates(cx);
        view
    }

    /// The loaded template rows.
    pub fn templates(&self) -> &[TemplateRow] {
        &self.templates
    }

    /// The current target-page value.
    pub fn target_value(&self) -> &str {
        &self.target_value
    }

    /// The editable variable rows.
    pub fn variables(&self) -> &[VariableRow] {
        &self.variables
    }

    /// The template-list load error, if any.
    pub fn load_error(&self) -> Option<&str> {
        self.load_error.as_deref()
    }

    /// The last inline status message, if any.
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    /// Set the target-page field (used by tests and programmatic flows).
    pub fn set_target(
        &mut self,
        value: impl Into<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let value = value.into();
        self.target_value = value.clone();
        let target = self.target.clone();
        target.update(cx, |input, cx| input.set_value(value, window, cx));
        cx.notify();
    }

    /// Append a blank variable row.
    pub fn add_variable(&mut self, cx: &mut Context<Self>) {
        self.variables.push(VariableRow::default());
        cx.notify();
    }

    /// Reload the template list.
    pub fn refresh_templates(&mut self, cx: &mut Context<Self>) {
        match self.provider.templates() {
            Ok(templates) => {
                self.templates = templates;
                self.load_error = None;
            }
            Err(error) => {
                self.templates.clear();
                self.load_error = Some(error);
            }
        }
        cx.notify();
    }

    /// Apply `name` to the target page with the non-blank variables.
    ///
    /// An empty target is a no-op (shipped behaviour). On success the target
    /// field clears and the shell receives a `templates.created` toast with the
    /// target path; on failure the shell receives the provider error.
    pub fn apply(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        let target = self.target_value.trim().to_string();
        if target.is_empty() {
            return;
        }
        let variables = model::filter_variables(&self.variables);
        match self.provider.apply_template(name, &target, variables) {
            Ok(_) => {
                self.message = Some(format!("{} {}", t_global("templates.created"), target));
                self.pending_toast = self.message.clone();
                self.set_target(String::new(), window, cx);
            }
            Err(error) => {
                self.message = Some(error.clone());
                self.pending_toast = Some(error);
                cx.notify();
            }
        }
    }

    /// Install the bundled starter templates and refresh the list.
    pub fn install_starter_templates(&mut self, cx: &mut Context<Self>) {
        match self.provider.install_starter_templates() {
            Ok(names) => {
                self.message = Some(format!(
                    "{} {}",
                    t_global("settings.install_starter_templates"),
                    names.join(", ")
                ));
                self.pending_toast = self.message.clone();
                self.refresh_templates(cx);
            }
            Err(error) => {
                self.message = Some(error.clone());
                self.pending_toast = Some(error);
                cx.notify();
            }
        }
    }

    /// The target input entity.
    pub fn target_input(&self) -> &Entity<InputState> {
        &self.target
    }

    /// Create or refresh the variable key/value inputs before painting.
    fn sync_variable_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        while self.variable_inputs.len() < self.variables.len() {
            let row = self.variables[self.variable_inputs.len()].clone();
            let key = cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder(t_global("query.name"))
                    .default_value(row.key)
            });
            let value = cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder(t_global("query.value"))
                    .default_value(row.value)
            });
            let key_subscription =
                cx.subscribe(&key, |this: &mut Self, _, event: &InputEvent, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.pull_variable_values(cx);
                    }
                });
            let value_subscription =
                cx.subscribe(&value, |this: &mut Self, _, event: &InputEvent, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.pull_variable_values(cx);
                    }
                });
            self.variable_inputs.push(VariableInputs {
                key,
                value,
                _subscriptions: vec![key_subscription, value_subscription],
            });
        }
        self.variable_inputs.truncate(self.variables.len());
    }

    /// Pull the variable input text back into the model.
    fn pull_variable_values(&mut self, cx: &mut Context<Self>) {
        let rows: Vec<VariableRow> = self
            .variable_inputs
            .iter()
            .map(|inputs| VariableRow {
                key: inputs.key.read(cx).value().to_string(),
                value: inputs.value.read(cx).value().to_string(),
            })
            .collect();
        for (index, row) in rows.into_iter().enumerate() {
            if let Some(existing) = self.variables.get_mut(index) {
                *existing = row;
            }
        }
        cx.notify();
    }
}

impl PanelRequests for TemplatesView {
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

impl Render for TemplatesView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_variable_inputs(window, cx);
        let theme = cx.theme();
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.colors.background)
            .text_color(theme.colors.foreground)
            .child(header(cx))
            .child(
                div()
                    .id("templates-body")
                    .flex()
                    .flex_col()
                    .flex_1()
                    .overflow_y_scroll()
                    .p(px(24.0))
                    .gap(px(10.0))
                    .child(target_row(self, cx))
                    .child(variables_section(self, cx))
                    .when_some(self.message.clone(), |element, message| {
                        element.child(status_row(message, cx))
                    })
                    .when_some(self.load_error.clone(), |element, error| {
                        element.child(status_row(error, cx))
                    })
                    .child(cards::template_cards(self, cx)),
            )
    }
}

/// The panel title bar.
fn header(cx: &mut Context<TemplatesView>) -> impl IntoElement {
    let theme = cx.theme();
    div()
        .flex()
        .items_center()
        .px(px(24.0))
        .py(px(12.0))
        .border_b_1()
        .border_color(theme.colors.border)
        .child(
            div()
                .text_size(px(18.0))
                .font_weight(FontWeight::BOLD)
                .text_color(theme.colors.foreground)
                .child(t_global("templates.templates")),
        )
}

/// The target-page field.
fn target_row(view: &TemplatesView, cx: &mut Context<TemplatesView>) -> impl IntoElement {
    let theme = cx.theme();
    div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .child(
            div()
                .text_size(px(11.0))
                .text_color(theme.colors.muted_foreground)
                .child(t_global("templates.target_page_path")),
        )
        .child(
            div()
                .id("template-target")
                .debug_selector(|| "template-target".to_string())
                .child(Input::new(&view.target)),
        )
}

/// The variable caption, editable rows and add action.
fn variables_section(view: &TemplatesView, cx: &mut Context<TemplatesView>) -> impl IntoElement {
    let theme = cx.theme();
    let muted = theme.colors.muted_foreground;
    let border = theme.colors.border;
    div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(
            div()
                .text_size(px(11.0))
                .text_color(muted)
                .child(t_global("templates.variables")),
        )
        .children(
            view.variable_inputs
                .iter()
                .enumerate()
                .map(|(index, inputs)| {
                    div()
                        .flex()
                        .gap(px(6.0))
                        .child(
                            div()
                                .id(format!("template-variable-key-{index}"))
                                .debug_selector(move || format!("template-variable-key-{index}"))
                                .flex_1()
                                .child(Input::new(&inputs.key)),
                        )
                        .child(
                            div()
                                .id(format!("template-variable-value-{index}"))
                                .debug_selector(move || format!("template-variable-value-{index}"))
                                .flex_1()
                                .child(Input::new(&inputs.value)),
                        )
                }),
        )
        .child(
            div()
                .id("template-add-variable")
                .debug_selector(|| "template-add-variable".to_string())
                .px(px(10.0))
                .py(px(4.0))
                .rounded(px(6.0))
                .border_1()
                .border_color(border)
                .text_size(px(11.0))
                .text_color(muted)
                .cursor_pointer()
                .child(t_global("templates.add_variable"))
                .on_click(cx.listener(|view, _, _, cx| view.add_variable(cx))),
        )
        .child(
            div()
                .id("template-install-starter")
                .debug_selector(|| "template-install-starter".to_string())
                .px(px(10.0))
                .py(px(4.0))
                .rounded(px(6.0))
                .text_size(px(11.0))
                .text_color(muted)
                .cursor_pointer()
                .child(t_global("settings.install_starter_templates"))
                .on_click(cx.listener(|view, _, _, cx| view.install_starter_templates(cx))),
        )
}

/// The inline status message.
fn status_row(message: String, cx: &mut Context<TemplatesView>) -> impl IntoElement {
    let theme = cx.theme();
    div()
        .id("template-message")
        .debug_selector(|| "template-message".to_string())
        .px(px(10.0))
        .py(px(6.0))
        .rounded(px(6.0))
        .bg(theme.colors.muted_foreground.opacity(0.12))
        .text_size(px(11.0))
        .text_color(theme.colors.foreground)
        .child(message)
}
