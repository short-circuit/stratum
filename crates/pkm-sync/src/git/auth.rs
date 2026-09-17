//! SSH key and passphrase credential handling for the git engine.
//!
//! Auth state lives on the [`GitEngine`] itself; these methods wire it
//! into child git/ssh commands and the GIT_ASKPASS helper script.

use pkm_core::{PkmError, PkmResult};
#[cfg(not(windows))]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

impl super::GitEngine {
    /// Inject SSH configuration into a git command.
    ///
    /// When an SSH key is configured, sets `GIT_SSH_COMMAND` to use that key
    /// with strict host key checking.
    ///
    /// When a passphrase is also configured, sets `GIT_ASKPASS` pointing to
    /// the helper script (written by `set_passphrase`) so the passphrase is
    /// never embedded in an environment variable or command line.
    pub(super) fn inject_ssh_key(&self, cmd: &mut std::process::Command) {
        if let Some(ref key_path) = self.ssh_key_path {
            let ssh_base = format!(
                "ssh -i {} -o IdentitiesOnly=yes -o StrictHostKeyChecking=accept-new",
                key_path.display()
            );

            if self.passphrase.is_some() {
                if let Some(ref script_path) = self.askpass_script_path {
                    cmd.env("GIT_ASKPASS", script_path);
                }
            }

            cmd.env("GIT_SSH_COMMAND", &ssh_base);
        }
    }

    /// Write the GIT_ASKPASS helper script that provides the SSH key passphrase.
    ///
    /// The script uses `printf` with octal-encoded bytes so there are no shell
    /// escaping concerns regardless of passphrase content. The file is written
    /// with 0700 permissions and removed in `Drop`.
    fn write_askpass_script(&mut self) -> PkmResult<PathBuf> {
        let passphrase = self
            .passphrase
            .clone()
            .ok_or_else(|| PkmError::Git("no passphrase set for askpass script".into()))?;

        // Clean up any previous script first
        self.cleanup_askpass_script();

        let path = std::env::temp_dir().join(format!("pkm-askpass-{}", std::process::id()));

        // Encode the passphrase as octal escapes so no shell characters can
        // interfere — printf(1) interprets \NNN in the format string.
        let octal: String = passphrase
            .bytes()
            .map(|b| format!("\\{:03o}", b))
            .collect::<Vec<_>>()
            .join("");
        let script = format!("#!/bin/sh\nprintf '{}'\n", octal);

        std::fs::write(&path, &script)
            .map_err(|e| PkmError::Git(format!("write askpass script: {e}")))?;
        #[cfg(not(windows))]
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))
            .map_err(|e| PkmError::Git(format!("chmod askpass script: {e}")))?;

        tracing::debug!("wrote GIT_ASKPASS script to {}", path.display());

        self.askpass_script_path = Some(path.clone());
        Ok(path)
    }

    /// Remove the askpass script from disk.
    pub(super) fn cleanup_askpass_script(&mut self) {
        if let Some(ref path) = self.askpass_script_path.take() {
            let _ = std::fs::remove_file(path);
            tracing::debug!("cleaned up GIT_ASKPASS script {}", path.display());
        }
    }

    pub fn set_ssh_key_path(&mut self, path: Option<PathBuf>) {
        self.ssh_key_path = path;
        // If we have both a key and passphrase, ensure the askpass script exists
        if self.passphrase.is_some() && self.ssh_key_path.is_some() {
            if self.askpass_script_path.is_none() {
                let _ = self.write_askpass_script();
            }
        } else {
            self.cleanup_askpass_script();
        }
    }

    pub fn ssh_key_path(&self) -> Option<&PathBuf> {
        self.ssh_key_path.as_ref()
    }

    pub fn set_passphrase(&mut self, passphrase: Option<String>) {
        self.passphrase = passphrase;
        // Recreate the askpass script with the new passphrase
        self.cleanup_askpass_script();
        if self.passphrase.is_some() && self.ssh_key_path.is_some() {
            let _ = self.write_askpass_script();
        }
    }

    pub fn passphrase(&self) -> Option<&str> {
        self.passphrase.as_deref()
    }

    #[allow(clippy::result_large_err)]
    pub fn credentials_callback(
        &self,
    ) -> impl FnMut(gix::credentials::helper::Action) -> gix::credentials::protocol::Result
           + Clone
           + 'static {
        let _key_path = self.ssh_key_path.clone();
        let _passphrase = self.passphrase.clone();
        move |action: gix::credentials::helper::Action| gix::credentials::builtin(action)
    }
}
