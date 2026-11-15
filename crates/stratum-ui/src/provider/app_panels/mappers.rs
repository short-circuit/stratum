//! Service-DTO to panel-row mappers for the service-backed provider.
//!
//! Split out of `app_panels.rs` to keep every provider source file under the
//! repository length limit.

use stratum_app::settings::GraphSettingsDto;

use super::super::panels::{GraphEdgeRow, GraphNodeRow, GraphSettingsRow, PluginRow};

/// Map a service graph settings DTO into a panel row.
pub(super) fn graph_settings_row(settings: GraphSettingsDto) -> GraphSettingsRow {
    GraphSettingsRow {
        show_connected: settings.show_connected,
        show_orphaned: settings.show_orphaned,
        show_tags: settings.show_tags,
        charge_strength: settings.charge_strength,
        link_distance: settings.link_distance,
        alpha_decay: settings.alpha_decay,
        velocity_decay: settings.velocity_decay,
        link_curvature: settings.link_curvature,
        use_3d: settings.use_3d,
    }
}

/// Map a service graph node into a panel row.
pub(super) fn graph_node_row(node: &stratum_app::graph::GraphNodeDto) -> GraphNodeRow {
    GraphNodeRow {
        id: node.id.clone(),
        title: node.title.clone(),
        path: node.path.clone(),
        tags: node.tags.clone(),
        degree: node.degree,
    }
}

/// Map a service graph edge into a panel row.
pub(super) fn graph_edge_row(edge: &stratum_app::graph::GraphEdgeDto) -> GraphEdgeRow {
    GraphEdgeRow {
        source: edge.source.clone(),
        target: edge.target.clone(),
        label: edge.label.clone(),
    }
}

/// Map a service plugin record into a panel row.
pub(super) fn plugin_row(info: stratum_app::plugins::PluginInfo) -> PluginRow {
    PluginRow {
        id: info.id,
        name: info.name,
        version: info.version,
        status: info.status,
        enabled: info.enabled,
        author: info.author,
        description: info.description,
        permissions: info.permissions,
        hooks: info.hooks,
        error: info.error,
    }
}
