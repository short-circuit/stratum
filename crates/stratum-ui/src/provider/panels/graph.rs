//! Graph panel data rows and the graph response.
//!
//! Extracted from [`super`] so the panel data contract module stays inside the
//! repository file-length limit. The rows mirror `stratum_app::graph` DTOs; the
//! provider seam maps the service response into them. [`GraphSettingsRow`]
//! mirrors `stratum_app::settings::GraphSettingsDto` for the persisted graph
//! settings seam.

/// One graph node row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphNodeRow {
    /// Stable node identity (vault-relative page path).
    pub id: String,
    /// Display title.
    pub title: String,
    /// Vault-relative page path.
    pub path: String,
    /// Tags attached to the page.
    pub tags: Vec<String>,
    /// Number of incident edges.
    pub degree: usize,
}

/// One directed graph edge row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphEdgeRow {
    /// Source node id.
    pub source: String,
    /// Target node id.
    pub target: String,
    /// Optional edge label.
    pub label: Option<String>,
}

/// One connected component row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphComponentRow {
    /// Nodes in the component.
    pub nodes: Vec<GraphNodeRow>,
    /// Component size.
    pub size: usize,
}

/// One orphaned-note row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphOrphanRow {
    /// Basename slug.
    pub slug: String,
    /// Display title.
    pub title: String,
    /// Vault-relative path.
    pub path: String,
}

/// The graph panel response: nodes, edges, components and orphans.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GraphPanelData {
    /// Graph nodes in source order.
    pub nodes: Vec<GraphNodeRow>,
    /// Graph edges in source order.
    pub edges: Vec<GraphEdgeRow>,
    /// Connected components.
    pub components: Vec<GraphComponentRow>,
    /// Orphaned notes.
    pub orphans: Vec<GraphOrphanRow>,
    /// Vault path that produced the graph.
    pub vault_path: String,
}

impl GraphPanelData {
    /// An empty graph, used by providers without graph support.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Whether the response contains no nodes.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

/// The persisted graph settings, mirroring
/// `stratum_app::settings::GraphSettingsDto`.
///
/// The defaults are the canonical `pkm_core::GraphConfig` values documented in
/// `docs/getting-started/configuration.md`. `node_cap` is intentionally absent:
/// the shipped app keeps it client-side only.
#[derive(Clone, Debug, PartialEq)]
pub struct GraphSettingsRow {
    /// Include notes with at least one connection.
    pub show_connected: bool,
    /// Include notes with no connections.
    pub show_orphaned: bool,
    /// Show tag labels on hover.
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
    /// Opt-in 3D rendering (mobile); desktop always renders 3D.
    pub use_3d: bool,
}

impl Default for GraphSettingsRow {
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
            use_3d: false,
        }
    }
}
