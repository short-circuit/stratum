use crate::block::{Block, BlockId};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
#[allow(unused_imports)]
use uuid::Uuid;

/// A tree of blocks using `parent_id` + `left_id` for O(1) sibling ordering.
///
/// This is Logseq's model:
/// - `parent_id` defines hierarchy (None = root block)
/// - `left_id` defines sibling ordering (None = first child in parent order)
///
/// Moving a subtree only requires updating the moved node's `parent_id` and `left_id`,
/// plus the `left_id` of the node that was after it (to close the gap).
///
/// # Determinism
///
/// Document order never depends on `HashMap` iteration order. Blocks are tracked
/// in the order they were first inserted, and that insertion order is the
/// documented tie-break/fallback for sibling ordering. An explicit `left_id`
/// chain still wins; insertion order only decides the head when several siblings
/// have no `left_id` and fills in siblings when the chain is incomplete.
/// Consequently `roots()`, `children()`, `depth_first_order()`,
/// `into_sorted_vec()`, `find_by_marker()` and `find_by_property()` are stable
/// for a given insertion sequence, which keeps `serialize_blocks` output
/// byte-stable.
///
/// Payloads deserialized from an older `BlockTree` (without a recorded insertion
/// order) fall back to ascending block-id order, so they remain deterministic
/// too.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct BlockTree {
    blocks: HashMap<BlockId, Block>,
    root_order: Vec<BlockId>, // cached root-level ordering
    dirty: bool,
    /// Block ids in first-insertion order; the deterministic sibling tie-break.
    /// `serde(default)` keeps payloads serialized before this field existed
    /// deserializable.
    #[serde(default)]
    insertion_order: Vec<BlockId>,
}

impl BlockTree {
    pub fn new() -> Self {
        Self {
            blocks: HashMap::new(),
            root_order: Vec::new(),
            dirty: false,
            insertion_order: Vec::new(),
        }
    }

    pub fn insert(&mut self, block: Block) {
        let id = block.id;
        // A replacement keeps its original position; only a genuinely new id is
        // appended to the insertion order.
        if !self.blocks.contains_key(&id) {
            self.insertion_order.push(id);
        }
        self.blocks.insert(id, block);
        self.dirty = true;
    }

    pub fn remove(&mut self, id: BlockId) -> Option<Block> {
        let removed = self.blocks.remove(&id);
        self.insertion_order.retain(|existing| *existing != id);
        self.dirty = true;
        removed
    }

    pub fn get(&self, id: BlockId) -> Option<&Block> {
        self.blocks.get(&id)
    }

    pub fn get_mut(&mut self, id: BlockId) -> Option<&mut Block> {
        self.blocks.get_mut(&id)
    }

    pub fn contains(&self, id: BlockId) -> bool {
        self.blocks.contains_key(&id)
    }

    pub fn len(&self) -> usize {
        self.blocks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    pub fn all_blocks(&self) -> impl Iterator<Item = &Block> {
        self.blocks.values()
    }

    pub fn iter_ids(&self) -> impl Iterator<Item = &BlockId> {
        self.blocks.keys()
    }

    /// Candidate blocks in insertion order, skipping ids that are no longer
    /// present.
    ///
    /// Every ordering-sensitive decision goes through this helper instead of
    /// `self.blocks.values()` so results do not depend on `HashMap` iteration
    /// order. Legacy payloads without a recorded insertion order append the
    /// remaining blocks sorted by id, which keeps them deterministic as well.
    fn ordered_candidates(&self) -> Vec<&Block> {
        let mut candidates: Vec<&Block> = self
            .insertion_order
            .iter()
            .filter_map(|id| self.blocks.get(id))
            .collect();

        if candidates.len() < self.blocks.len() {
            let known: HashSet<BlockId> = self.insertion_order.iter().copied().collect();
            let mut rest: Vec<&Block> = self
                .blocks
                .values()
                .filter(|b| !known.contains(&b.id))
                .collect();
            rest.sort_by_key(|b| b.id);
            candidates.extend(rest);
        }

        candidates
    }

    /// Insertion positions for every block currently in the tree.
    ///
    /// This is the single ordering source shared by `collect_siblings()`,
    /// `depth_first_order()` and `into_sorted_vec()`. Ids that were inserted are
    /// ranked by their first-insertion position; ids that predate the
    /// `insertion_order` field (legacy payloads) are ranked after every recorded
    /// id, in ascending id order. Building the index once per call keeps the
    /// result deterministic without allocating a candidate vector per sibling
    /// lookup.
    fn insertion_index(&self) -> HashMap<BlockId, usize> {
        let mut index: HashMap<BlockId, usize> = HashMap::with_capacity(self.blocks.len());
        for (position, id) in self.insertion_order.iter().enumerate() {
            if self.blocks.contains_key(id) {
                index.insert(*id, position);
            }
        }

        // Legacy payloads have no recorded insertion order. Ranking their ids
        // after every recorded position keeps them deterministic and never
        // ahead of an explicitly inserted block.
        let mut legacy: Vec<BlockId> = self
            .blocks
            .keys()
            .filter(|id| !index.contains_key(*id))
            .copied()
            .collect();
        legacy.sort_unstable();
        let base = self.insertion_order.len();
        for (offset, id) in legacy.into_iter().enumerate() {
            index.insert(id, base + offset);
        }

        index
    }

    /// Group candidate blocks by parent id (`None` = roots) in one pass.
    ///
    /// Replaces the previous per-block `children()` scan, which was O(n²) on a
    /// page with many roots.
    fn parent_groups<'a>(&'a self) -> HashMap<Option<BlockId>, Vec<&'a Block>> {
        let mut groups: HashMap<Option<BlockId>, Vec<&'a Block>> = HashMap::new();
        for block in self.ordered_candidates() {
            groups.entry(block.parent_id).or_default().push(block);
        }
        groups
    }

    /// Emit the ids of one depth-first traversal over pre-built parent groups.
    ///
    /// Sibling order comes from `order_group()` for each group, so the total
    /// cost is O(n + Σ k log k). The visited set guards against parent cycles,
    /// which would otherwise recurse forever.
    fn depth_first_ids(
        &self,
        index: &HashMap<BlockId, usize>,
        groups: &HashMap<Option<BlockId>, Vec<&Block>>,
    ) -> Vec<BlockId> {
        let mut result = Vec::with_capacity(self.blocks.len());
        let mut visited: HashSet<BlockId> = HashSet::new();
        let mut stack: Vec<BlockId> = Vec::new();

        if let Some(roots) = groups.get(&None) {
            for root in order_group(roots, index).into_iter().rev() {
                stack.push(root.id);
            }
        }

        while let Some(id) = stack.pop() {
            if !visited.insert(id) {
                continue; // cycle guard: a parent loop is visited once
            }
            if !self.blocks.contains_key(&id) {
                continue;
            }
            result.push(id);
            if let Some(children) = groups.get(&Some(id)) {
                for child in order_group(children, index).into_iter().rev() {
                    stack.push(child.id);
                }
            }
        }

        result
    }

    /// Get root blocks in display order (via left_id linked list).
    pub fn roots(&self) -> Vec<&Block> {
        self.collect_siblings(None)
    }

    /// Get children of a block in display order.
    pub fn children(&self, parent_id: BlockId) -> Vec<&Block> {
        self.collect_siblings(Some(parent_id))
    }

    /// Get the first child of a block.
    pub fn first_child(&self, parent_id: BlockId) -> Option<&Block> {
        self.ordered_candidates()
            .into_iter()
            .find(|b| b.parent_id == Some(parent_id) && b.left_id.is_none())
    }

    /// Get the last child of a block.
    pub fn last_child(&self, parent_id: BlockId) -> Option<&Block> {
        let children: Vec<&Block> = self.children(parent_id);
        children.into_iter().last()
    }

    /// Get the parent block.
    pub fn parent(&self, child_id: BlockId) -> Option<&Block> {
        self.get(child_id)
            .and_then(|b| b.parent_id)
            .and_then(|pid| self.get(pid))
    }

    /// Get the next sibling (block to the right).
    pub fn next_sibling(&self, block_id: BlockId) -> Option<&Block> {
        self.ordered_candidates()
            .into_iter()
            .find(|b| b.left_id == Some(block_id))
    }

    /// Get the previous sibling (block to the left).
    pub fn prev_sibling(&self, block_id: BlockId) -> Option<BlockId> {
        self.get(block_id).and_then(|b| b.left_id)
    }

    /// Get all descendants of a block (including itself).
    pub fn subtree(&self, root_id: BlockId) -> Vec<&Block> {
        let mut result = Vec::new();
        let mut queue = vec![root_id];
        while let Some(id) = queue.pop() {
            if let Some(block) = self.get(id) {
                result.push(block);
                for child in self.children(id) {
                    queue.push(child.id);
                }
            }
        }
        result
    }

    /// Get all ancestors of a block (parent chain, including self).
    pub fn ancestors(&self, block_id: BlockId) -> Vec<&Block> {
        let mut result = Vec::new();
        let mut current = block_id;
        while let Some(block) = self.get(current) {
            result.push(block);
            match block.parent_id {
                Some(pid) => current = pid,
                None => break,
            }
        }
        result.reverse();
        result
    }

    /// Get the depth of a block (0 for roots).
    pub fn depth(&self, block_id: BlockId) -> usize {
        let mut depth = 0;
        let mut current = block_id;
        while let Some(block) = self.get(current) {
            match block.parent_id {
                Some(pid) => {
                    depth += 1;
                    current = pid;
                }
                None => break,
            }
        }
        depth
    }

    /// Get all blocks in depth-first traversal order.
    ///
    /// One pass groups every block by parent, then a DFS emits the groups in
    /// `left_id`/insertion order. Cost is O(n + Σ k log k) instead of the
    /// previous O(n²) per-block sibling scan.
    pub fn depth_first_order(&self) -> Vec<&Block> {
        let index = self.insertion_index();
        let groups = self.parent_groups();
        self.depth_first_ids(&index, &groups)
            .into_iter()
            .filter_map(|id| self.blocks.get(&id))
            .collect()
    }

    /// Check if any block in the tree has a given task marker.
    pub fn find_by_marker(&self, marker: crate::block::TaskMarker) -> Vec<&Block> {
        self.ordered_candidates()
            .into_iter()
            .filter(|b| b.marker == Some(marker))
            .collect()
    }

    /// Check if any block has a given property set.
    pub fn find_by_property(&self, key: &str, value: &str) -> Vec<&Block> {
        self.ordered_candidates()
            .into_iter()
            .filter(|b| b.properties.get(key).map(|v| v.as_str()) == Some(value))
            .collect()
    }

    /// Build a set of all block IDs in the tree.
    pub fn id_set(&self) -> HashSet<BlockId> {
        self.blocks.keys().copied().collect()
    }

    /// Collect siblings in order for a given parent (None = roots).
    fn collect_siblings(&self, parent_id: Option<BlockId>) -> Vec<&Block> {
        let index = self.insertion_index();
        let group: Vec<&Block> = self
            .ordered_candidates()
            .into_iter()
            .filter(|b| b.parent_id == parent_id)
            .collect();
        order_group(&group, &index)
    }

    /// Rebuild root ordering cache (call after structural changes).
    pub fn rebuild_cache(&mut self) {
        self.root_order = self.roots().iter().map(|b| b.id).collect();
        self.dirty = false;
    }

    /// Convert the tree into a sorted vec of blocks (for iteration in order).
    ///
    /// Uses the same single-pass grouping and DFS as `depth_first_order()`, so
    /// no per-node `values()` scan is performed.
    pub fn into_sorted_vec(self) -> Vec<Block> {
        let index = self.insertion_index();
        let groups = self.parent_groups();
        self.depth_first_ids(&index, &groups)
            .into_iter()
            .filter_map(|id| self.blocks.get(&id).cloned())
            .collect()
    }
}

/// Return one sibling group in document order.
///
/// `blocks` must already be restricted to a single parent group. The group is
/// sorted deterministically first: by insertion position (`index`), and for ids
/// absent from `index` (legacy payloads) by ascending block id after the indexed
/// ones. The first `left_id == None` entry is the head; the chain is then walked
/// through a `HashMap` (O(1) per step) with a visited set for cycle protection.
/// Blocks that the chain did not reach are appended in the sorted order so a
/// broken or cyclic chain still yields every sibling exactly once.
fn order_group<'a>(blocks: &[&'a Block], index: &HashMap<BlockId, usize>) -> Vec<&'a Block> {
    let mut sorted: Vec<&'a Block> = blocks.to_vec();
    sorted.sort_by(|a, b| match (index.get(&a.id), index.get(&b.id)) {
        (Some(a_position), Some(b_position)) => a_position.cmp(b_position),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => a.id.cmp(&b.id),
    });

    // `left_id` points at the *previous* sibling, so walking forward needs a
    // successor map keyed by `left_id`. Keeping the first entry in sorted order
    // mirrors the previous `iter().find()` behaviour if corrupt data repeats a
    // pointer.
    let mut successors: HashMap<BlockId, &'a Block> = HashMap::with_capacity(sorted.len());
    for block in &sorted {
        if let Some(left) = block.left_id {
            successors.entry(left).or_insert(*block);
        }
    }

    let mut result: Vec<&'a Block> = Vec::with_capacity(sorted.len());
    let mut seen: HashSet<BlockId> = HashSet::new();

    let mut cursor = sorted.iter().find(|b| b.left_id.is_none()).copied();
    while let Some(block) = cursor {
        if !seen.insert(block.id) {
            break; // cycle detection
        }
        result.push(block);
        cursor = successors.get(&block.id).copied();
    }

    for block in sorted {
        if !seen.contains(&block.id) {
            seen.insert(block.id);
            result.push(block);
        }
    }

    result
}

#[cfg(test)]
mod tests;
