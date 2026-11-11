//! Immutable seed data for the fixture panel surfaces.
//!
//! Everything here is deterministic: saved queries, reminder rows bucketed
//! around the local clock, plugin rows, catalog snapshots and built-in theme
//! values. The mutable copies live in [`super::PanelState`].

use std::collections::BTreeMap;

use chrono::{Duration, Local, Utc};

use stratum_app::plugins::catalog::{PluginCatalogEntry, ThemeCatalogEntry};

use crate::provider::panels::{
    GraphComponentRow, GraphEdgeRow, GraphNodeRow, GraphOrphanRow, GraphPanelData,
    InstalledThemeRow, PluginRow, ReminderRow, SavedQueryRow, ThemeValuesRow,
};

/// A stable 64-character sha256 placeholder for fixture catalog entries.
pub(super) const FIXTURE_SHA: &str =
    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
/// Built-in theme id understood by the fixture.
pub(super) const BUILTIN_THEME_ID: &str = "__builtin__";

/// The two seeded saved queries.
pub(super) fn fixture_saved_queries() -> Vec<SavedQueryRow> {
    vec![
        SavedQueryRow {
            name: "Open TODOs".to_string(),
            query: "{:query [:find ?b ?content :where [?b :block/marker \"TODO\"] [?b :block/content ?content]]}".to_string(),
            updated_at: "2026-11-09T09:00:00Z".to_string(),
        },
        SavedQueryRow {
            name: "Project pages".to_string(),
            query: "{:query [:find ?b ?content :where [?b :block/page ?p] [?p :page/title \"Project Notes\"]]}".to_string(),
            updated_at: "2026-11-08T18:00:00Z".to_string(),
        },
    ]
}

/// The two seeded plugin rows.
pub(super) fn fixture_plugins() -> Vec<PluginRow> {
    vec![
        PluginRow {
            id: "word-count".to_string(),
            name: "Word Count".to_string(),
            version: "1.2.0".to_string(),
            status: "ready".to_string(),
            enabled: true,
            author: "Stratum".to_string(),
            description: "Counts words in the active note.".to_string(),
            permissions: vec!["read_notes".to_string()],
            hooks: Vec::new(),
            error: None,
        },
        PluginRow {
            id: "auto-link".to_string(),
            name: "Auto Link".to_string(),
            version: "0.9.0".to_string(),
            status: "disabled".to_string(),
            enabled: false,
            author: "Stratum".to_string(),
            description: "Suggests wiki-links on save.".to_string(),
            permissions: vec!["read_notes".to_string(), "write_notes".to_string()],
            hooks: vec!["onSave".to_string(), "onOpen".to_string()],
            error: None,
        },
    ]
}

/// Build one freshly installed fixture plugin row.
pub(super) fn fixture_installed_plugin(id: &str) -> PluginRow {
    PluginRow {
        id: id.to_string(),
        name: id.to_string(),
        version: "1.0.0".to_string(),
        status: "ready".to_string(),
        enabled: true,
        author: "Stratum".to_string(),
        description: "Installed from the fixture catalog.".to_string(),
        permissions: vec!["read_notes".to_string()],
        hooks: Vec::new(),
        error: None,
    }
}

/// Five reminders: two overdue, one today and two upcoming.
pub(super) fn fixture_reminders(now: chrono::DateTime<Local>) -> Vec<ReminderRow> {
    let today = if (now + Duration::minutes(5)).date_naive() == now.date_naive() {
        now + Duration::minutes(5)
    } else {
        now + Duration::seconds(1)
    };
    vec![
        reminder(
            "overdue-1",
            "pages/fixture.md",
            "Fixture Note",
            "Review the fixture outline",
            now - Duration::hours(2),
        ),
        reminder(
            "overdue-2",
            "pages/Project Notes.md",
            "Project Notes",
            "Send the sprint update",
            now - Duration::minutes(30),
        ),
        reminder(
            "today-1",
            "pages/Reading List.md",
            "Reading List",
            "Read the offline knowledge chapter",
            today,
        ),
        reminder(
            "upcoming-1",
            "pages/Welcome.md",
            "Welcome",
            "Tour the new vault",
            now + Duration::days(1),
        ),
        reminder(
            "upcoming-2",
            "pages/fixture.md",
            "Fixture Note",
            "Plan next week's capture",
            now + Duration::days(2),
        ),
    ]
}

/// Build one fixture reminder row.
fn reminder(
    id: &str,
    page_path: &str,
    page_title: &str,
    content: &str,
    due: chrono::DateTime<Local>,
) -> ReminderRow {
    ReminderRow {
        block_id: format!("00000000-0000-5000-8000-{id:0>12}"),
        page_path: page_path.to_string(),
        page_title: Some(page_title.to_string()),
        content: content.to_string(),
        due: due.with_timezone(&Utc).to_rfc3339(),
    }
}

/// Three fixture plugin catalog entries.
pub(super) fn fixture_plugin_catalog() -> Vec<PluginCatalogEntry> {
    ["word-count", "auto-link", "graph-export"]
        .into_iter()
        .map(|id| PluginCatalogEntry {
            schema_version: 1,
            id: id.to_string(),
            name: format!("Plugin {id}"),
            version: "1.0.0".to_string(),
            description: format!("Fixture catalog entry for {id}."),
            author: Some("Stratum".to_string()),
            entry: "plugin.wasm".to_string(),
            permissions: vec!["read_notes".to_string()],
            hooks: Some(BTreeMap::new()),
            repo: "https://github.com/short-circuit/stratum-plugins".to_string(),
            source: None,
            sha256: FIXTURE_SHA.to_string(),
            download_url: format!("https://example.test/plugins/{id}.wasm"),
        })
        .collect()
}

/// Two fixture theme catalog entries.
pub(super) fn fixture_theme_catalog() -> Vec<ThemeCatalogEntry> {
    [
        ("midnight", true, "#0f172a", "#334155", 15.0),
        ("paper", false, "#f97316", "#6b7280", 16.0),
    ]
    .into_iter()
    .map(|(id, dark, primary, secondary, size)| ThemeCatalogEntry {
        id: id.to_string(),
        name: format!("Theme {id}"),
        version: "1.0.0".to_string(),
        description: format!("Fixture catalog entry for {id}."),
        author: Some("Stratum".to_string()),
        dark_mode: dark,
        primary_color: primary.to_string(),
        secondary_color: secondary.to_string(),
        font_size: size,
        repo: "https://github.com/short-circuit/stratum-plugins".to_string(),
        source: None,
        sha256: FIXTURE_SHA.to_string(),
        download_url: format!("https://example.test/themes/{id}.json"),
    })
    .collect()
}

/// Built-in theme values for the fixture.
pub(super) fn builtin_theme_values() -> ThemeValuesRow {
    ThemeValuesRow {
        dark_mode: true,
        primary_color: "#f97316".to_string(),
        secondary_color: "#6b7280".to_string(),
        font_size: 16.0,
    }
}

/// Deterministic graph fixture: 24 notes in two components (12 and 9 nodes)
/// plus three orphans, with tags that exercise the search filter.
pub(super) fn fixture_graph_panel_data() -> GraphPanelData {
    let mut nodes: Vec<GraphNodeRow> = (0..24)
        .map(|index| {
            let mut tags = Vec::new();
            if index % 4 == 0 {
                tags.push("project".to_string());
            }
            if index % 5 == 0 {
                tags.push("research".to_string());
            }
            GraphNodeRow {
                id: format!("pages/graph-{index:02}.md"),
                title: format!("Graph Note {index:02}"),
                path: format!("pages/graph-{index:02}.md"),
                tags,
                degree: 0,
            }
        })
        .collect();
    let mut edges = Vec::new();
    add_ring(&mut edges, &mut nodes, 0, 12);
    add_graph_edge(&mut edges, &mut nodes, 0, 6);
    add_graph_edge(&mut edges, &mut nodes, 2, 8);
    add_graph_edge(&mut edges, &mut nodes, 4, 10);
    add_ring(&mut edges, &mut nodes, 12, 9);
    add_graph_edge(&mut edges, &mut nodes, 12, 16);
    add_graph_edge(&mut edges, &mut nodes, 14, 19);
    let components = vec![
        GraphComponentRow {
            nodes: nodes[0..12].to_vec(),
            size: 12,
        },
        GraphComponentRow {
            nodes: nodes[12..21].to_vec(),
            size: 9,
        },
    ];
    let orphans = (21..24)
        .map(|index| GraphOrphanRow {
            slug: format!("graph-{index:02}"),
            title: format!("Graph Note {index:02}"),
            path: format!("pages/graph-{index:02}.md"),
        })
        .collect();
    GraphPanelData {
        nodes,
        edges,
        components,
        orphans,
        vault_path: "/fixture".to_string(),
    }
}

/// Add a ring of edges over `count` nodes starting at `start`.
fn add_ring(edges: &mut Vec<GraphEdgeRow>, nodes: &mut [GraphNodeRow], start: usize, count: usize) {
    for offset in 0..count {
        let source = start + offset;
        let target = start + (offset + 1) % count;
        add_graph_edge(edges, nodes, source, target);
    }
}

/// Add one fixture edge and update both endpoint degrees.
fn add_graph_edge(
    edges: &mut Vec<GraphEdgeRow>,
    nodes: &mut [GraphNodeRow],
    source: usize,
    target: usize,
) {
    edges.push(GraphEdgeRow {
        source: nodes[source].id.clone(),
        target: nodes[target].id.clone(),
        label: None,
    });
    nodes[source].degree += 1;
    nodes[target].degree += 1;
}

/// Build one installed-theme row for the fixture.
pub(super) fn installed_theme_row(id: &str, name: &str) -> InstalledThemeRow {
    InstalledThemeRow {
        id: id.to_string(),
        name: name.to_string(),
        active: true,
    }
}
