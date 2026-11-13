//! Query-panel input synchronisation.
//!
//! The raw Datalog editor, the per-row builder value inputs and the save/rename
//! name field are gpui input entities. They are created lazily and kept in sync
//! with [`PanelState`] before each paint, so programmatic transitions (reset,
//! copy-to-editor, saved-query load) update the visible widgets without a
//! round-trip through the event loop.

use gpui_kit::component::input::{InputEvent, InputState, TextareaState};
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use super::QueryView;

impl QueryView {
    /// Synchronise the raw editor, builder value inputs and dialog input with
    /// the model before painting.
    pub(super) fn sync_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.sync_editor(window, cx);
        self.sync_row_inputs(window, cx);
        self.sync_dialog_input(window, cx);
    }

    /// Create or update the raw Datalog editor.
    pub(super) fn sync_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let datalog = self.state.datalog().to_string();
        if self.editor.is_none() {
            let editor = cx.new(move |cx| {
                TextareaState::new(window, cx)
                    .default_value(datalog)
                    .placeholder(t_global("query.enter_datalog_query"))
            });
            let subscription = cx.subscribe(
                &editor,
                |this: &mut Self, editor, event: &InputEvent, cx| {
                    if matches!(event, InputEvent::Change) {
                        let value = editor.read(cx).value().to_string();
                        this.state.set_datalog(value);
                        cx.notify();
                    }
                },
            );
            self.editor = Some(editor);
            self.editor_subscription = Some(subscription);
            return;
        }
        let Some(editor) = self.editor.clone() else {
            return;
        };
        let current = editor.read(cx).value().to_string();
        if current != self.state.datalog() {
            let datalog = self.state.datalog().to_string();
            editor.update(cx, |editor, cx| editor.set_value(datalog, window, cx));
        }
    }

    /// Create or update one value input per value-needing builder row.
    fn sync_row_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let rows: Vec<(String, String, bool)> = self
            .state
            .rows()
            .iter()
            .map(|row| (row.id.clone(), row.value.clone(), row.op.needs_value()))
            .collect();
        let mut needed = Vec::new();
        for (id, value, needs_value) in rows {
            if !needs_value {
                continue;
            }
            needed.push(id.clone());
            if let Some(input) = self.row_inputs.get(&id).cloned() {
                let current = input.read(cx).value().to_string();
                if current != value {
                    input.update(cx, |input, cx| {
                        input.set_value(value.clone(), window, cx);
                    });
                }
                continue;
            }
            let window_ref = &mut *window;
            let input = cx.new(move |cx| {
                InputState::new(window_ref, cx)
                    .placeholder(t_global("query.value"))
                    .default_value(value)
            });
            let subscription =
                cx.subscribe(&input, |this: &mut Self, _, event: &InputEvent, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.update_row_values(cx);
                    }
                });
            self.row_inputs.insert(id, input);
            self.row_subscriptions.push(subscription);
        }
        self.row_inputs.retain(|id, _| needed.contains(id));
    }

    /// Pull builder value input text back into the model.
    fn update_row_values(&mut self, cx: &mut Context<Self>) {
        let values: Vec<(String, String)> = self
            .row_inputs
            .iter()
            .map(|(id, input)| (id.clone(), input.read(cx).value().to_string()))
            .collect();
        for (id, value) in values {
            self.state.set_row_value(&id, value);
        }
        cx.notify();
    }

    /// Create or update the save/rename name input.
    fn sync_dialog_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(dialog) = self.dialog.as_ref() else {
            self.dialog_input = None;
            return;
        };
        let name = dialog.name.clone();
        if self.dialog_input.is_none() {
            let input = cx.new(move |cx| {
                InputState::new(window, cx)
                    .placeholder(t_global("query.name"))
                    .default_value(name)
            });
            let subscription =
                cx.subscribe(&input, |this: &mut Self, input, event: &InputEvent, cx| {
                    if matches!(event, InputEvent::Change) {
                        let value = input.read(cx).value().to_string();
                        if let Some(dialog) = &mut this.dialog {
                            dialog.name = value;
                            dialog.error = None;
                        }
                        cx.notify();
                    }
                });
            self.dialog_input = Some(input);
            self.dialog_subscription = Some(subscription);
            return;
        }
        let Some(input) = self.dialog_input.clone() else {
            return;
        };
        let current = input.read(cx).value().to_string();
        if current != name {
            input.update(cx, |input, cx| input.set_value(name, window, cx));
        }
    }
}
