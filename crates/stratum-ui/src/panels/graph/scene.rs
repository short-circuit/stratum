//! Layout, scene and cached-frame pipeline for the native graph panel.
//!
//! The shipped panel hands the filtered graph to `d3-force-3d` and a three.js
//! renderer. The native panel reproduces the same pipeline headlessly: build a
//! [`stratum_graph::GraphData`] from the filtered set, run the deterministic
//! simulation for a fixed tick count, build the render [`Scene`], place the
//! shipped `3d-force-graph` camera and rasterize a cached [`stratum_graph::Frame`].
//!
//! The frame is converted to the BGRA [`RenderImage`] gpui paints. Re-rendering
//! only happens when the graph, settings or camera change, so the panel stays a
//! static cached image rather than a per-frame animation loop.

use std::sync::Arc;

use gpui_kit::*;
use image::RgbaImage;
use stratum_graph::{
    Camera, FilteredGraph, Frame, GraphData, GraphPanelData as SceneGraphPanelData, RenderConfig,
    Renderer, Scene, SimWorker, Simulation, SnapshotRequest, Vec3,
};

use crate::panels::graph::model::GraphSettings;
use crate::provider::{GraphEdgeRow, GraphNodeRow, GraphPanelData as ProviderGraphPanelData};

/// Fixed render width for the cached graph image.
pub const IMAGE_WIDTH: u32 = 960;
/// Fixed render height for the cached graph image.
pub const IMAGE_HEIGHT: u32 = 600;
/// Deterministic simulation seed (matches the fixture-oriented tests).
pub const SIM_SEED: u64 = 0x5eed;

/// Convert the provider DTO into the render crate's panel shape.
pub fn to_scene_data(panel: &ProviderGraphPanelData) -> SceneGraphPanelData {
    SceneGraphPanelData {
        graph: GraphData::new(
            panel.nodes.iter().map(scene_node).collect(),
            panel.edges.iter().map(scene_edge).collect(),
            panel.vault_path.clone(),
        ),
        components: panel
            .components
            .iter()
            .map(|component| stratum_graph::Component {
                nodes: component.nodes.iter().map(scene_node).collect(),
                size: component.size,
            })
            .collect(),
        orphans: panel
            .orphans
            .iter()
            .map(|orphan| stratum_graph::Orphan {
                slug: orphan.slug.clone(),
                title: orphan.title.clone(),
                path: orphan.path.clone(),
            })
            .collect(),
    }
}

/// Map one provider node row into the render crate node.
fn scene_node(node: &GraphNodeRow) -> stratum_graph::GraphNode {
    stratum_graph::GraphNode {
        id: node.id.clone(),
        title: node.title.clone(),
        path: node.path.clone(),
        tags: node.tags.clone(),
        degree: node.degree,
    }
}

/// Map one provider edge row into the render crate edge.
fn scene_edge(edge: &GraphEdgeRow) -> stratum_graph::GraphEdge {
    stratum_graph::GraphEdge {
        source: edge.source.clone(),
        target: edge.target.clone(),
        label: edge.label.clone(),
    }
}

/// The layout result: a renderable scene, the framed camera and node positions.
pub struct GraphLayout {
    /// Renderable scene.
    pub scene: Scene,
    /// Framed camera.
    pub camera: Camera,
    /// Simulated node positions used to build the scene.
    pub positions: Vec<Vec3>,
}

/// Build the scene for `filtered` and frame the camera.
///
/// Returns a stable error string when the simulation or scene construction
/// rejects the data (for example an edge whose endpoint is missing).
pub fn build_layout(
    filtered: &stratum_graph::FilteredGraph,
    settings: &GraphSettings,
    ticks: usize,
) -> Result<GraphLayout, String> {
    let data = GraphData::new(
        filtered.nodes.clone(),
        filtered.edges.clone(),
        String::new(),
    );
    let mut simulation = Simulation::new(&data, SIM_SEED).map_err(|error| error.to_string())?;
    simulation.set_params(settings.force_params());
    simulation.advance(ticks.max(1));
    let positions = simulation.positions();
    let scene = Scene::build(&data, &positions, &settings.scene_options())
        .map_err(|error| error.to_string())?;
    let mut camera = Camera::shipped_for_nodes(filtered.nodes.len());
    camera.set_orthographic(!settings.use_3d);
    Ok(GraphLayout {
        scene,
        camera,
        positions,
    })
}

/// Build the renderer for the fixed capture target.
///
/// The GPU backend is preferred; when no adapter is available the deterministic
/// software rasterizer is used so headless captures and CI stay reproducible.
pub fn build_renderer() -> Result<Renderer, String> {
    let config = RenderConfig {
        width: IMAGE_WIDTH,
        height: IMAGE_HEIGHT,
        ..RenderConfig::default()
    };
    Renderer::auto(config).map_err(|error| error.to_string())
}

/// A background layout job for large graphs.
///
/// The worker settles the simulation off the UI thread. Poll [`LayoutJob::poll`]
/// from a render frame; it returns `None` until the snapshot arrives.
pub struct LayoutJob {
    _worker: SimWorker,
    request: SnapshotRequest,
    data: GraphData,
    settings: GraphSettings,
}

/// The completed output of a [`LayoutJob`].
pub struct LayoutResult {
    /// Renderable scene.
    pub scene: Scene,
    /// Framed camera.
    pub camera: Camera,
    /// Simulated node positions used to build the scene.
    pub positions: Vec<Vec3>,
}

impl LayoutJob {
    /// Take the completed layout, if the worker has already replied.
    ///
    /// The outer `None` means the snapshot has not arrived yet; `Some(Err(_))`
    /// reports a scene-build failure so the panel can clear the job instead of
    /// polling a consumed snapshot forever.
    pub fn poll(&self) -> Option<Result<LayoutResult, String>> {
        let snapshot = self.request.try_take()?;
        let scene = match Scene::build(
            &self.data,
            &snapshot.positions,
            &self.settings.scene_options(),
        ) {
            Ok(scene) => scene,
            Err(error) => return Some(Err(error.to_string())),
        };
        let mut camera = Camera::shipped_for_nodes(snapshot.positions.len());
        camera.set_orthographic(!self.settings.use_3d);
        Some(Ok(LayoutResult {
            scene,
            camera,
            positions: snapshot.positions,
        }))
    }
}

/// Start a non-blocking layout for a large filtered graph.
pub fn start_layout(
    filtered: &FilteredGraph,
    settings: &GraphSettings,
    ticks: usize,
) -> Result<LayoutJob, String> {
    let data = GraphData::new(
        filtered.nodes.clone(),
        filtered.edges.clone(),
        String::new(),
    );
    let worker = SimWorker::spawn(data.clone(), SIM_SEED).map_err(|error| error.to_string())?;
    worker
        .set_params(settings.force_params())
        .map_err(|error| error.to_string())?;
    worker
        .tick(ticks.max(1))
        .map_err(|error| error.to_string())?;
    let request = worker.snapshot_async().map_err(|error| error.to_string())?;
    Ok(LayoutJob {
        _worker: worker,
        request,
        data,
        settings: settings.clone(),
    })
}

/// Rebuild a scene with an optional hover highlight.
pub fn rebuild_scene(
    data: &GraphData,
    positions: &[Vec3],
    settings: &GraphSettings,
    highlight: Option<usize>,
) -> Result<Scene, String> {
    let mut options = settings.scene_options();
    options.highlight = highlight;
    Scene::build(data, positions, &options).map_err(|error| error.to_string())
}

/// Rasterize `scene` from `camera` and convert it to a gpui render image.
pub fn render_image(
    scene: &Scene,
    camera: &Camera,
    renderer: &mut Renderer,
) -> Result<Arc<RenderImage>, String> {
    let frame = renderer
        .render(scene, camera)
        .map_err(|error| error.to_string())?;
    Ok(frame_image(&frame))
}

/// Convert an RGBA frame into the BGRA [`RenderImage`] gpui paints.
fn frame_image(frame: &Frame) -> Arc<RenderImage> {
    let mut rgba = frame.rgba();
    // gpui's RenderImage stores BGRA bytes; the graph renderer produces RGBA.
    for pixel in rgba.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
    let buffer = RgbaImage::from_raw(frame.width(), frame.height(), rgba)
        .expect("graph frame buffer matches its dimensions");
    Arc::new(RenderImage::new(vec![image::Frame::new(buffer)]))
}

/// World position of one scene node, used by the focus action.
pub fn node_position(scene: &Scene, index: usize) -> Option<Vec3> {
    scene.nodes.get(index).map(|node| node.position)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::{GraphComponentRow, GraphEdgeRow, GraphNodeRow, GraphPanelData};
    use ::core::prelude::v1::test;

    fn node(id: &str, degree: usize) -> GraphNodeRow {
        GraphNodeRow {
            id: id.into(),
            title: id.into(),
            path: format!("{id}.md"),
            tags: Vec::new(),
            degree,
        }
    }

    #[test]
    fn provider_rows_convert_to_the_scene_shape() {
        let panel = GraphPanelData {
            nodes: vec![node("a", 1), node("b", 1)],
            edges: vec![GraphEdgeRow {
                source: "a".into(),
                target: "b".into(),
                label: None,
            }],
            components: vec![GraphComponentRow {
                nodes: vec![node("a", 1), node("b", 1)],
                size: 2,
            }],
            orphans: Vec::new(),
            vault_path: "/fixture".into(),
        };
        let data = to_scene_data(&panel);
        assert_eq!(data.graph.node_count, 2);
        assert_eq!(data.graph.edge_count, 1);
        assert_eq!(data.components[0].size, 2);
    }

    #[test]
    fn layout_builds_a_scene_and_frames_the_camera() {
        let filtered = stratum_graph::FilteredGraph {
            nodes: vec![stratum_graph::GraphNode {
                id: "a".into(),
                title: "a".into(),
                path: "a.md".into(),
                tags: Vec::new(),
                degree: 1,
            }],
            edges: Vec::new(),
            pre_cap_node_count: 1,
            total_before_batch: 1,
            effective_cap: 0,
            default_cap_active: false,
            node_cap_active: false,
        };
        let layout = build_layout(&filtered, &GraphSettings::default(), 8).expect("layout");
        assert_eq!(layout.scene.nodes.len(), 1);
        assert!(!layout.camera.is_orthographic());
        assert_eq!(
            node_position(&layout.scene, 0),
            Some(layout.scene.nodes[0].position)
        );
        assert_eq!(node_position(&layout.scene, 9), None);
    }

    #[test]
    fn disabling_3d_switches_to_an_orthographic_camera() {
        let filtered = stratum_graph::FilteredGraph {
            nodes: vec![stratum_graph::GraphNode {
                id: "a".into(),
                title: "a".into(),
                path: "a.md".into(),
                tags: Vec::new(),
                degree: 0,
            }],
            edges: Vec::new(),
            pre_cap_node_count: 1,
            total_before_batch: 1,
            effective_cap: 0,
            default_cap_active: false,
            node_cap_active: false,
        };
        let settings = GraphSettings {
            use_3d: false,
            ..GraphSettings::default()
        };
        let layout = build_layout(&filtered, &settings, 4).expect("layout");
        assert!(layout.camera.is_orthographic());
    }

    #[test]
    fn async_layout_matches_the_synchronous_layout() {
        use crate::provider::{FixtureProvider, PanelData};
        use std::time::Duration;

        let provider = FixtureProvider::new();
        let panel = provider.graph_panel_data().expect("graph");
        let data = to_scene_data(&panel);
        let filtered =
            stratum_graph::filter_graph(&data, &stratum_graph::GraphFilterRequest::default());
        let settings = GraphSettings::default();
        let sync = build_layout(&filtered, &settings, 32).expect("sync layout");
        let job = start_layout(&filtered, &settings, 32).expect("job");
        let mut result = None;
        for _ in 0..2000 {
            if let Some(value) = job.poll() {
                result = Some(value.expect("scene"));
                break;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        let result = result.expect("async layout");
        assert_eq!(result.positions, sync.positions);
        assert_eq!(result.scene.nodes.len(), sync.scene.nodes.len());
        assert!(!result.camera.is_orthographic());
    }

    #[test]
    fn render_image_has_the_fixed_target_size() {
        let filtered = stratum_graph::FilteredGraph {
            nodes: vec![stratum_graph::GraphNode {
                id: "a".into(),
                title: "a".into(),
                path: "a.md".into(),
                tags: Vec::new(),
                degree: 0,
            }],
            edges: Vec::new(),
            pre_cap_node_count: 1,
            total_before_batch: 1,
            effective_cap: 0,
            default_cap_active: false,
            node_cap_active: false,
        };
        let layout = build_layout(&filtered, &GraphSettings::default(), 4).expect("layout");
        let mut renderer = build_renderer().expect("renderer");
        let image = render_image(&layout.scene, &layout.camera, &mut renderer).expect("image");
        let size = image.size(0);
        assert_eq!(size.width.0 as u32, IMAGE_WIDTH);
        assert_eq!(size.height.0 as u32, IMAGE_HEIGHT);
        assert_eq!(image.frame_count(), 1);
    }
}
