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
        let links = data
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
                })
            })
            .collect();
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
}
