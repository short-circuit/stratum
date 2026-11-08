//! Persistence of graph-view settings.

use crate::settings::dto::GraphSettingsDto;
use crate::state::StratumService;

impl StratumService {
    /// Persist only the graph section of the vault configuration.
    pub async fn save_graph_settings(&self, graph: GraphSettingsDto) -> Result<(), String> {
        let state = self.lock()?;
        let config_path = state.vault_path.join(".pkm").join("config.toml");

        let mut config = if config_path.exists() {
            pkm_core::Config::load(&config_path).map_err(|e| e.to_string())?
        } else {
            pkm_core::Config {
                vault_path: state.vault_path.clone(),
                ..Default::default()
            }
        };

        config.graph = pkm_core::GraphConfig {
            show_connected: graph.show_connected,
            show_orphaned: graph.show_orphaned,
            show_tags: graph.show_tags,
            charge_strength: graph.charge_strength,
            link_distance: graph.link_distance,
            alpha_decay: graph.alpha_decay,
            velocity_decay: graph.velocity_decay,
            link_curvature: graph.link_curvature,
            use_3d: graph.use_3d,
        };

        config.save(&config_path).map_err(|e| e.to_string())?;
        Ok(())
    }
}
