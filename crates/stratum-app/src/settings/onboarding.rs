//! First-run onboarding flags.

use crate::state::{StratumService, VaultState};

impl StratumService {
    /// Report whether the first-run onboarding wizard should be shown.
    ///
    /// Deterministic detection that keeps the existing e2e harness green:
    ///
    /// - If `<vault>/.pkm/config.toml` exists, return the persisted
    ///   `onboarding_pending` flag (default `false` for configs that predate it,
    ///   so a populated vault never re-shows the wizard).
    /// - If no config exists, a brand-new empty vault (no pages) is treated as
    ///   onboarding-pending; a vault that already has pages (CI harness, CLI-created
    ///   vaults) is not. On any store error the command returns `false` (safe).
    ///
    /// Completion persists `pending=false` via [`Self::mark_onboarding_done`], so
    /// the wizard never reappears for that vault. "Reset onboarding" re-arms it
    /// via [`Self::set_onboarding_pending`].
    pub async fn get_onboarding_state(&self) -> Result<bool, String> {
        let state = self.lock()?;
        let config_path = state.vault_path.join(".pkm").join("config.toml");

        if config_path.exists() {
            let config = pkm_core::Config::load(&config_path).map_err(|e| e.to_string())?;
            return Ok(config.onboarding_pending);
        }

        match state.get_store() {
            Ok(store) => match store.page_count() {
                Ok(n) => Ok(n == 0),
                Err(_) => Ok(false),
            },
            Err(_) => Ok(false),
        }
    }

    /// Persist that the onboarding wizard has been completed for this vault.
    ///
    /// Loads the existing config (or creates one with defaults) and sets
    /// `onboarding_pending = false`, then saves it to `<vault>/.pkm/config.toml`.
    pub async fn mark_onboarding_done(&self) -> Result<(), String> {
        let state = self.lock()?;
        let config_path = state.vault_path.join(".pkm").join("config.toml");
        let mut config = load_or_default_config(&state, &config_path)?;
        config.onboarding_pending = false;
        config.save(&config_path).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Set whether the onboarding wizard should be shown on the next launch.
    ///
    /// Used by the Settings "Reset onboarding" action to re-arm the wizard.
    pub async fn set_onboarding_pending(&self, pending: bool) -> Result<(), String> {
        let state = self.lock()?;
        let config_path = state.vault_path.join(".pkm").join("config.toml");
        let mut config = load_or_default_config(&state, &config_path)?;
        config.onboarding_pending = pending;
        config.save(&config_path).map_err(|e| e.to_string())?;
        Ok(())
    }
}

/// Load the vault config, or start from defaults when none exists yet.
fn load_or_default_config(
    state: &VaultState,
    config_path: &std::path::Path,
) -> Result<pkm_core::Config, String> {
    if config_path.exists() {
        pkm_core::Config::load(config_path).map_err(|e| e.to_string())
    } else {
        Ok(pkm_core::Config {
            vault_path: state.vault_path.clone(),
            ..Default::default()
        })
    }
}
