//! State model for the native graph panel.
//!
//! The shipped React panel keeps the view mode, visibility toggles, force
//! settings and node cap in one hook. This module mirrors that shape without
//! pulling in React state: [`GraphSettings`] maps to
//! [`stratum_graph::ForceParams`] and [`stratum_graph::SceneOptions`],
//! [`GraphStart`] carries the deterministic capture start state, and
//! [`banner_text`] reproduces the shipped node-cap warning with the catalog
//! placeholders. [`GraphSettings::to_row`] / [`GraphSettings::from_row`] bridge
//! the panel state to the persisted [`GraphSettingsRow`] seam.

use stratum_graph::{
    FilteredGraph, ForceParams, GraphFilterRequest, GraphVisibility, SceneOptions, ViewMode,
};
use stratum_i18n::{t_global, tr_global};

use crate::provider::panels::GraphSettingsRow;

/// Default deterministic simulation ticks used before the first frame.
///
/// This is the shipped `cooldownTicks` value: the React panel lets
/// `d3-force-3d` settle for 300 ticks and then stops the timer. The native
/// panel renders a cached frame instead of running an animation loop, so the
/// same fixed tick count keeps captures byte-stable.
pub const DEFAULT_TICKS: usize = 300;

/// The graph panel settings.
///
/// The force defaults mirror `pkm_core::GraphConfig::default()` and the
/// documented values in `docs/getting-started/configuration.md` (charge `-30`,
/// link distance `100`, alpha decay `0.02`, velocity decay `0.4`, curvature
/// `0.15`). The shipped desktop panel loads these from the persisted vault
/// configuration, and the native panel does the same through
/// [`crate::provider::PanelData::graph_settings`].
#[derive(Clone, Debug, PartialEq)]
pub struct GraphSettings {
    /// Include notes with at least one connection.
    pub show_connected: bool,
    /// Include notes with no connections.
    pub show_orphaned: bool,
    /// Show tag labels on hover (settings parity; it does not filter).
    pub show_tags: bool,
    /// Many-body charge strength.
    pub charge_strength: f64,
    /// Link rest distance.
    pub link_distance: f64,
    /// Alpha decay per tick.
    pub alpha_decay: f64,
    /// Velocity decay (friction).
    pub velocity_decay: f64,
    /// Link curvature.
    pub link_curvature: f64,
    /// Explicit node cap (`0` means unlimited).
    pub node_cap: usize,
    /// Render the graph in 3D.
    ///
    /// This is the one intentional default change from the shipped desktop
    /// panel (`use_3d: false`): the native shell always renders the 3D graph and
    /// only switches to an orthographic view when the user disables it.
    pub use_3d: bool,
}

impl Default for GraphSettings {
    fn default() -> Self {
        Self {
            show_connected: true,
            show_orphaned: true,
            show_tags: true,
            charge_strength: -30.0,
            link_distance: 100.0,
            alpha_decay: 0.02,
            velocity_decay: 0.4,
            link_curvature: 0.15,
            node_cap: 0,
            use_3d: true,
        }
    }
}

impl GraphSettings {
    /// Build the persisted settings row for the current panel state.
    ///
    /// `node_cap` is client-side only in the shipped app, so it is not carried
    /// by the row and stays in memory exactly like the React panel.
    pub fn to_row(&self) -> GraphSettingsRow {
        GraphSettingsRow {
            show_connected: self.show_connected,
            show_orphaned: self.show_orphaned,
            show_tags: self.show_tags,
            charge_strength: self.charge_strength,
            link_distance: self.link_distance,
            alpha_decay: self.alpha_decay,
            velocity_decay: self.velocity_decay,
            link_curvature: self.link_curvature,
            use_3d: self.use_3d,
        }
    }

    /// Build panel settings from a persisted settings row.
    ///
    /// The node cap always starts at `0` because the shipped app never persists
    /// it. Callers that need the native 3D default (the shipped desktop always
    /// renders 3D) override `use_3d` after mapping the row.
    pub fn from_row(row: GraphSettingsRow) -> Self {
        Self {
            show_connected: row.show_connected,
            show_orphaned: row.show_orphaned,
            show_tags: row.show_tags,
            charge_strength: row.charge_strength,
            link_distance: row.link_distance,
            alpha_decay: row.alpha_decay,
            velocity_decay: row.velocity_decay,
            link_curvature: row.link_curvature,
            node_cap: 0,
            use_3d: row.use_3d,
        }
    }

    /// Force parameters for the current settings.
    pub fn force_params(&self) -> ForceParams {
        ForceParams {
            charge_strength: self.charge_strength,
            link_distance: self.link_distance,
            alpha_decay: self.alpha_decay,
            velocity_decay: self.velocity_decay,
            link_curvature: self.link_curvature,
            ..ForceParams::default()
        }
    }

    /// Scene options for the current settings.
    pub fn scene_options(&self) -> SceneOptions {
        SceneOptions {
            link_curvature: self.link_curvature as f32,
            ..SceneOptions::default()
        }
    }

    /// Visibility toggles for the filter request.
    pub fn visibility(&self) -> GraphVisibility {
        GraphVisibility {
            show_connected: self.show_connected,
            show_orphaned: self.show_orphaned,
            show_tags: self.show_tags,
        }
    }

    /// Build the filter request for the current toolbar state.
    pub fn filter_request(
        &self,
        view_mode: ViewMode,
        selected_component: usize,
        search: &str,
    ) -> GraphFilterRequest {
        GraphFilterRequest {
            view_mode,
            selected_component,
            search: search.to_string(),
            visibility: self.visibility(),
            node_cap: self.node_cap,
        }
    }
}

/// Deterministic graph panel start state for the capture harness.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphStart {
    /// Initial view mode (`STRATUM_GRAPH_MODE`).
    pub view_mode: Option<ViewMode>,
    /// Initial component index (`STRATUM_GRAPH_COMPONENT`).
    pub component: usize,
    /// Open the settings panel (`STRATUM_GRAPH_SETTINGS=1`).
    pub settings_open: bool,
    /// Initial search text (`STRATUM_GRAPH_SEARCH`).
    pub search: Option<String>,
    /// Simulation ticks before the first frame (`STRATUM_GRAPH_TICKS`).
    pub ticks: usize,
}

impl Default for GraphStart {
    fn default() -> Self {
        Self {
            view_mode: None,
            component: 0,
            settings_open: false,
            search: None,
            ticks: DEFAULT_TICKS,
        }
    }
}

/// Parse a `STRATUM_GRAPH_MODE` value.
pub fn view_mode_from_env(value: &str) -> Option<ViewMode> {
    match value.to_ascii_lowercase().as_str() {
        "full" => Some(ViewMode::Full),
        "component" => Some(ViewMode::Component),
        "orphans" => Some(ViewMode::Orphans),
        _ => None,
    }
}

/// The graph panel load state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GraphStatus {
    /// Provider data has not been requested yet.
    Idle,
    /// A provider request is running (provider calls are synchronous, so this
    /// is only visible while the constructor runs).
    Loading,
    /// The graph loaded and is ready to render.
    Ready,
    /// The vault has no notes.
    Empty,
    /// Loading failed with a stable message.
    Failed(String),
}

/// The localized node-cap banner, when a cap is limiting the rendered set.
///
/// The shipped panel appends a hardcoded English clause for the safe-default
/// case; the native panel keeps only the catalog text so every rendered string
/// resolves through the locale catalogs.
pub fn banner_text(filtered: &FilteredGraph) -> Option<String> {
    if !filtered.node_cap_active {
        return None;
    }
    let cap_key = if filtered.default_cap_active {
        "graph._large_vault_capped_to"
    } else {
        "graph._cap"
    };
    let cap = format!(" {} {})", t_global(cap_key), filtered.effective_cap);
    let count = filtered.nodes.len().to_string();
    let total = filtered.pre_cap_node_count.to_string();
    Some(tr_global(
        "graph.showing",
        &[("count", &count), ("total", &total), ("cap", &cap)],
    ))
}

/// Toolbar node/edge/orphan counter.
pub fn stats_text(filtered: &FilteredGraph, total_nodes: usize, orphans: usize) -> String {
    let nodes = filtered.nodes.len();
    let edges = filtered.edges.len();
    let mut text = format!("{nodes}/{total_nodes} {}", t_global("graph.n"));
    text.push_str(&format!("{edges} e"));
    if orphans > 0 {
        text.push_str(&format!(" · {orphans} o"));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::core::prelude::v1::test;

    fn filtered(node_cap_active: bool, default_cap_active: bool, cap: usize) -> FilteredGraph {
        FilteredGraph {
            nodes: Vec::new(),
            edges: Vec::new(),
            pre_cap_node_count: 12,
            total_before_batch: 4,
            effective_cap: cap,
            default_cap_active,
            node_cap_active,
        }
    }

    #[test]
    fn default_settings_match_the_canonical_graph_config() {
        let settings = GraphSettings::default();
        let params = settings.force_params();
        assert_eq!(params.charge_strength, -30.0);
        assert_eq!(params.link_distance, 100.0);
        assert_eq!(params.alpha_decay, 0.02);
        assert_eq!(params.velocity_decay, 0.4);
        assert_eq!(params.link_curvature, 0.15);
        assert!(settings.use_3d);
        assert_eq!(settings.node_cap, 0);
    }

    #[test]
    fn settings_row_round_trips_without_the_node_cap() {
        let settings = GraphSettings {
            show_connected: false,
            show_orphaned: false,
            show_tags: false,
            charge_strength: -12.0,
            link_distance: 140.0,
            alpha_decay: 0.05,
            velocity_decay: 0.6,
            link_curvature: 0.3,
            node_cap: 500,
            use_3d: false,
        };
        let row = settings.to_row();
        let restored = GraphSettings::from_row(row.clone());
        assert_eq!(restored.to_row(), row);
        assert_eq!(restored.node_cap, 0);
        assert!(!restored.use_3d);
        // Every persisted field is carried; only the client-side cap is lost.
        assert_eq!(restored.charge_strength, settings.charge_strength);
        assert_eq!(restored.link_distance, settings.link_distance);
        assert_eq!(restored.alpha_decay, settings.alpha_decay);
        assert_eq!(restored.velocity_decay, settings.velocity_decay);
        assert_eq!(restored.link_curvature, settings.link_curvature);
    }

    #[test]
    fn filter_request_carries_the_toolbar_state() {
        let settings = GraphSettings::default();
        let request = settings.filter_request(ViewMode::Component, 2, "project");
        assert_eq!(request.view_mode, ViewMode::Component);
        assert_eq!(request.selected_component, 2);
        assert_eq!(request.search, "project");
        assert_eq!(request.node_cap, 0);
    }

    #[test]
    fn banner_is_absent_without_a_cap() {
        assert!(banner_text(&filtered(false, false, 0)).is_none());
    }

    #[test]
    fn banner_reports_explicit_and_default_caps() {
        let explicit = banner_text(&filtered(true, false, 500)).expect("banner");
        assert!(explicit.contains("500"), "{explicit}");
        let default = banner_text(&filtered(true, true, 2000)).expect("banner");
        assert!(default.contains("2000"), "{default}");
    }

    #[test]
    fn view_mode_parsing_is_case_insensitive() {
        assert_eq!(view_mode_from_env("FULL"), Some(ViewMode::Full));
        assert_eq!(view_mode_from_env("component"), Some(ViewMode::Component));
        assert_eq!(view_mode_from_env("Orphans"), Some(ViewMode::Orphans));
        assert_eq!(view_mode_from_env("nope"), None);
    }
}
