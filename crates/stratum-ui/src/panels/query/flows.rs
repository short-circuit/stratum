//! Query-panel transitions: run, tab switching, reset/copy and the saved-query
//! dialog flows.
//!
//! These are the actions the toolbar, builder, saved list and dialogs dispatch
//! to. All provider access stays behind [`QueryView::provider`]; the methods
//! only update [`PanelState`] and record pending toast requests.

use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use super::dialogs::{SavedDialog, SavedDialogKind};
use super::model::QueryTab;
use super::QueryView;
use crate::provider::SavedQueryRow;

impl QueryView {
    /// Run the current query through the provider.
    pub fn run(&mut self, cx: &mut Context<Self>) {
        if let Some(key) = self.state.build_error_key() {
            self.state.set_run_error(t_global(key));
            cx.notify();
            return;
        }
        let datalog = self.state.datalog().to_string();
        self.state.set_running(true);
        match self.provider.run_query(&datalog) {
            Ok(table) => self.state.set_result(table),
            Err(error) => self.state.set_run_error(error),
        }
        cx.notify();
    }

    /// Switch the active tab.
    pub fn switch_tab(&mut self, tab: QueryTab, cx: &mut Context<Self>) {
        self.state.switch_tab(tab);
        cx.notify();
    }

    /// Reset the editor to the shipped reset query.
    pub fn reset(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.state.reset();
        self.sync_editor(window, cx);
        cx.notify();
    }

    /// Copy the generated builder query into the raw editor.
    pub fn copy_to_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.state.copy_to_editor() {
            self.pending_toast = Some(t_global("query.copied_to_editor"));
            self.sync_editor(window, cx);
        }
        cx.notify();
    }

    /// Load a saved query into the editor.
    pub fn load_saved(&mut self, row: &SavedQueryRow, window: &mut Window, cx: &mut Context<Self>) {
        self.state.load_saved(&row.query);
        self.sync_editor(window, cx);
        cx.notify();
    }

    /// Reload the saved-query list.
    pub fn refresh_saved(&mut self, cx: &mut Context<Self>) {
        self.saved_loading = true;
        match self.provider.saved_queries() {
            Ok(rows) => {
                self.saved = rows;
                self.saved_error = None;
            }
            Err(error) => {
                self.saved_error = Some(format!(
                    "{} {error}",
                    t_global("query.could_not_load_saved_queries")
                ));
            }
        }
        self.saved_loading = false;
        cx.notify();
    }

    /// Open the save dialog.
    pub fn open_save_dialog(&mut self, cx: &mut Context<Self>) {
        self.dialog = Some(SavedDialog::save());
        self.dialog_input = None;
        cx.notify();
    }

    /// Open the rename dialog for `name`.
    pub fn open_rename_dialog(&mut self, name: &str, cx: &mut Context<Self>) {
        self.dialog = Some(SavedDialog::rename(name));
        self.dialog_input = None;
        cx.notify();
    }

    /// Open the delete confirmation for `name`.
    pub fn open_delete_dialog(&mut self, name: &str, cx: &mut Context<Self>) {
        self.delete_target = Some(name.to_string());
        cx.notify();
    }

    /// Set the dialog name field.
    pub fn set_dialog_name(&mut self, name: impl Into<String>, cx: &mut Context<Self>) {
        if let Some(dialog) = &mut self.dialog {
            dialog.name = name.into();
            dialog.error = None;
        }
        cx.notify();
    }

    /// Submit the open save/rename dialog.
    pub fn submit_dialog(&mut self, cx: &mut Context<Self>) {
        let Some(dialog) = self.dialog.as_ref() else {
            return;
        };
        let kind = dialog.kind;
        let target = dialog.target.clone();
        let name = dialog.name.trim().to_string();
        if name.is_empty() {
            if let Some(dialog) = &mut self.dialog {
                dialog.error = Some(t_global("query.name_must_not_be_empty"));
            }
            cx.notify();
            return;
        }
        let result = match kind {
            SavedDialogKind::Save => self.provider.save_query(&name, self.state.datalog()),
            SavedDialogKind::Rename => self
                .provider
                .rename_query(target.as_deref().unwrap_or_default(), &name),
        };
        match result {
            Ok(_) => {
                self.dialog = None;
                self.dialog_input = None;
                self.pending_toast = Some(t_global(match kind {
                    SavedDialogKind::Save => "query.save_query",
                    SavedDialogKind::Rename => "query.rename_saved_query",
                }));
                self.refresh_saved(cx);
            }
            Err(error) => {
                if let Some(dialog) = &mut self.dialog {
                    dialog.error = Some(error);
                }
                cx.notify();
            }
        }
    }

    /// Confirm the pending delete.
    pub fn confirm_delete(&mut self, cx: &mut Context<Self>) {
        let Some(name) = self.delete_target.clone() else {
            return;
        };
        match self.provider.delete_query(&name) {
            Ok(()) => {
                self.delete_target = None;
                self.pending_toast = Some(t_global("query.delete_saved_query"));
                self.refresh_saved(cx);
            }
            Err(error) => {
                self.saved_error = Some(format!(
                    "{} {error}",
                    t_global("query.could_not_load_saved_queries")
                ));
                cx.notify();
            }
        }
    }

    /// Cancel the pending delete.
    pub fn cancel_delete(&mut self, cx: &mut Context<Self>) {
        self.delete_target = None;
        cx.notify();
    }

    /// Dismiss the save/rename dialog.
    pub fn dismiss_dialog(&mut self, cx: &mut Context<Self>) {
        self.dialog = None;
        self.dialog_input = None;
        cx.notify();
    }
}
