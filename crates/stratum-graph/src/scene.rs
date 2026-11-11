//! Scene construction and CPU picking helpers.
//!
//! A [`Scene`] is the renderer-facing snapshot of the graph: node positions and
//! colours, link widths, label text and the highlight/dim state. Building it is
//! pure and deterministic, which lets the render smoke tests compare byte output
//! across identical inputs and lets the golden runner project nodes for picking.

use crate::camera::Camera;
use crate::data::{node_color, GraphData, DIM_COLOR};
use crate::math::Vec3;
use pkm_core::{PkmError, PkmResult};

/// The shipped node radius (`nodeRelSize`).
pub const NODE_RADIUS: f32 = 4.0;
/// The shipped default label text height.
pub const LABEL_HEIGHT: f32 = 6.0;
/// The shipped default label cap.
pub const MAX_LABELS: usize = 2048;
/// The shipped `DEFAULT_SETTINGS.link_curvature` value.
pub const DEFAULT_LINK_CURVATURE: f32 = 0.15;

pub use crate::curve::{
    link_curve_control, link_curve_points, link_perpendicular, LINK_CURVE_SEGMENTS,
};

/// Default graph background colour (sRGB bytes).
pub const DEFAULT_BACKGROUND: [u8; 4] = [0x0b, 0x0f, 0x14, 0xff];
/// Default label/link colour (sRGB bytes).
pub const DEFAULT_TEXT_COLOR: [u8; 3] = [0xff, 0xff, 0xff];

/// A renderable node.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneNode {
    /// Source node index.
    pub index: usize,
    /// World position.
    pub position: Vec3,
    /// sRGB colour.
    pub color: [u8; 3],
    /// Opacity (`0..1`).
    pub opacity: f32,
    /// Sphere radius.
    pub radius: f32,
}

/// A renderable link.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneLink {
    /// Source node index.
    pub source: usize,
    /// Target node index.
    pub target: usize,
    /// sRGB colour.
    pub color: [u8; 3],
    /// Line width in world units.
    pub width: f32,
    /// Opacity (`0..1`).
    pub opacity: f32,
    /// Signed perpendicular control-point offset in world units.
    ///
    /// The shipped panel stores `link_curvature` in `DEFAULT_SETTINGS` and hands
    /// it to three.js as `linkCurvature`; the Rust port keeps the same render
    /// setting and resolves it to a world-space offset here. The value is
    /// `link_curvature * length` (sign alternating deterministically per link),
    /// so `0.0` produces a straight segment.
    pub curvature: f32,
}

/// A renderable label.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneLabel {
    /// Node the label belongs to.
    pub node: usize,
    /// Label text.
    pub text: String,
    /// World position (node centre).
    pub position: Vec3,
    /// Text height in world units.
    pub height: f32,
    /// sRGB colour.
    pub color: [u8; 3],
}

/// Scene construction options.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneOptions {
    /// Node to highlight; when set, other nodes and links are dimmed.
    pub highlight: Option<usize>,
    /// Maximum number of labels to emit (`0` disables labels).
    pub max_labels: usize,
    /// Label text height.
    pub label_height: f32,
    /// Link curvature setting (`0..`; the shipped default is `0.15`).
    ///
    /// Carried on the render options exactly like the shipped panel stores it in
    /// `DEFAULT_SETTINGS`; it is not a physics parameter even though the panel
    /// keeps it on the same settings struct.
    pub link_curvature: f32,
    /// Background sRGB bytes.
    pub background: [u8; 4],
    /// Text/link sRGB colour.
    pub text_color: [u8; 3],
}

impl Default for SceneOptions {
    fn default() -> Self {
        Self {
            highlight: None,
            max_labels: MAX_LABELS,
            label_height: LABEL_HEIGHT,
            link_curvature: DEFAULT_LINK_CURVATURE,
            background: DEFAULT_BACKGROUND,
            text_color: DEFAULT_TEXT_COLOR,
        }
    }
}

/// A deterministic render scene.
#[derive(Debug, Clone, PartialEq)]
pub struct Scene {
    /// Renderable nodes.
    pub nodes: Vec<SceneNode>,
    /// Renderable links.
    pub links: Vec<SceneLink>,
    /// Renderable labels.
    pub labels: Vec<SceneLabel>,
    /// Background sRGB bytes.
    pub background: [u8; 4],
    /// Text/link sRGB colour.
    pub text_color: [u8; 3],
}

impl Scene {
    /// Build a scene from graph data and simulated positions.
    ///
    /// `positions` must contain one entry per node. Returns
    /// [`PkmError::Internal`] when the lengths differ.
    pub fn build(data: &GraphData, positions: &[Vec3], options: &SceneOptions) -> PkmResult<Self> {
        if positions.len() != data.nodes.len() {
            return Err(PkmError::Internal(format!(
                "scene position count {} does not match node count {}",
                positions.len(),
                data.nodes.len()
            )));
        }
        let highlight_set = highlight_set(data, options.highlight);
        let nodes = data
            .nodes
            .iter()
            .enumerate()
            .map(|(index, node)| {
                let highlighted = options.highlight.is_none() || highlight_set[index];
                let color = if highlighted {
                    node_color(node)
                } else {
                    [DIM_COLOR[0], DIM_COLOR[1], DIM_COLOR[2]]
                };
                SceneNode {
                    index,
                    position: positions[index],
                    color,
                    opacity: if highlighted {
                        1.0
                    } else {
                        f32::from(DIM_COLOR[3]) / 255.0
                    },
                    radius: NODE_RADIUS,
                }
            })
            .collect();
        let mut index_by_id = std::collections::HashMap::with_capacity(data.nodes.len());
        for (index, node) in data.nodes.iter().enumerate() {
            index_by_id.insert(node.id.as_str(), index);
        }
        let mut links: Vec<SceneLink> = data
            .edges
            .iter()
            .filter_map(|edge| {
                let source = *index_by_id.get(edge.source.as_str())?;
                let target = *index_by_id.get(edge.target.as_str())?;
                let highlighted = match options.highlight {
                    None => false,
                    Some(selected) => {
                        highlight_set[source] && (source == selected || target == selected)
                    }
                };
                // The shipped accessor returns 0.5 without a hover selection,
                // 2 for links in the selected neighbourhood and 0.1 otherwise.
                let width = match options.highlight {
                    None => 0.5,
                    Some(_) if highlighted => 2.0,
                    Some(_) => 0.1,
                };
                Some(SceneLink {
                    source,
                    target,
                    color: options.text_color,
                    width,
                    opacity: 1.0,
                    curvature: 0.0,
                })
            })
            .collect();
        // Resolve the render-only curvature setting into a signed world-space
        // control offset. The link index seeds the deterministic perpendicular
        // direction, so no RNG state leaks between links or builds.
        for (link_index, link) in links.iter_mut().enumerate() {
            let length = positions[link.target].sub(positions[link.source]).length();
            if options.link_curvature == 0.0 || length <= f32::EPSILON {
                link.curvature = 0.0;
            } else {
                let sign = if link_index % 2 == 0 { 1.0 } else { -1.0 };
                link.curvature = options.link_curvature * length * sign;
            }
        }
        let labels = build_labels(data, positions, options);
        Ok(Self {
            nodes,
            links,
            labels,
            background: options.background,
            text_color: options.text_color,
        })
    }

    /// Project a node centre to screen pixels, if it is in front of the camera.
    pub fn project_node(
        &self,
        camera: &Camera,
        width: u32,
        height: u32,
        index: usize,
    ) -> Option<(f32, f32)> {
        let node = self.nodes.get(index)?;
        camera
            .project(node.position, width, height)
            .map(|(x, y, _)| (x, y))
    }

    /// CPU ray/sphere pick. Returns the closest node whose sphere is hit.
    pub fn pick_cpu(
        &self,
        camera: &Camera,
        x: f32,
        y: f32,
        width: u32,
        height: u32,
    ) -> Option<usize> {
        let (origin, direction) = camera.ray_from_screen(x, y, width, height);
        let mut best: Option<(f32, usize)> = None;
        for node in &self.nodes {
            let offset = node.position.sub(origin);
            let along = offset.dot(direction);
            if along <= 0.0 {
                continue;
            }
            let closest = origin.add(direction.scale(along));
            let distance_squared = closest.sub(node.position).length_squared();
            if distance_squared <= node.radius * node.radius
                && best.map(|(t, _)| along < t).unwrap_or(true)
            {
                best = Some((along, node.index));
            }
        }
        best.map(|(_, index)| index)
    }
}

fn highlight_set(data: &GraphData, highlight: Option<usize>) -> Vec<bool> {
    let mut set = vec![false; data.nodes.len()];
    let Some(selected) = highlight else {
        return vec![true; data.nodes.len()];
    };
    if selected >= set.len() {
        return vec![true; set.len()];
    }
    set[selected] = true;
    let mut index_by_id = std::collections::HashMap::with_capacity(data.nodes.len());
    for (index, node) in data.nodes.iter().enumerate() {
        index_by_id.insert(node.id.as_str(), index);
    }
    for edge in &data.edges {
        let source = index_by_id.get(edge.source.as_str()).copied();
        let target = index_by_id.get(edge.target.as_str()).copied();
        match (source, target) {
            (Some(source), Some(target)) if source == selected => set[target] = true,
            (Some(source), Some(target)) if target == selected => set[source] = true,
            _ => {}
        }
    }
    set
}

fn build_labels(data: &GraphData, positions: &[Vec3], options: &SceneOptions) -> Vec<SceneLabel> {
    if options.max_labels == 0 {
        return Vec::new();
    }
    let mut labels = Vec::new();
    for (index, node) in data.nodes.iter().enumerate() {
        if labels.len() >= options.max_labels {
            break;
        }
        if node.title.is_empty() {
            continue;
        }
        labels.push(SceneLabel {
            node: index,
            text: node.title.clone(),
            position: positions[index],
            height: options.label_height,
            color: options.text_color,
        });
    }
    labels
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::fixture;

    #[test]
    fn scene_dimension_mismatch_is_an_error() {
        let data = fixture(4, 1);
        assert!(Scene::build(&data, &[], &SceneOptions::default()).is_err());
    }

    #[test]
    fn highlight_dims_unrelated_nodes_and_links() {
        let data = fixture(10, 1);
        let positions = data.seed_positions();
        let options = SceneOptions {
            highlight: Some(0),
            ..SceneOptions::default()
        };
        let scene = Scene::build(&data, &positions, &options).expect("scene");
        assert_eq!(scene.nodes[0].opacity, 1.0);
        let dimmed = scene.nodes.iter().filter(|n| n.opacity < 1.0).count();
        assert!(dimmed > 0);
        assert!(scene.links.iter().any(|l| l.width == 2.0));
    }

    #[test]
    fn labels_are_capped() {
        let data = fixture(20, 1);
        let positions = data.seed_positions();
        let options = SceneOptions {
            max_labels: 5,
            ..SceneOptions::default()
        };
        let scene = Scene::build(&data, &positions, &options).expect("scene");
        assert_eq!(scene.labels.len(), 5);
    }

    #[test]
    fn project_and_pick_agree_for_visible_node() {
        let data = fixture(3, 1);
        let mut positions = data.seed_positions();
        positions[0] = Vec3::ZERO;
        let scene = Scene::build(&data, &positions, &SceneOptions::default()).expect("scene");
        let camera = Camera::default();
        let (x, y) = scene
            .project_node(&camera, 1280, 720, 0)
            .expect("node visible");
        assert_eq!(scene.pick_cpu(&camera, x, y, 1280, 720), Some(0));
        assert_eq!(scene.pick_cpu(&camera, 1.0, 1.0, 1280, 720), None);
    }

    fn single_link() -> GraphData {
        let node = |id: &str| crate::data::GraphNode {
            id: id.into(),
            title: id.into(),
            path: format!("{id}.md"),
            tags: vec![],
            degree: 1,
        };
        GraphData::new(
            vec![node("a"), node("b")],
            vec![crate::data::GraphEdge {
                source: "a".into(),
                target: "b".into(),
                label: None,
            }],
            "/fixture",
        )
    }

    #[test]
    fn link_curvature_offsets_the_control_point_perpendicular() {
        let data = single_link();
        let positions = vec![Vec3::new(-50.0, 0.0, 0.0), Vec3::new(50.0, 0.0, 0.0)];
        let options = SceneOptions {
            link_curvature: 0.15,
            ..SceneOptions::default()
        };
        let scene = Scene::build(&data, &positions, &options).expect("scene");
        let link = &scene.links[0];
        let start = positions[link.source];
        let end = positions[link.target];
        let length = end.sub(start).length();
        let expected = 0.15 * length;
        assert!(
            (link.curvature.abs() - expected).abs() < 1e-3,
            "curvature {} != {expected}",
            link.curvature
        );
        let midpoint = start.lerp(end, 0.5);
        let control = link_curve_control(start, end, link.curvature, 0);
        let offset = control.sub(midpoint);
        assert!(
            (offset.length() - expected).abs() < 1e-3,
            "control offset {} != {expected}",
            offset.length()
        );
        assert!(offset.dot(end.sub(start).normalized()).abs() < 1e-3);
        // The deterministic perpendicular is a unit vector orthogonal to the link.
        let perpendicular = link_perpendicular(end.sub(start), 0);
        assert!((perpendicular.length() - 1.0).abs() < 1e-3);
        assert!(perpendicular.dot(end.sub(start).normalized()).abs() < 1e-3);
    }

    #[test]
    fn zero_curvature_keeps_the_link_straight() {
        let data = single_link();
        let positions = vec![Vec3::new(-50.0, 0.0, 0.0), Vec3::new(50.0, 0.0, 0.0)];
        let options = SceneOptions {
            link_curvature: 0.0,
            ..SceneOptions::default()
        };
        let scene = Scene::build(&data, &positions, &options).expect("scene");
        let link = &scene.links[0];
        assert_eq!(link.curvature, 0.0);
        let start = positions[link.source];
        let end = positions[link.target];
        let midpoint = start.lerp(end, 0.5);
        assert_eq!(link_curve_control(start, end, 0.0, 0), midpoint);
        let points = link_curve_points(start, end, 0.0, 0);
        let direction = end.sub(start).normalized();
        for point in points {
            let offset = point.sub(midpoint);
            assert!(offset.dot(direction).abs() <= offset.length() + 1e-4);
            assert!(offset.cross(direction).length() < 1e-3);
        }
    }
}
