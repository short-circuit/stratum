//! Tick phases for the deterministic simulation.
//!
//! The phase order matches `d3-force-3d`: alpha decay, many-body charge, link
//! spring, collide, the shipped z-spring and finally the velocity integration.
//! [`Simulation::tick_inner`] is shared by the sequential parity path and the
//! parallel path; only the many-body apply loop differs between them.

use crate::physics::barnes_hut::CollideNode;
use crate::physics::init::Lcg;
use crate::physics::octree::Octree;
use crate::physics::sim::Simulation;
use rayon::prelude::*;

impl Simulation {
    /// Run one tick, selecting the parallel many-body path when requested.
    pub(super) fn tick_inner(&mut self, parallel: bool) {
        let alpha = self.alpha + (0.0 - self.alpha) * self.params.alpha_decay;
        self.alpha = alpha;
        let points: Vec<(f64, f64, f64)> = self
            .nodes
            .iter()
            .map(|node| (node.x, node.y, node.z))
            .collect();
        let strengths = vec![self.params.charge_strength; self.nodes.len()];
        let mut tree = Octree::build(&points);
        tree.accumulate(&strengths, &points);
        if parallel && self.nodes.len() >= 128 {
            let seed = self.seed;
            let tick = self.ticks;
            self.nodes.par_iter_mut().for_each(|node| {
                let mut rng = per_node_rng(seed, tick, node.index);
                tree.apply_many_body(node, &strengths, alpha, &mut rng);
            });
        } else {
            for node in self.nodes.iter_mut() {
                tree.apply_many_body(node, &strengths, alpha, &mut self.rng);
            }
        }
        self.apply_links(alpha);
        self.apply_collide();
        self.apply_z_spring();
        self.integrate();
        self.ticks += 1;
    }

    fn apply_links(&mut self, alpha: f64) {
        let links = &self.links;
        for link in links {
            let (sx, sy, sz, svx, svy, svz) = {
                let source = &self.nodes[link.source];
                (
                    source.x, source.y, source.z, source.vx, source.vy, source.vz,
                )
            };
            let (tx, ty, tz, tvx, tvy, tvz) = {
                let target = &self.nodes[link.target];
                (
                    target.x, target.y, target.z, target.vx, target.vy, target.vz,
                )
            };
            let mut x = tx + tvx - sx - svx;
            if x == 0.0 {
                x = self.rng.jiggle();
            }
            let mut y = ty + tvy - sy - svy;
            if y == 0.0 {
                y = self.rng.jiggle();
            }
            let mut z = tz + tvz - sz - svz;
            if z == 0.0 {
                z = self.rng.jiggle();
            }
            let mut length = (x * x + y * y + z * z).sqrt();
            length = (length - link.distance) / length * alpha * link.strength;
            x *= length;
            y *= length;
            z *= length;
            let bias = link.bias;
            let target = &mut self.nodes[link.target];
            target.vx -= x * bias;
            target.vy -= y * bias;
            target.vz -= z * bias;
            let source = &mut self.nodes[link.source];
            source.vx += x * (1.0 - bias);
            source.vy += y * (1.0 - bias);
            source.vz += z * (1.0 - bias);
        }
    }

    fn apply_collide(&mut self) {
        let radii = vec![self.params.collide_radius; self.nodes.len()];
        let points: Vec<(f64, f64, f64)> = self
            .nodes
            .iter()
            .map(|node| (node.x + node.vx, node.y + node.vy, node.z + node.vz))
            .collect();
        let mut state: Vec<CollideNode> = self
            .nodes
            .iter()
            .map(|node| CollideNode {
                x: node.x,
                y: node.y,
                z: node.z,
                vx: node.vx,
                vy: node.vy,
                vz: node.vz,
            })
            .collect();
        let mut tree = Octree::build(&points);
        tree.prepare_radii(&radii);
        for index in 0..state.len() {
            let xi = state[index].x + state[index].vx;
            let yi = state[index].y + state[index].vy;
            let zi = state[index].z + state[index].vz;
            tree.apply_collide(&mut state, index, xi, yi, zi, &radii, &mut self.rng);
        }
        for (node, collide) in self.nodes.iter_mut().zip(state) {
            node.vx = collide.vx;
            node.vy = collide.vy;
            node.vz = collide.vz;
        }
    }

    fn apply_z_spring(&mut self) {
        let gain = self.params.z_spring;
        for node in &mut self.nodes {
            node.vz += (node.z_target - node.z) * gain;
        }
    }

    fn integrate(&mut self) {
        let damping = 1.0 - self.params.velocity_decay;
        for node in &mut self.nodes {
            if let Some(fx) = node.fx {
                node.x = fx;
                node.vx = 0.0;
            } else {
                node.vx *= damping;
                node.x += node.vx;
            }
            if let Some(fy) = node.fy {
                node.y = fy;
                node.vy = 0.0;
            } else {
                node.vy *= damping;
                node.y += node.vy;
            }
            if let Some(fz) = node.fz {
                node.z = fz;
                node.vz = 0.0;
            } else {
                node.vz *= damping;
                node.z += node.vz;
            }
        }
    }
}

/// Deterministic per-node RNG for the parallel apply phase.
fn per_node_rng(seed: u32, tick: u64, index: usize) -> Lcg {
    let mut value = u64::from(seed)
        ^ tick.wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (index as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^= value >> 31;
    Lcg::new(value as u32)
}
