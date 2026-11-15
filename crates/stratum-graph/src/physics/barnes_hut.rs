//! `d3-force-3d` many-body and collide force kernels over the octree.
//!
//! The Barnes-Hut traversal order and force formulas mirror the JavaScript
//! sources byte-for-byte where possible: children are visited in ascending
//! octant order, coincident leaves form a newest-first chain, and the jiggle
//! stream is consumed in the same sequence. The sequential tick path therefore
//! reproduces the shipped simulation, while the parallel path only substitutes
//! a per-node deterministic RNG for the jiggle term.

use crate::physics::init::Lcg;
use crate::physics::octree::{Bounds, OctNode, Octree};
use crate::physics::sim::SimNode;

/// Theta-squared used by the shipped many-body force (`0.9^2`).
pub const THETA2: f64 = 0.81;
/// Minimum squared distance used by the many-body force.
const DISTANCE_MIN2: f64 = 1.0;

/// Mutable collide state (positions and velocities) used during the collide
/// pass. Kept separate so pairwise updates can be applied in d3's exact order.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct CollideNode {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub vx: f64,
    pub vy: f64,
    pub vz: f64,
}

impl Octree {
    /// Apply the many-body force to one node.
    pub(crate) fn apply_many_body(
        &self,
        node: &mut SimNode,
        strengths: &[f64],
        alpha: f64,
        rng: &mut Lcg,
    ) {
        if let Some(root) = self.root.as_ref() {
            apply_rec(root, self.bounds, node, strengths, alpha, rng);
        }
    }

    /// Apply the collide force to node `node_index` in `state`.
    ///
    /// `xi/yi/zi` are the predicted positions captured before the leaf loop,
    /// matching d3's per-node capture.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn apply_collide(
        &self,
        state: &mut [CollideNode],
        node_index: usize,
        xi: f64,
        yi: f64,
        zi: f64,
        radii: &[f64],
        rng: &mut Lcg,
    ) {
        if let Some(root) = self.root.as_ref() {
            collide_rec(root, self.bounds, state, node_index, xi, yi, zi, radii, rng);
        }
    }
}

fn apply_rec(
    tree: &OctNode,
    bounds: Bounds,
    node: &mut SimNode,
    strengths: &[f64],
    alpha: f64,
    rng: &mut Lcg,
) -> bool {
    if tree.value == 0.0 || tree.value.is_nan() {
        return true;
    }
    let mut x = tree.x - node.x;
    let mut y = tree.y - node.y;
    let mut z = tree.z - node.z;
    let w = bounds.x1 - bounds.x0;
    let mut l = x * x + y * y + z * z;
    if w * w / THETA2 < l {
        if x == 0.0 {
            x = rng.jiggle();
            l += x * x;
        }
        if y == 0.0 {
            y = rng.jiggle();
            l += y * y;
        }
        if z == 0.0 {
            z = rng.jiggle();
            l += z * z;
        }
        if l < DISTANCE_MIN2 {
            l = (DISTANCE_MIN2 * l).sqrt();
        }
        node.vx += x * tree.value * alpha / l;
        node.vy += y * tree.value * alpha / l;
        node.vz += z * tree.value * alpha / l;
        return true;
    }
    if tree.is_internal() {
        // `d3-octree` pushes children in descending octant order and pops them
        // in ascending order, so the visit order is a depth-first pre-order
        // starting with child 0. Reproduce that order here because the jiggle
        // stream and the velocity updates are order-sensitive.
        if let Some(children) = &tree.children {
            for (index, child) in children.iter().enumerate() {
                if let Some(child) = child {
                    apply_rec(child, bounds.child(index), node, strengths, alpha, rng);
                }
            }
        }
        return false;
    }
    let has_chain = tree
        .leaf
        .as_ref()
        .map(|leaf| leaf.next.is_some())
        .unwrap_or(false);
    let first_data = tree.leaf.as_ref().map(|leaf| leaf.data).unwrap_or(0);
    if first_data != node.index || has_chain {
        if x == 0.0 {
            x = rng.jiggle();
            l += x * x;
        }
        if y == 0.0 {
            y = rng.jiggle();
            l += y * y;
        }
        if z == 0.0 {
            z = rng.jiggle();
            l += z * z;
        }
        if l < DISTANCE_MIN2 {
            l = (DISTANCE_MIN2 * l).sqrt();
        }
    }
    let mut head = tree.leaf.as_ref();
    while let Some(leaf) = head {
        if leaf.data != node.index {
            let strength = strengths.get(leaf.data).copied().unwrap_or(0.0);
            let factor = strength * alpha / l;
            node.vx += x * factor;
            node.vy += y * factor;
            node.vz += z * factor;
        }
        head = leaf.next.as_deref();
    }
    false
}

#[allow(clippy::too_many_arguments)]
fn collide_rec(
    tree: &OctNode,
    bounds: Bounds,
    state: &mut [CollideNode],
    node_index: usize,
    xi: f64,
    yi: f64,
    zi: f64,
    radii: &[f64],
    rng: &mut Lcg,
) -> bool {
    let ri = radii.get(node_index).copied().unwrap_or(0.0);
    let ri2 = ri * ri;
    let r = ri + tree.r;
    if let Some(leaf) = tree.leaf.as_ref() {
        let mut head = Some(leaf);
        while let Some(entry) = head {
            let data_index = entry.data;
            if data_index > node_index {
                let other = state.get(data_index).copied().unwrap_or_default();
                let mut x = xi - other.x - other.vx;
                let mut y = yi - other.y - other.vy;
                let mut z = zi - other.z - other.vz;
                let mut l = x * x + y * y + z * z;
                if l < r * r {
                    if x == 0.0 {
                        x = rng.jiggle();
                        l += x * x;
                    }
                    if y == 0.0 {
                        y = rng.jiggle();
                        l += y * y;
                    }
                    if z == 0.0 {
                        z = rng.jiggle();
                        l += z * z;
                    }
                    let mut l = l.sqrt();
                    l = (r - l) / l;
                    x *= l;
                    y *= l;
                    z *= l;
                    let rj = tree.r;
                    let rj2 = rj * rj;
                    let ratio = rj2 / (ri2 + rj2);
                    let (left, right) = state.split_at_mut(data_index);
                    let node = &mut left[node_index];
                    let data = &mut right[0];
                    node.vx += x * ratio;
                    node.vy += y * ratio;
                    node.vz += z * ratio;
                    let other_ratio = 1.0 - ratio;
                    data.vx -= x * other_ratio;
                    data.vy -= y * other_ratio;
                    data.vz -= z * other_ratio;
                }
            }
            head = entry.next.as_deref();
        }
        return false;
    }
    if bounds.x0 > xi + r
        || bounds.x1 < xi - r
        || bounds.y0 > yi + r
        || bounds.y1 < yi - r
        || bounds.z0 > zi + r
        || bounds.z1 < zi - r
    {
        return true;
    }
    if let Some(children) = tree.children.as_ref() {
        for (index, child) in children.iter().enumerate() {
            if let Some(child) = child {
                collide_rec(
                    child,
                    bounds.child(index),
                    state,
                    node_index,
                    xi,
                    yi,
                    zi,
                    radii,
                    rng,
                );
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collide_separates_overlapping_nodes() {
        let points = [(0.0, 0.0, 0.0), (1.0, 0.0, 0.0)];
        let mut tree = Octree::build(&points);
        tree.prepare_radii(&[4.0, 4.0]);
        let mut state = vec![
            CollideNode {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                ..Default::default()
            },
            CollideNode {
                x: 1.0,
                y: 0.0,
                z: 0.0,
                ..Default::default()
            },
        ];
        let mut rng = Lcg::new(1);
        let xi = state[0].x + state[0].vx;
        let yi = state[0].y + state[0].vy;
        let zi = state[0].z + state[0].vz;
        tree.apply_collide(&mut state, 0, xi, yi, zi, &[4.0, 4.0], &mut rng);
        assert!(state[0].vx < 0.0);
        assert!(state[1].vx > 0.0);
    }
}
