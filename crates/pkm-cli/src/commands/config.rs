use pkm_core::PkmResult;
use std::path::Path;

/// Show the vault config file.
pub(crate) fn cmd_config(vault: &Path) -> PkmResult<()> {
    let config_path = vault.join(".pkm/config.toml");
    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)?;
        println!("{}", content);
    } else {
        println!("No config file found at {}", config_path.display());
        println!("Run `stratum init` to create one.");
    }
    Ok(())
}
