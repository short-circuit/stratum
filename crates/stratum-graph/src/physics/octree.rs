//! Deterministic `d3-octree` port used by the many-body and collide forces.
//!
//! The tree construction, extent expansion, coincident-leaf chaining and
//! post-order accumulation mirror the JavaScript sources exactly: children use
//! the same octant numbering, coincident points form a newest-first chain, and
//! the accumulated strength uses the same `sqrt(4 / 8)` scaling as d3. The force
//! kernels themselves live in [`super::barnes_hut`].

/// An axis-aligned octree cell.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Bounds {
    pub x0: f64,
    pub y0: f64,
    pub z0: f64,
    pub x1: f64,
    pub y1: f64,
    pub z1: f64,
}

impl Bounds {
    pub(crate) fn child(self, index: usize) -> Self {
        let xm = (self.x0 + self.x1) / 2.0;
        let ym = (self.y0 + self.y1) / 2.0;
        let zm = (self.z0 + self.z1) / 2.0;
        let right = index & 1 != 0;
        let bottom = index & 2 != 0;
        let deep = index & 4 != 0;
        Self {
            x0: if right { xm } else { self.x0 },
            y0: if bottom { ym } else { self.y0 },
            z0: if deep { zm } else { self.z0 },
            x1: if right { self.x1 } else { xm },
            y1: if bottom { self.y1 } else { ym },
            z1: if deep { self.z1 } else { zm },
        }
    }

    pub(crate) fn octant(self, x: f64, y: f64, z: f64) -> usize {
        let xm = (self.x0 + self.x1) / 2.0;
        let ym = (self.y0 + self.y1) / 2.0;
        let zm = (self.z0 + self.z1) / 2.0;
        (usize::from(z >= zm) << 2) | (usize::from(y >= ym) << 1) | usize::from(x >= xm)
    }
}

/// A coincident-leaf chain. The newest point is the head, matching d3.
#[derive(Debug)]
pub(crate) struct OctLeaf {
    pub data: usize,
    pub next: Option<Box<OctLeaf>>,
}

/// An octree node: either a leaf chain or eight child cells.
#[derive(Debug)]
pub(crate) struct OctNode {
    pub children: Option<[Option<Box<OctNode>>; 8]>,
    pub leaf: Option<OctLeaf>,
    pub value: f64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub r: f64,
}

impl OctNode {
    pub(crate) fn internal() -> Self {
        Self {
            children: Some(std::array::from_fn(|_| None)),
            leaf: None,
            value: 0.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
            r: 0.0,
        }
    }

    pub(crate) fn leaf(data: usize) -> Self {
        Self {
            children: None,
            leaf: Some(OctLeaf { data, next: None }),
            value: 0.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
            r: 0.0,
        }
    }

    pub(crate) fn is_internal(&self) -> bool {
        self.children.is_some()
    }
}

/// A deterministic octree over point data.
pub(crate) struct Octree {
    pub root: Option<Box<OctNode>>,
    pub bounds: Bounds,
}

impl Octree {
    pub(crate) fn empty() -> Self {
        Self {
            root: None,
            bounds: Bounds {
                x0: f64::NAN,
                y0: f64::NAN,
                z0: f64::NAN,
                x1: f64::NAN,
                y1: f64::NAN,
                z1: f64::NAN,
            },
        }
    }

    /// Build a tree over `points`, mirroring `d3-octree`'s `addAll`.
    pub(crate) fn build(points: &[(f64, f64, f64)]) -> Self {
        let mut tree = Self::empty();
        let mut min = (f64::INFINITY, f64::INFINITY, f64::INFINITY);
        let mut max = (f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
        for &(x, y, z) in points {
            if !x.is_finite() || !y.is_finite() || !z.is_finite() {
                continue;
            }
            min = (min.0.min(x), min.1.min(y), min.2.min(z));
            max = (max.0.max(x), max.1.max(y), max.2.max(z));
        }
        if min.0 > max.0 {
            return tree;
        }
        tree.cover(min.0, min.1, min.2);
        tree.cover(max.0, max.1, max.2);
        for (index, &(x, y, z)) in points.iter().enumerate() {
            tree.add(x, y, z, index, points);
        }
        tree
    }

    /// Expand the tree extent so it covers `(x, y, z)`.
    fn cover(&mut self, x: f64, y: f64, z: f64) {
        if !x.is_finite() || !y.is_finite() || !z.is_finite() {
            return;
        }
        if self.bounds.x0.is_nan() {
            self.bounds.x0 = x.floor();
            self.bounds.y0 = y.floor();
            self.bounds.z0 = z.floor();
            self.bounds.x1 = self.bounds.x0 + 1.0;
            self.bounds.y1 = self.bounds.y0 + 1.0;
            self.bounds.z1 = self.bounds.z0 + 1.0;
            return;
        }
        let mut t = self.bounds.x1 - self.bounds.x0;
        if t == 0.0 {
            t = 1.0;
        }
        // `d3-octree`'s `cover` only rewraps the root when the existing root is an
        // internal node (`if (this._root && this._root.length)`). When the tree is
        // empty or the root is a leaf it expands the extent but leaves the root
        // untouched. Mirror that here, otherwise the empty-tree cover performed by
        // `addAll` would install a deep chain of childless internal nodes.
        let root_is_internal = self
            .root
            .as_ref()
            .map(|node| node.is_internal())
            .unwrap_or(false);
        let mut node = if root_is_internal {
            self.root.take()
        } else {
            None
        };
        while self.bounds.x0 > x
            || x >= self.bounds.x1
            || self.bounds.y0 > y
            || y >= self.bounds.y1
            || self.bounds.z0 > z
            || z >= self.bounds.z1
        {
            let index = (usize::from(z < self.bounds.z0) << 2)
                | (usize::from(y < self.bounds.y0) << 1)
                | usize::from(x < self.bounds.x0);
            if root_is_internal {
                let mut parent = Box::new(OctNode::internal());
                if let Some(children) = parent.children.as_mut() {
                    children[index] = node.take();
                }
                node = Some(parent);
            }
            t *= 2.0;
            match index {
                0 => {
                    self.bounds.x1 = self.bounds.x0 + t;
                    self.bounds.y1 = self.bounds.y0 + t;
                    self.bounds.z1 = self.bounds.z0 + t;
                }
                1 => {
                    self.bounds.x0 = self.bounds.x1 - t;
                    self.bounds.y1 = self.bounds.y0 + t;
                    self.bounds.z1 = self.bounds.z0 + t;
                }
                2 => {
                    self.bounds.x1 = self.bounds.x0 + t;
                    self.bounds.y0 = self.bounds.y1 - t;
                    self.bounds.z1 = self.bounds.z0 + t;
                }
                3 => {
                    self.bounds.x0 = self.bounds.x1 - t;
                    self.bounds.y0 = self.bounds.y1 - t;
                    self.bounds.z1 = self.bounds.z0 + t;
                }
                4 => {
                    self.bounds.x1 = self.bounds.x0 + t;
                    self.bounds.y1 = self.bounds.y0 + t;
                    self.bounds.z0 = self.bounds.z1 - t;
                }
                5 => {
                    self.bounds.x0 = self.bounds.x1 - t;
                    self.bounds.y1 = self.bounds.y0 + t;
                    self.bounds.z0 = self.bounds.z1 - t;
                }
                6 => {
                    self.bounds.x1 = self.bounds.x0 + t;
                    self.bounds.y0 = self.bounds.y1 - t;
                    self.bounds.z0 = self.bounds.z1 - t;
                }
                _ => {
                    self.bounds.x0 = self.bounds.x1 - t;
                    self.bounds.y0 = self.bounds.y1 - t;
                    self.bounds.z0 = self.bounds.z1 - t;
                }
            }
        }
        if root_is_internal {
            self.root = node;
        }
    }

    /// Insert one point at the exact octant d3 would use.
    fn add(&mut self, x: f64, y: f64, z: f64, data: usize, points: &[(f64, f64, f64)]) {
        if !x.is_finite() || !y.is_finite() || !z.is_finite() {
            return;
        }
        let root = self.root.take();
        self.root = add_rec(root, self.bounds, x, y, z, data, points);
    }

    /// Accumulate many-body strengths and centres of mass.
    pub(crate) fn accumulate(&mut self, strengths: &[f64], points: &[(f64, f64, f64)]) {
        if let Some(root) = self.root.as_mut() {
            accumulate_node(root, strengths, points);
        }
    }

    /// Prepare collide radii bottom-up.
    pub(crate) fn prepare_radii(&mut self, radii: &[f64]) {
        if let Some(root) = self.root.as_mut() {
            prepare_r(root, radii);
        }
    }
}

fn add_rec(
    node: Option<Box<OctNode>>,
    bounds: Bounds,
    x: f64,
    y: f64,
    z: f64,
    data: usize,
    points: &[(f64, f64, f64)],
) -> Option<Box<OctNode>> {
    let Some(mut node) = node else {
        return Some(Box::new(OctNode::leaf(data)));
    };
    if node.is_internal() {
        let index = bounds.octant(x, y, z);
        let child = node
            .children
            .as_mut()
            .and_then(|children| children[index].take());
        let replacement = add_rec(child, bounds.child(index), x, y, z, data, points);
        if let Some(children) = node.children.as_mut() {
            children[index] = replacement;
        }
        return Some(node);
    }
    let old_data = node.leaf.as_ref().map(|leaf| leaf.data).unwrap_or(0);
    let old = points.get(old_data).copied().unwrap_or((x, y, z));
    if x == old.0 && y == old.1 && z == old.2 {
        let mut leaf = OctNode::leaf(data);
        if let Some(chain) = leaf.leaf.as_mut() {
            chain.next = Some(Box::new(OctLeaf {
                data: old_data,
                next: take_chain(&mut node),
            }));
        }
        return Some(Box::new(leaf));
    }
    Some(Box::new(split_leaf(
        node, bounds, x, y, z, data, old, points,
    )))
}

/// Move the existing leaf chain out of `node`, leaving an empty leaf shell.
fn take_chain(node: &mut Box<OctNode>) -> Option<Box<OctLeaf>> {
    node.leaf.take().and_then(|mut leaf| leaf.next.take())
}

#[allow(clippy::too_many_arguments, clippy::only_used_in_recursion)]
fn split_leaf(
    old: Box<OctNode>,
    bounds: Bounds,
    x: f64,
    y: f64,
    z: f64,
    data: usize,
    old_coords: (f64, f64, f64),
    points: &[(f64, f64, f64)],
) -> OctNode {
    let new_octant = bounds.octant(x, y, z);
    let old_octant = bounds.octant(old_coords.0, old_coords.1, old_coords.2);
    let mut internal = OctNode::internal();
    if let Some(children) = internal.children.as_mut() {
        if new_octant == old_octant {
            children[new_octant] = Some(Box::new(split_leaf(
                old,
                bounds.child(new_octant),
                x,
                y,
                z,
                data,
                old_coords,
                points,
            )));
        } else {
            children[new_octant] = Some(Box::new(OctNode::leaf(data)));
            children[old_octant] = Some(old);
        }
    }
    internal
}

fn accumulate_node(node: &mut OctNode, strengths: &[f64], points: &[(f64, f64, f64)]) {
    if let Some(children) = node.children.as_mut() {
        let mut strength = 0.0;
        let mut weight = 0.0;
        let mut x = 0.0;
        let mut y = 0.0;
        let mut z = 0.0;
        for child in children.iter_mut().flatten() {
            accumulate_node(child, strengths, points);
            let value = child.value;
            let abs = value.abs();
            if abs != 0.0 {
                strength += value;
                weight += abs;
                x += abs * child.x;
                y += abs * child.y;
                z += abs * child.z;
            }
        }
        strength *= (4.0_f64 / 8.0).sqrt();
        if weight != 0.0 {
            node.x = x / weight;
            node.y = y / weight;
            node.z = z / weight;
        }
        node.value = strength;
        return;
    }
    let mut strength = 0.0;
    let mut head = node.leaf.as_ref();
    if let Some(first) = head {
        node.x = points.get(first.data).copied().unwrap_or((0.0, 0.0, 0.0)).0;
        node.y = points.get(first.data).copied().unwrap_or((0.0, 0.0, 0.0)).1;
        node.z = points.get(first.data).copied().unwrap_or((0.0, 0.0, 0.0)).2;
    }
    while let Some(leaf) = head {
        strength += strengths.get(leaf.data).copied().unwrap_or(0.0);
        head = leaf.next.as_deref();
    }
    node.value = strength;
}

fn prepare_r(node: &mut OctNode, radii: &[f64]) {
    if let Some(children) = node.children.as_mut() {
        let mut max = 0.0_f64;
        for child in children.iter_mut().flatten() {
            prepare_r(child, radii);
            max = max.max(child.r);
        }
        node.r = max;
        return;
    }
    node.r = node
        .leaf
        .as_ref()
        .and_then(|leaf| radii.get(leaf.data).copied())
        .unwrap_or(0.0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn covering_an_empty_tree_leaves_the_root_empty() {
        // `addAll` covers the extent of the point set before inserting anything.
        // On an empty tree `d3-octree` expands the extent without rewrapping the
        // (absent) root, so the root must stay empty and the extent must grow.
        let mut tree = Octree::empty();
        tree.cover(0.0, 0.0, 0.0);
        tree.cover(100.0, 100.0, 100.0);
        assert!(tree.root.is_none());
        assert_eq!(tree.bounds.x0, 0.0);
        assert_eq!(tree.bounds.x1, 128.0);
    }

    #[test]
    fn distant_points_split_into_direct_leaf_children() {
        let points = [
            (1.0, 41.0, -21.0),
            (-17.0, -5.0, -43.0),
            (22.0, 8.0, -42.0),
            (13.0, 45.0, 7.0),
        ];
        let tree = Octree::build(&points);
        let root = tree.root.as_ref().expect("root");
        let children = root.children.as_ref().expect("internal");
        let occupied: Vec<usize> = children
            .iter()
            .enumerate()
            .filter_map(|(index, child)| child.as_ref().map(|_| index))
            .collect();
        assert_eq!(occupied, vec![0, 1, 2, 6]);
        for child in children.iter().flatten() {
            assert!(!child.is_internal(), "each point must be a direct leaf");
        }
    }

    #[test]
    fn tree_covers_all_points() {
        let points = [(1.0, 2.0, 3.0), (50.0, -20.0, 7.0), (-30.0, 4.0, 60.0)];
        let tree = Octree::build(&points);
        assert!(tree.root.is_some());
        assert!(tree.bounds.x0 <= -30.0 && tree.bounds.x1 > 50.0);
        assert!(tree.bounds.y0 <= -20.0 && tree.bounds.y1 > 4.0);
        assert!(tree.bounds.z0 <= 3.0 && tree.bounds.z1 > 60.0);
    }

    #[test]
    fn coincident_points_form_a_chain() {
        let points = [(1.0, 1.0, 1.0), (1.0, 1.0, 1.0)];
        let mut tree = Octree::build(&points);
        tree.accumulate(&[-30.0, -30.0], &points);
        let root = tree.root.as_ref().expect("root");
        let leaf = root.leaf.as_ref().expect("leaf");
        assert!(leaf.next.is_some());
        assert_eq!(root.value, -60.0);
    }
}
