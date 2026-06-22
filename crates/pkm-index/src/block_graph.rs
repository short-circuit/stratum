//! Block-level graph — tracks references between blocks.

use pkm_block::BlockId;
use std::collections::{HashMap, HashSet};

/// A directed edge from one block to another or to a page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockEdge {
    BlockRef { source: BlockId, target: BlockId },
    PageRef { source: BlockId, target_page: String },
    Embed { source: BlockId, target: BlockOrPage },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BlockOrPage {
    Block(BlockId),
    Page(String),
}

/// Tracks references between blocks for backlink resolution.
#[derive(Debug, Clone, Default)]
pub struct BlockGraph {
    /// Edges by source block ID.
    outgoing: HashMap<BlockId, Vec<BlockEdge>>,
    /// Reverse edges: block ID → blocks that reference it.
    incoming_block: HashMap<BlockId, Vec<BlockId>>,
    /// Reverse edges: page name → blocks that reference it.
    incoming_page: HashMap<String, Vec<BlockId>>,
    /// Block metadata: (page_path, content_preview).
    block_info: HashMap<BlockId, (String, String)>,
}

impl BlockGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a block with metadata.
    pub fn register_block(&mut self, id: BlockId, page_path: &str, content: &str) {
        self.block_info.insert(id, (page_path.to_string(), content.to_string()));
    }

    /// Remove a block from the graph.
    pub fn remove_block(&mut self, id: BlockId) {
        self.outgoing.remove(&id);
        self.block_info.remove(&id);
        self.incoming_block.remove(&id);
        for refs in self.incoming_block.values_mut() {
            refs.retain(|bid| *bid != id);
        }
        for refs in self.incoming_page.values_mut() {
            refs.retain(|bid| *bid != id);
        }
    }

    /// Add a block-to-block reference.
    pub fn add_block_ref(&mut self, source: BlockId, target: BlockId) {
        let edge = BlockEdge::BlockRef { source, target };
        self.outgoing.entry(source).or_default().push(edge);
        self.incoming_block.entry(target).or_default().push(source);
    }

    /// Add a block-to-page reference.
    pub fn add_page_ref(&mut self, source: BlockId, target_page: &str) {
        let edge = BlockEdge::PageRef {
            source,
            target_page: target_page.to_string(),
        };
        self.outgoing.entry(source).or_default().push(edge);
        self.incoming_page
            .entry(target_page.to_string())
            .or_default()
            .push(source);
    }

    /// Add an embed reference.
    pub fn add_embed(&mut self, source: BlockId, target: BlockOrPage) {
        let edge = BlockEdge::Embed {
            source,
            target: target.clone(),
        };
        match &target {
            BlockOrPage::Block(bid) => {
                self.incoming_block.entry(*bid).or_default().push(source);
            }
            BlockOrPage::Page(name) => {
                self.incoming_page.entry(name.clone()).or_default().push(source);
            }
        }
        self.outgoing.entry(source).or_default().push(edge);
    }

    /// Get backlinks to a specific block.
    pub fn get_block_backlinks(&self, target: BlockId) -> Vec<BlockBacklink> {
        self.incoming_block
            .get(&target)
            .map(|sources| {
                sources
                    .iter()
                    .filter_map(|src_id| {
                        self.block_info.get(src_id).map(|(page, content)| BlockBacklink {
                            source_id: *src_id,
                            source_page: page.clone(),
                            context: content.clone(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get backlinks to a page.
    pub fn get_page_backlinks(&self, page_name: &str) -> Vec<BlockBacklink> {
        self.incoming_page
            .get(page_name)
            .map(|sources| {
                sources
                    .iter()
                    .filter_map(|src_id| {
                        self.block_info.get(src_id).map(|(page, content)| BlockBacklink {
                            source_id: *src_id,
                            source_page: page.clone(),
                            context: content.clone(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Find blocks that mention text matching a page name but don't explicitly link.
    pub fn find_unlinked_mentions(&self, page_name: &str) -> Vec<BlockBacklink> {
        let lower_name = page_name.to_lowercase();
        // Try exact case first, then lowercase for lookup
        let linked: HashSet<BlockId> = {
            let mut ids: HashSet<BlockId> = self
                .get_page_backlinks(page_name)
                .iter()
                .map(|bl| bl.source_id)
                .collect();
            // Also check lowercase version of keys
            for (key, sources) in &self.incoming_page {
                if key.to_lowercase() == lower_name {
                    ids.extend(sources.iter().copied());
                }
            }
            ids
        };

        self.block_info
            .iter()
            .filter(|(id, (_page, content))| {
                !linked.contains(id) && content.to_lowercase().contains(&lower_name)
            })
            .map(|(id, (page, content))| BlockBacklink {
                source_id: *id,
                source_page: page.clone(),
                context: content.clone(),
            })
            .collect()
    }

    /// Check if a block has outgoing references.
    pub fn has_outgoing(&self, block_id: BlockId) -> bool {
        self.outgoing.contains_key(&block_id)
    }

    /// Get all blocks that have outgoing references.
    pub fn referenced_blocks(&self) -> HashSet<BlockId> {
        self.incoming_block.keys().copied().collect()
    }

    /// Get all blocks that have incoming references.
    pub fn referencing_blocks(&self) -> HashSet<BlockId> {
        self.outgoing.keys().copied().collect()
    }

    /// Total number of edges.
    pub fn edge_count(&self) -> usize {
        self.outgoing.values().map(|v| v.len()).sum()
    }

    /// Total number of blocks tracked.
    pub fn block_count(&self) -> usize {
        self.block_info.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockBacklink {
    pub source_id: BlockId,
    pub source_page: String,
    pub context: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_add_block_ref() {
        let mut graph = BlockGraph::new();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        graph.register_block(a, "pages/a.md", "block A content");
        graph.register_block(b, "pages/b.md", "block B content");
        graph.add_block_ref(a, b);

        let backlinks = graph.get_block_backlinks(b);
        assert_eq!(backlinks.len(), 1);
        assert_eq!(backlinks[0].source_id, a);
        assert_eq!(backlinks[0].source_page, "pages/a.md");
    }

    #[test]
    fn test_add_page_ref() {
        let mut graph = BlockGraph::new();
        let a = Uuid::new_v4();
        graph.register_block(a, "pages/a.md", "see [[Target Page]]");
        graph.add_page_ref(a, "Target Page");

        let backlinks = graph.get_page_backlinks("Target Page");
        assert_eq!(backlinks.len(), 1);
        assert_eq!(backlinks[0].source_id, a);
    }

    #[test]
    fn test_remove_block() {
        let mut graph = BlockGraph::new();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        graph.register_block(a, "pages/a.md", "A");
        graph.register_block(b, "pages/b.md", "B");
        graph.add_block_ref(a, b);

        graph.remove_block(a);

        assert!(graph.get_block_backlinks(b).is_empty());
        assert_eq!(graph.block_count(), 1);
    }

    #[test]
    fn test_unlinked_mentions() {
        let mut graph = BlockGraph::new();
        let a = Uuid::new_v4();
        graph.register_block(a, "pages/a.md", "I mention target page here");

        let mentions = graph.find_unlinked_mentions("target page");
        assert_eq!(mentions.len(), 1);
        assert_eq!(mentions[0].source_id, a);
    }

    #[test]
    fn test_unlinked_mentions_excludes_linked() {
        let mut graph = BlockGraph::new();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        graph.register_block(a, "pages/a.md", "see [[Target Page]] explicitly");
        graph.register_block(b, "pages/b.md", "I mention target page casually");
        graph.add_page_ref(a, "Target Page");

        let mentions = graph.find_unlinked_mentions("target page");
        // Only block b should be an unlinked mention; block a is explicitly linked
        assert_eq!(mentions.len(), 1);
        assert_eq!(mentions[0].source_id, b);
    }

    #[test]
    fn test_edge_count() {
        let mut graph = BlockGraph::new();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();
        graph.register_block(a, "p/a.md", "A");
        graph.register_block(b, "p/b.md", "B");
        graph.register_block(c, "p/c.md", "C");
        graph.add_block_ref(a, b);
        graph.add_block_ref(b, c);
        graph.add_page_ref(a, "Some Page");

        assert_eq!(graph.edge_count(), 3);
        assert_eq!(graph.block_count(), 3);
    }
}
