//! Maintenance, sync and backup actions for the native Settings panel.
//!
//! Split from [`super::actions`] so each file stays under the repository
//! length limit; the methods remain inherent [`SettingsView`] actions.

use gpui_kit::prelude::*;
use stratum_i18n::{t_global, tr_global};

use super::SettingsView;

impl SettingsView {
    /// Rebuild the vault index from disk.
    pub fn reindex(&mut self, cx: &mut Context<Self>) {
        self.busy = Some("reindex");
        match self.provider.reindex_vault() {
            Ok(result) if result.failed > 0 => {
                self.busy = None;
                self.set_error(
                    format!(
                        "{} {} ok, {} failed out of {}",
                        t_global("settings.reindexed"),
                        result.succeeded,
                        result.failed,
                        result.processed
                    ),
                    cx,
                );
            }
            Ok(result) => {
                let count = result.succeeded.to_string();
                self.busy = None;
                self.set_message(
                    tr_global("settings.reindexed_count", &[("count", count.as_str())]),
                    cx,
                );
            }
            Err(error) => {
                self.busy = None;
                self.set_error(
                    format!("{} {error}", t_global("settings.reindex_failed")),
                    cx,
                );
            }
        }
    }

    /// Reconcile the database with the files on disk.
    pub fn repair(&mut self, cx: &mut Context<Self>) {
        self.busy = Some("repair");
        match self.provider.repair_db_from_disk() {
            Ok(result) if result.failed > 0 => {
                self.busy = None;
                self.set_error(
                    format!(
                        "{} {} ok, {} failed out of {}",
                        t_global("settings.repaired"),
                        result.succeeded,
                        result.failed,
                        result.processed
                    ),
                    cx,
                );
            }
            Ok(result) => {
                let count = result.succeeded.to_string();
                self.busy = None;
                self.set_message(
                    tr_global("settings.repaired_count", &[("count", count.as_str())]),
                    cx,
                );
            }
            Err(error) => {
                self.busy = None;
                self.set_error(
                    format!("{} {error}", t_global("settings.repair_failed")),
                    cx,
                );
            }
        }
    }

    /// Normalize every page through the block parser.
    pub fn normalize_all(&mut self, cx: &mut Context<Self>) {
        self.busy = Some("normalize");
        match self.provider.normalize_all_files() {
            Ok(count) => {
                let count = count.to_string();
                self.busy = None;
                self.set_message(
                    tr_global("settings.normalized_count", &[("count", count.as_str())]),
                    cx,
                );
            }
            Err(error) => {
                self.busy = None;
                self.set_error(
                    format!("{} {error}", t_global("settings.normalize_failed")),
                    cx,
                );
            }
        }
    }

    /// Run a manual sync, prompting for a passphrase when the key needs one.
    pub fn sync_now(&mut self, cx: &mut Context<Self>) {
        self.busy = Some("sync");
        self.message = None;
        match self.provider.sync_now() {
            Ok(status) => {
                self.busy = None;
                self.finish_sync(status, cx);
            }
            Err(error) => {
                self.busy = None;
                if error.contains("passphrase") || error.contains("Passphrase") {
                    self.passphrase_prompt = true;
                    cx.notify();
                } else {
                    self.set_error(format!("{} {error}", t_global("settings.sync_failed")), cx);
                }
            }
        }
    }

    /// Retry the manual sync with an ephemeral SSH passphrase.
    pub fn submit_passphrase(&mut self, cx: &mut Context<Self>) {
        let passphrase = self.form.passphrase.clone();
        self.busy = Some("sync");
        match self.provider.sync_with_passphrase(&passphrase) {
            Ok(status) => {
                self.busy = None;
                self.passphrase_prompt = false;
                self.finish_sync(status, cx);
            }
            Err(error) => {
                self.busy = None;
                self.set_error(format!("{} {error}", t_global("settings.sync_failed")), cx);
            }
        }
    }

    /// Dismiss the inline passphrase prompt.
    pub fn dismiss_passphrase(&mut self, cx: &mut Context<Self>) {
        self.passphrase_prompt = false;
        cx.notify();
    }

    /// Start the periodic sync scheduler.
    pub fn start_scheduler(&mut self, cx: &mut Context<Self>) {
        match self.provider.start_sync_scheduler() {
            Ok(()) => self.set_message(t_global("settings.scheduler_started"), cx),
            Err(error) => self.set_error(
                format!("{} {error}", t_global("settings.scheduler_error")),
                cx,
            ),
        }
    }

    /// Accept one conflicted file, then refresh the conflict list.
    pub fn resolve_conflict(&mut self, path: &str, cx: &mut Context<Self>) {
        self.passphrase_prompt = false;
        match self.provider.resolve_conflict(path) {
            Ok(()) => {
                self.sync_status = self.provider.sync_status().ok();
                cx.notify();
            }
            Err(error) => {
                self.set_error(format!("{} {error}", t_global("settings.sync_failed")), cx)
            }
        }
    }

    /// Accept every conflicted file in the current list.
    pub fn resolve_all_conflicts(&mut self, paths: &[String], cx: &mut Context<Self>) {
        self.passphrase_prompt = false;
        for path in paths {
            if let Err(error) = self.provider.resolve_conflict(path) {
                self.set_error(format!("{} {error}", t_global("settings.sync_failed")), cx);
                return;
            }
        }
        self.sync_status = self.provider.sync_status().ok();
        cx.notify();
    }

    /// Abort the in-progress merge and refresh the sync status.
    pub fn abort_merge(&mut self, cx: &mut Context<Self>) {
        self.passphrase_prompt = false;
        match self.provider.abort_merge() {
            Ok(()) => {
                self.sync_status = self.provider.sync_status().ok();
                self.set_message(t_global("settings.sync_completed"), cx);
            }
            Err(error) => {
                self.set_error(format!("{} {error}", t_global("settings.sync_failed")), cx)
            }
        }
    }

    /// Expand or collapse the commit log, loading it on first open.
    pub fn toggle_commits(&mut self, cx: &mut Context<Self>) {
        if !self.commits_open {
            self.commits = self.provider.commit_log().unwrap_or_default();
        }
        self.commits_open = !self.commits_open;
        cx.notify();
    }

    /// Run a cloud/folder backup now.
    pub fn run_backup(&mut self, cx: &mut Context<Self>) {
        self.backup_running = true;
        match self.provider.run_backup() {
            Ok(status) => {
                self.backup_running = false;
                self.backup_status = Some(status.clone());
                if let Some(error) = status.last_error {
                    self.set_error(
                        format!("{} {error}", t_global("settings.backup_failed")),
                        cx,
                    );
                } else {
                    self.set_message(t_global("settings.backup_success"), cx);
                }
            }
            Err(error) => {
                self.backup_running = false;
                self.set_error(
                    format!("{} {error}", t_global("settings.backup_failed")),
                    cx,
                );
            }
        }
    }

    /// Probe the configured WebDAV endpoint.
    pub fn test_webdav(&mut self, cx: &mut Context<Self>) {
        let backup = &self.form.settings.sync.backup;
        let Some(url) = backup.webdav_url.clone().filter(|url| !url.is_empty()) else {
            return;
        };
        self.webdav_testing = true;
        self.webdav_result = None;
        let result = self.provider.test_webdav(
            &url,
            backup.webdav_username.as_deref(),
            backup.webdav_password.as_deref(),
        );
        self.webdav_testing = false;
        match result {
            Ok(detail) => self.webdav_result = Some(detail),
            Err(error) => self.webdav_result = Some(error),
        }
        cx.notify();
    }

    /// Mark the first-run onboarding wizard as pending again.
    pub fn reset_onboarding(&mut self, cx: &mut Context<Self>) {
        match self.provider.set_onboarding_pending(true) {
            Ok(()) => self.set_message(t_global("settings.onboarding_reset_done"), cx),
            Err(error) => self.set_error(
                format!("{} {error}", t_global("settings.reset_onboarding_help")),
                cx,
            ),
        }
    }

    /// Install the starter templates.
    pub fn install_templates(&mut self, cx: &mut Context<Self>) {
        self.busy = Some("templates");
        match self.provider.install_starter_templates() {
            Ok(names) => {
                let count = names.len().to_string();
                self.busy = None;
                self.set_message(
                    tr_global("settings.templates_installed", &[("count", count.as_str())]),
                    cx,
                );
            }
            Err(error) => {
                self.busy = None;
                self.set_error(
                    format!("{} {error}", t_global("settings.install_starter_templates")),
                    cx,
                );
            }
        }
    }

    /// Finish a sync with a status result.
    fn finish_sync(&mut self, status: crate::provider::SyncStatusDto, cx: &mut Context<Self>) {
        let conflicts = status.conflicts.len();
        self.sync_status = Some(status);
        if conflicts > 0 {
            self.set_message(
                tr_global(
                    "settings.conflicts_in",
                    &[("count", conflicts.to_string().as_str())],
                ),
                cx,
            );
        } else if self.sync_status.as_ref().map(|s| s.status.as_str()) == Some("ok") {
            self.set_message(t_global("settings.sync_completed"), cx);
        } else {
            self.set_message(t_global("settings.sync_had_errors"), cx);
        }
    }
}
