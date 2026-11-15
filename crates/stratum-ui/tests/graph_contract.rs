//! Window-free contract tests for the native graph surface.
//!
//! These tests exercise the pure graph model and pipeline the gpui panel
//! delegates to: the shipped default settings, the view-mode/visibility/cap
//! filter parity, the deterministic fixture graph, the node-cap banner and the
//! capture start-state parsing. They also walk [`GRAPH_I18N_KEYS`] against all
//! eight catalogs, so a visible string without a catalog entry fails the build
//! instead of leaking a raw key into the UI.

use stratum_graph::{
    filter_graph, Camera, FrameTimer, GraphFilterRequest, GraphVisibility, MoveKey, Vec3, ViewMode,
    CAMERA_DISTANCE2NODES_FACTOR, HISTOGRAM_EDGES_MS, PAN_SPEED, WORK_BUDGET_MS, ZOOM_SPEED,
};
use stratum_i18n::{t, Locale};
use stratum_ui::panels::graph::model::{
    banner_text, stats_text, view_mode_from_env, GraphSettings, GraphStart, DEFAULT_TICKS,
};
use stratum_ui::panels::graph::scene::{build_layout, start_layout, to_scene_data};
use stratum_ui::panels::graph::GRAPH_I18N_KEYS;
use stratum_ui::provider::{FixtureProvider, GraphSettingsRow, PanelData};

#[test]
fn graph_keys_are_present_in_every_catalog() {
    for locale in Locale::ALL {
        for &key in GRAPH_I18N_KEYS {
            assert_ne!(t(locale, key), key, "{key} missing in {}", locale.code());
        }
    }
}

#[test]
fn graph_keys_are_unique() {
    let mut seen = std::collections::HashSet::new();
    for key in GRAPH_I18N_KEYS {
        assert!(seen.insert(*key), "duplicate key: {key}");
    }
}

#[test]
fn graph_keys_cover_the_shipped_controls() {
    for key in [
        "graph.component",
        "graph.components",
        "graph.connected_notes",
        "graph.curvature",
        "graph.enable_3d_view",
        "graph.filter_by_title_tag",
        "graph.friction",
        "graph.full",
        "graph.graph_settings",
        "graph.graph_settings_label",
        "graph.groups_total",
        "graph.link_distance",
        "graph.node_cap",
        "graph.orphaned_notes",
        "graph.orphans",
        "graph.refresh",
        "graph.repulsion",
        "graph.showing",
        "graph.tags_on_hover",
        "graph.unlimited_auto_cap_for_large_vaults",
        "mobile_nav.graph",
    ] {
        assert!(
            GRAPH_I18N_KEYS.contains(&key),
            "{key} is missing from the release gate list"
        );
    }
}

#[test]
fn default_settings_match_the_canonical_graph_config() {
    let settings = GraphSettings::default();
    assert!(settings.show_connected);
    assert!(settings.show_orphaned);
    assert!(settings.show_tags);
    assert_eq!(settings.charge_strength, -30.0);
    assert_eq!(settings.link_distance, 100.0);
    assert_eq!(settings.alpha_decay, 0.02);
    assert_eq!(settings.velocity_decay, 0.4);
    assert_eq!(settings.link_curvature, 0.15);
    assert_eq!(settings.node_cap, 0);
    // The native panel intentionally defaults to the 3D camera.
    assert!(settings.use_3d);
}

#[test]
fn fixture_graph_settings_load_and_save_through_the_provider_seam() {
    let provider = FixtureProvider::new();
    let initial = provider
        .graph_settings()
        .expect("graph settings")
        .expect("row");
    assert_eq!(initial, GraphSettingsRow::default());
    assert_eq!(initial.link_distance, 100.0);
    assert_eq!(initial.alpha_decay, 0.02);

    let saved = GraphSettingsRow {
        show_orphaned: false,
        charge_strength: -45.0,
        link_distance: 140.0,
        alpha_decay: 0.05,
        use_3d: true,
        ..GraphSettingsRow::default()
    };
    provider
        .save_graph_settings(saved.clone())
        .expect("save graph settings");
    let loaded = provider
        .graph_settings()
        .expect("graph settings")
        .expect("row");
    assert_eq!(loaded, saved);

    let settings = GraphSettings::from_row(loaded);
    assert_eq!(settings.to_row(), saved);
    assert_eq!(settings.node_cap, 0);
}

#[test]
fn fixture_graph_has_the_documented_shape() {
    let provider = FixtureProvider::new();
    let data = provider.graph_panel_data().expect("fixture graph");
    assert_eq!(data.nodes.len(), 24);
    assert_eq!(data.edges.len(), 26);
    assert_eq!(data.components.len(), 2);
    assert_eq!(data.components[0].size, 12);
    assert_eq!(data.components[1].size, 9);
    assert_eq!(data.orphans.len(), 3);
    for orphan in &data.orphans {
        assert!(data.nodes.iter().any(|node| node.id == orphan.path));
    }
}

#[test]
fn full_mode_renders_every_fixture_node() {
    let provider = FixtureProvider::new();
    let data = to_scene_data(&provider.graph_panel_data().expect("fixture graph"));
    let filtered = filter_graph(&data, &GraphFilterRequest::default());
    assert_eq!(filtered.nodes.len(), 24);
    assert_eq!(filtered.edges.len(), 26);
    assert!(!filtered.node_cap_active);
    assert_eq!(filtered.effective_cap, 0);
}

#[test]
fn component_mode_selects_one_component() {
    let provider = FixtureProvider::new();
    let data = to_scene_data(&provider.graph_panel_data().expect("fixture graph"));
    let request = GraphFilterRequest {
        view_mode: ViewMode::Component,
        selected_component: 0,
        ..GraphFilterRequest::default()
    };
    let filtered = filter_graph(&data, &request);
    assert_eq!(filtered.nodes.len(), 12);
    assert_eq!(filtered.edges.len(), 15);
}

#[test]
fn orphan_mode_renders_only_unconnected_notes() {
    let provider = FixtureProvider::new();
    let data = to_scene_data(&provider.graph_panel_data().expect("fixture graph"));
    let request = GraphFilterRequest {
        view_mode: ViewMode::Orphans,
        ..GraphFilterRequest::default()
    };
    let filtered = filter_graph(&data, &request);
    assert_eq!(filtered.nodes.len(), 3);
    assert!(filtered.edges.is_empty());
}

#[test]
fn visibility_toggles_hide_orphans() {
    let provider = FixtureProvider::new();
    let data = to_scene_data(&provider.graph_panel_data().expect("fixture graph"));
    let request = GraphFilterRequest {
        visibility: GraphVisibility {
            show_connected: true,
            show_orphaned: false,
            show_tags: true,
        },
        ..GraphFilterRequest::default()
    };
    let filtered = filter_graph(&data, &request);
    assert_eq!(filtered.nodes.len(), 21);
}

#[test]
fn tag_search_keeps_only_matching_nodes() {
    let provider = FixtureProvider::new();
    let data = to_scene_data(&provider.graph_panel_data().expect("fixture graph"));
    let request = GraphFilterRequest {
        search: "project".to_string(),
        ..GraphFilterRequest::default()
    };
    let filtered = filter_graph(&data, &request);
    assert_eq!(filtered.nodes.len(), 6);
    assert!(filtered
        .nodes
        .iter()
        .all(|node| node.tags.iter().any(|tag| tag == "project")));
}

#[test]
fn explicit_cap_limits_the_rendered_set_and_shows_a_banner() {
    let provider = FixtureProvider::new();
    let data = to_scene_data(&provider.graph_panel_data().expect("fixture graph"));
    let request = GraphFilterRequest {
        node_cap: 5,
        ..GraphFilterRequest::default()
    };
    let filtered = filter_graph(&data, &request);
    assert_eq!(filtered.nodes.len(), 5);
    assert!(filtered.node_cap_active);
    assert!(!filtered.default_cap_active);
    let banner = banner_text(&filtered).expect("cap banner");
    assert!(banner.contains('5'), "{banner}");
}

#[test]
fn stats_report_nodes_edges_and_orphans() {
    let provider = FixtureProvider::new();
    let data = to_scene_data(&provider.graph_panel_data().expect("fixture graph"));
    let filtered = filter_graph(&data, &GraphFilterRequest::default());
    let text = stats_text(&filtered, data.graph.node_count, data.orphans.len());
    assert!(text.starts_with("24/24"), "{text}");
    assert!(text.contains("26 e"), "{text}");
    assert!(text.contains("3 o"), "{text}");
}

#[test]
fn start_state_defaults_are_deterministic() {
    let start = GraphStart::default();
    assert!(start.view_mode.is_none());
    assert_eq!(start.component, 0);
    assert!(!start.settings_open);
    assert!(start.search.is_none());
    assert_eq!(start.ticks, DEFAULT_TICKS);
}

#[test]
fn view_mode_parsing_accepts_the_capture_values() {
    assert_eq!(view_mode_from_env("full"), Some(ViewMode::Full));
    assert_eq!(view_mode_from_env("COMPONENT"), Some(ViewMode::Component));
    assert_eq!(view_mode_from_env("Orphans"), Some(ViewMode::Orphans));
    assert_eq!(view_mode_from_env("unknown"), None);
}

#[test]
fn camera_pan_and_zoom_match_the_shipped_trackballcontrols() {
    let mut camera = Camera::new(Vec3::new(0.0, 0.0, 200.0), Vec3::ZERO);
    let before = camera;
    camera.pan_pixels(40.0, 0.0, 720.0);
    let moved = camera.position.sub(before.position);
    let expected = 2.0 * 200.0 * (camera.fov_y * 0.5).tan() / 720.0 * PAN_SPEED as f32 * 40.0;
    assert!((moved.length() - expected).abs() < 1e-3);
    // Camera and target move by the same vector.
    assert!(moved.sub(camera.target.sub(before.target)).length() < 1e-6);

    let mut zoom = Camera::new(Vec3::new(0.0, 0.0, 200.0), Vec3::ZERO);
    zoom.zoom_steps(1.0);
    let distance = zoom.position.sub(zoom.target).length();
    let expected = 200.0 * 0.95_f32.powf(ZOOM_SPEED as f32);
    assert!((distance - expected).abs() < 1e-2);
    assert_eq!(zoom.target, Vec3::ZERO);
}

#[test]
fn wasd_steps_move_the_camera_horizontally_by_the_shipped_speed() {
    let mut camera = Camera::new(Vec3::new(0.0, 0.0, 10.0), Vec3::ZERO);
    let before_position = camera.position;
    let before_target = camera.target;
    camera.step(MoveKey::Right);
    assert!((camera.position.y - before_position.y).abs() < 1e-6);
    assert!((camera.position.sub(before_position).length() - 0.8).abs() < 1e-6);
    assert!((camera.target.sub(before_target).length() - 0.8).abs() < 1e-6);
}

#[test]
fn frame_histogram_buckets_cover_every_sample() {
    let mut timer = FrameTimer::new();
    for ms in [3.0, 9.0, 20.0, 200.0] {
        timer.record_ms(ms);
    }
    let report = timer.report(WORK_BUDGET_MS);
    let total: usize = report.histogram.iter().map(|bucket| bucket.count).sum();
    assert_eq!(total, report.frame_count);
    assert_eq!(
        report
            .histogram
            .iter()
            .map(|bucket| bucket.upper_ms)
            .collect::<Vec<_>>(),
        HISTOGRAM_EDGES_MS.to_vec()
    );
    assert_eq!(report.histogram[0].count, 1);
    assert_eq!(report.histogram[2].count, 1);
    assert_eq!(report.histogram[4].count, 1);
    assert_eq!(report.histogram[8].count, 1);
    assert!(report.histogram[8].upper_ms.is_infinite());
}

#[test]
fn fixture_layout_places_the_shipped_3d_force_graph_camera() {
    let provider = FixtureProvider::new();
    let data = to_scene_data(&provider.graph_panel_data().expect("fixture graph"));
    let filtered = filter_graph(&data, &GraphFilterRequest::default());
    let settings = GraphSettings::default();
    let layout = build_layout(&filtered, &settings, 24).expect("layout");
    let expected = (filtered.nodes.len() as f32).cbrt() * CAMERA_DISTANCE2NODES_FACTOR;
    assert_eq!(filtered.nodes.len(), 24);
    assert!((layout.camera.position.z - expected).abs() < 1e-3);
    assert!(layout.camera.position.x.abs() < 1e-6);
    assert!(layout.camera.position.y.abs() < 1e-6);
    assert_eq!(layout.camera.target, Vec3::ZERO);
    assert!(!layout.camera.is_orthographic());

    let settings = GraphSettings {
        use_3d: false,
        ..GraphSettings::default()
    };
    let flat = build_layout(&filtered, &settings, 24).expect("layout");
    assert!(flat.camera.is_orthographic());
}

#[test]
fn background_layout_matches_the_synchronous_layout() {
    let provider = FixtureProvider::new();
    let data = to_scene_data(&provider.graph_panel_data().expect("fixture graph"));
    let filtered = filter_graph(&data, &GraphFilterRequest::default());
    let settings = GraphSettings::default();
    let sync = build_layout(&filtered, &settings, 24).expect("sync layout");
    let job = start_layout(&filtered, &settings, 24).expect("layout job");
    let mut result = None;
    for _ in 0..2000 {
        if let Some(value) = job.poll() {
            result = Some(value.expect("scene"));
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    let result = result.expect("async layout");
    assert_eq!(result.positions, sync.positions);
    assert_eq!(result.scene.nodes.len(), sync.scene.nodes.len());
    assert!(!result.camera.is_orthographic());
}
