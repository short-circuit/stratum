//! Deterministic d3-force-3d simulation port.
//!
//! The tick order, alpha decay, force formulas, velocity integration and pinned
//! node behaviour mirror the shipped `d3-force-3d` configuration used by
//! `GraphCanvas.tsx`: charge `-30`, link distance `50`, collide radius `4`, a
//! `0.08` z-spring toward the seeded z, alpha decay `0.15` and velocity decay
//! `0.4`. [`Simulation::tick_sequential`] reproduces the JavaScript tick exactly
//! (including its single LCG stream); [`Simulation::tick`] uses a deterministic
//! per-node RNG for the Barnes-Hut apply phase so the parallel path remains
//! reproducible.

use crate::data::GraphData;
use crate::math::Vec3;
use crate::physics::init::Lcg;
use pkm_core::{PkmError, PkmResult};
use std::collections::HashMap;

/// Force parameters matching the shipped graph panel defaults.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ForceParams {
    /// Many-body charge strength (`-30` in the shipped panel).
    pub charge_strength: f64,
    /// Link rest distance (`50` in the shipped panel).
    pub link_distance: f64,
    /// Collide radius (`4`, the shipped `nodeRelSize`).
    pub collide_radius: f64,
    /// z-spring gain toward the seeded z (`0.08`).
    pub z_spring: f64,
    /// Alpha decay per tick (`0.15`).
    pub alpha_decay: f64,
    /// Velocity decay configured by the UI (`0.4`); the integrator uses `1 - this`.
    pub velocity_decay: f64,
    /// Alpha below which the simulation is considered cooled.
    pub alpha_min: f64,
}

impl Default for ForceParams {
    fn default() -> Self {
        Self {
            charge_strength: -30.0,
            link_distance: 50.0,
            collide_radius: 4.0,
            z_spring: 0.08,
            alpha_decay: 0.15,
            velocity_decay: 0.4,
            alpha_min: 0.001,
        }
    }
}

/// A simulated node with the same fields d3-force-3d exposes.
#[derive(Debug, Clone, PartialEq)]
pub struct SimNode {
    /// Stable node identity.
    pub id: String,
    /// Node index (assigned in source order).
    pub index: usize,
    /// X position.
    pub x: f64,
    /// Y position.
    pub y: f64,
    /// Z position.
    pub z: f64,
    /// X velocity.
    pub vx: f64,
    /// Y velocity.
    pub vy: f64,
    /// Z velocity.
    pub vz: f64,
    /// Pinned x position, when set.
    pub fx: Option<f64>,
    /// Pinned y position, when set.
    pub fy: Option<f64>,
    /// Pinned z position, when set.
    pub fz: Option<f64>,
    /// z-spring target (the shipped seeded z).
    pub z_target: f64,
}

/// A resolved link with precomputed bias and strength.
#[derive(Debug, Clone, Copy)]
pub(super) struct SimLink {
    pub(super) source: usize,
    pub(super) target: usize,
    pub(super) bias: f64,
    pub(super) strength: f64,
    pub(super) distance: f64,
}

/// A deterministic force simulation.
#[derive(Debug)]
pub struct Simulation {
    pub(super) nodes: Vec<SimNode>,
    pub(super) links: Vec<SimLink>,
    pub(super) params: ForceParams,
    pub(super) alpha: f64,
    pub(super) seed: u32,
    pub(super) ticks: u64,
    pub(super) rng: Lcg,
    pub(super) parallel: bool,
}

impl Simulation {
    /// Create a simulation from graph data, seeding every node with the shipped
    /// deterministic position hash.
    ///
    /// Returns [`PkmError::Internal`] when an edge references a missing node.
    pub fn new(data: &GraphData, seed: u64) -> PkmResult<Self> {
        let mut index_by_id = HashMap::with_capacity(data.nodes.len());
        for (index, node) in data.nodes.iter().enumerate() {
            index_by_id.insert(node.id.as_str(), index);
        }
        let mut nodes = Vec::with_capacity(data.nodes.len());
        for (index, node) in data.nodes.iter().enumerate() {
            let seed_position = crate::data::seed_position(&node.id);
            nodes.push(SimNode {
                id: node.id.clone(),
                index,
                x: f64::from(seed_position.x),
                y: f64::from(seed_position.y),
                z: f64::from(seed_position.z),
                vx: 0.0,
                vy: 0.0,
                vz: 0.0,
                fx: None,
                fy: None,
                fz: None,
                z_target: f64::from(seed_position.z),
            });
        }
        let mut counts = vec![0_usize; nodes.len()];
        let mut resolved = Vec::with_capacity(data.edges.len());
        for edge in &data.edges {
            let source = *index_by_id.get(edge.source.as_str()).ok_or_else(|| {
                PkmError::Internal(format!(
                    "graph edge references unknown node: {}",
                    edge.source
                ))
            })?;
            let target = *index_by_id.get(edge.target.as_str()).ok_or_else(|| {
                PkmError::Internal(format!(
                    "graph edge references unknown node: {}",
                    edge.target
                ))
            })?;
            counts[source] += 1;
            counts[target] += 1;
            resolved.push((source, target));
        }
        let params = ForceParams::default();
        let links = resolved
            .into_iter()
            .map(|(source, target)| {
                let denominator = counts[source] + counts[target];
                let bias = if denominator == 0 {
                    0.5
                } else {
                    counts[source] as f64 / denominator as f64
                };
                let minimum = counts[source].min(counts[target]).max(1);
                SimLink {
                    source,
                    target,
                    bias,
                    strength: 1.0 / minimum as f64,
                    distance: params.link_distance,
                }
            })
            .collect();
        Ok(Self {
            nodes,
            links,
            params,
            alpha: 1.0,
            seed: seed as u32,
            ticks: 0,
            rng: Lcg::new(1),
            parallel: true,
        })
    }

    /// Number of nodes.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Number of links.
    pub fn link_count(&self) -> usize {
        self.links.len()
    }

    /// Current alpha.
    pub fn alpha(&self) -> f64 {
        self.alpha
    }

    /// Number of completed ticks.
    pub fn ticks(&self) -> u64 {
        self.ticks
    }

    /// Force parameters.
    pub fn params(&self) -> &ForceParams {
        &self.params
    }

    /// Replace force parameters and refresh link distances.
    pub fn set_params(&mut self, params: ForceParams) {
        self.params = params;
        for link in &mut self.links {
            link.distance = params.link_distance;
        }
    }

    /// Whether the parallel Barnes-Hut apply path is enabled.
    pub fn parallel(&self) -> bool {
        self.parallel
    }

    /// Enable or disable the parallel Barnes-Hut apply path.
    pub fn set_parallel(&mut self, parallel: bool) {
        self.parallel = parallel;
    }

    /// Read a node by index.
    pub fn node(&self, index: usize) -> Option<&SimNode> {
        self.nodes.get(index)
    }

    /// Read a node mutably by index.
    pub fn node_mut(&mut self, index: usize) -> Option<&mut SimNode> {
        self.nodes.get_mut(index)
    }

    /// Set a node position.
    pub fn set_position(&mut self, index: usize, position: Vec3) -> PkmResult<()> {
        let node = self
            .nodes
            .get_mut(index)
            .ok_or_else(|| PkmError::Internal(format!("node index out of range: {index}")))?;
        node.x = f64::from(position.x);
        node.y = f64::from(position.y);
        node.z = f64::from(position.z);
        Ok(())
    }

    /// Set a node velocity.
    pub fn set_velocity(&mut self, index: usize, velocity: Vec3) -> PkmResult<()> {
        let node = self
            .nodes
            .get_mut(index)
            .ok_or_else(|| PkmError::Internal(format!("node index out of range: {index}")))?;
        node.vx = f64::from(velocity.x);
        node.vy = f64::from(velocity.y);
        node.vz = f64::from(velocity.z);
        Ok(())
    }

    /// Pin a node to `position`, or unpin it with `None`.
    pub fn pin(&mut self, index: usize, position: Option<Vec3>) -> PkmResult<()> {
        let node = self
            .nodes
            .get_mut(index)
            .ok_or_else(|| PkmError::Internal(format!("node index out of range: {index}")))?;
        match position {
            Some(position) => {
                node.fx = Some(f64::from(position.x));
                node.fy = Some(f64::from(position.y));
                node.fz = Some(f64::from(position.z));
            }
            None => {
                node.fx = None;
                node.fy = None;
                node.fz = None;
            }
        }
        Ok(())
    }

    /// Read a node position.
    pub fn position(&self, index: usize) -> Option<Vec3> {
        self.nodes
            .get(index)
            .map(|node| Vec3::new(node.x as f32, node.y as f32, node.z as f32))
    }

    /// Snapshot all node positions.
    pub fn positions(&self) -> Vec<Vec3> {
        self.nodes
            .iter()
            .map(|node| Vec3::new(node.x as f32, node.y as f32, node.z as f32))
            .collect()
    }

    /// Run one tick using the parallel Barnes-Hut apply path.
    pub fn tick(&mut self) {
        self.tick_inner(true);
    }

    /// Run one tick matching the JavaScript simulation exactly.
    pub fn tick_sequential(&mut self) {
        self.tick_inner(false);
    }

    /// Advance `count` ticks.
    pub fn advance(&mut self, count: usize) {
        for _ in 0..count {
            self.tick();
        }
    }

    /// Advance `count` ticks on the sequential parity path.
    pub fn advance_sequential(&mut self, count: usize) {
        for _ in 0..count {
            self.tick_sequential();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{fixture, GraphData};

    fn tiny() -> GraphData {
        fixture(20, 1)
    }

    #[test]
    fn simulation_seeds_positions_from_the_shipped_hash() {
        let sim = Simulation::new(&tiny(), 1).expect("simulation");
        assert_eq!(sim.node_count(), 20);
        let node = sim.node(0).expect("node");
        assert_eq!(node.id, "note-0000");
        assert_eq!(
            Vec3::new(node.x as f32, node.y as f32, node.z as f32),
            crate::data::seed_position("note-0000")
        );
    }

    #[test]
    fn sequential_and_parallel_paths_are_reproducible() {
        let data = tiny();
        let mut a = Simulation::new(&data, 1).expect("simulation");
        let mut b = Simulation::new(&data, 1).expect("simulation");
        a.advance_sequential(20);
        b.advance_sequential(20);
        assert_eq!(a.positions(), b.positions());

        let mut c = Simulation::new(&data, 1).expect("simulation");
        let mut d = Simulation::new(&data, 1).expect("simulation");
        c.advance(20);
        d.advance(20);
        assert_eq!(c.positions(), d.positions());
    }

    #[test]
    fn pinned_nodes_stay_put() {
        let mut sim = Simulation::new(&tiny(), 1).expect("simulation");
        sim.pin(0, Some(Vec3::new(1.0, 2.0, 3.0))).expect("pin");
        sim.advance(10);
        let node = sim.node(0).expect("node");
        assert_eq!((node.x, node.y, node.z), (1.0, 2.0, 3.0));
        assert_eq!(node.vx, 0.0);
    }

    #[test]
    fn unknown_edge_endpoint_is_an_error() {
        let data = GraphData::new(
            vec![crate::data::GraphNode {
                id: "a".into(),
                title: "a".into(),
                path: "a.md".into(),
                tags: vec![],
                degree: 0,
            }],
            vec![crate::data::GraphEdge {
                source: "a".into(),
                target: "missing".into(),
                label: None,
            }],
            "/v",
        );
        assert!(Simulation::new(&data, 1).is_err());
    }
}
