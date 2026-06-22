use pkm_core::{Backlink, Note};
use std::collections::{HashMap, HashSet, VecDeque};

/// A node in the note graph.
#[derive(Debug, Clone)]
pub struct Node {
    pub path: String,
    pub title: String,
    pub slug: String,
    pub tags: Vec<String>,
}

/// A directed edge from source note to target note (wiki-link).
#[derive(Debug, Clone)]
pub struct Edge {
    pub source: String,
    pub target: String,
    pub display_text: Option<String>,
}

/// The note graph structure with nodes and directed edges.
#[derive(Debug, Clone)]
pub struct Graph {
    nodes: HashMap<String, Node>,         // slug -> Node
    outgoing: HashMap<String, Vec<Edge>>, // source slug -> edges
    incoming: HashMap<String, Vec<Edge>>, // target slug -> edges (backlinks)
}

impl Graph {
    /// Create a new empty graph.
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            outgoing: HashMap::new(),
            incoming: HashMap::new(),
        }
    }

    /// Add or update a node in the graph.
    pub fn add_node(&mut self, note: &Note) {
        let slug = note.slug.clone();
        let title = note.derive_title();
        let tags: Vec<String> = note.tags.iter().map(|t| t.name.clone()).collect();

        self.nodes.insert(
            slug.clone(),
            Node {
                path: note.rel_path.to_string_lossy().to_string(),
                title,
                slug: slug.clone(),
                tags,
            },
        );

        // Ensure edge lists exist
        self.outgoing.entry(slug.clone()).or_default();
        self.incoming.entry(slug).or_default();
    }

    /// Add an edge representing a wiki-link from source to target.
    pub fn add_edge(&mut self, source_slug: &str, target_slug: &str, display_text: Option<String>) {
        let edge = Edge {
            source: source_slug.to_string(),
            target: target_slug.to_string(),
            display_text,
        };

        self.outgoing
            .entry(source_slug.to_string())
            .or_default()
            .push(edge.clone());

        self.incoming
            .entry(target_slug.to_string())
            .or_default()
            .push(edge);
    }

    /// Get backlinks (notes that link to the given target slug).
    pub fn get_backlinks(&self, target_slug: &str) -> Vec<Backlink> {
        let mut backlinks = Vec::new();

        if let Some(edges) = self.incoming.get(target_slug) {
            for edge in edges {
                if let Some(source_node) = self.nodes.get(&edge.source) {
                    let target_node = self.nodes.get(target_slug);
                    backlinks.push(Backlink {
                        source: std::path::PathBuf::from(&source_node.path),
                        source_title: source_node.title.clone(),
                        target: target_node
                            .map(|n| std::path::PathBuf::from(&n.path))
                            .unwrap_or_else(|| std::path::PathBuf::from(target_slug)),
                        target_title: target_node
                            .map(|n| n.title.clone())
                            .unwrap_or_else(|| target_slug.to_string()),
                        display_text: edge.display_text.clone(),
                        context_snippet: String::new(), // Would need full text to populate
                        line: 0,
                    });
                }
            }
        }

        backlinks
    }

    /// Get outgoing links from a source slug.
    pub fn get_outgoing_links(&self, source_slug: &str) -> Vec<Edge> {
        self.outgoing
            .get(source_slug)
            .cloned()
            .unwrap_or_default()
    }

    /// Find text that matches a note title but isn't linked (unlinked mentions).
    /// Checks body text against all known note titles/slugs.
    pub fn find_unlinked_mentions(&self, note: &Note) -> Vec<String> {
        let body = note.body.to_lowercase();
        let existing_targets: HashSet<String> = note
            .links
            .iter()
            .map(|l| l.target.to_lowercase())
            .collect();

        let mut mentions = Vec::new();

        for (slug, node) in &self.nodes {
            // Skip self-references
            if slug == &note.slug {
                continue;
            }

            // Check if the note title appears in the body
            let title_lower = node.title.to_lowercase();
            if !title_lower.is_empty() && body.contains(&title_lower) {
                // Check if it's already linked (by slug match or title match)
                let slug_lower = slug.to_lowercase();
                if !existing_targets.contains(&slug_lower)
                    && !existing_targets.contains(&title_lower)
                {
                    mentions.push(node.title.clone());
                }
            }
        }

        mentions
    }

    /// Compute connected components. Returns groups of connected note slugs.
    pub fn connected_components(&self) -> Vec<Vec<String>> {
        let mut visited: HashSet<String> = HashSet::new();
        let mut components = Vec::new();

        for slug in self.nodes.keys() {
            if visited.contains(slug) {
                continue;
            }

            // BFS
            let mut component = Vec::new();
            let mut queue = VecDeque::new();
            queue.push_back(slug.clone());
            visited.insert(slug.clone());

            while let Some(current) = queue.pop_front() {
                component.push(current.clone());

                // Follow outgoing edges
                if let Some(edges) = self.outgoing.get(&current) {
                    for edge in edges {
                        if visited.insert(edge.target.clone()) {
                            queue.push_back(edge.target.clone());
                        }
                    }
                }

                // Follow incoming edges
                if let Some(edges) = self.incoming.get(&current) {
                    for edge in edges {
                        if visited.insert(edge.source.clone()) {
                            queue.push_back(edge.source.clone());
                        }
                    }
                }
            }

            components.push(component);
        }

        components
    }

    /// Find orphaned notes (notes with no incoming or outgoing links).
    pub fn find_orphaned_notes(&self) -> Vec<String> {
        self.nodes
            .keys()
            .filter(|slug| {
                let has_outgoing = self
                    .outgoing
                    .get(*slug)
                    .map(|e| !e.is_empty())
                    .unwrap_or(false);
                let has_incoming = self
                    .incoming
                    .get(*slug)
                    .map(|e| !e.is_empty())
                    .unwrap_or(false);
                !has_outgoing && !has_incoming
            })
            .cloned()
            .collect()
    }

    /// Get all nodes in the graph.
    pub fn all_nodes(&self) -> Vec<&Node> {
        self.nodes.values().collect()
    }

    /// Get a node by slug.
    pub fn get_node(&self, slug: &str) -> Option<&Node> {
        self.nodes.get(slug)
    }

    /// Get total node count.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get total edge count.
    pub fn edge_count(&self) -> usize {
        self.outgoing.values().map(|v| v.len()).sum()
    }
}

impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pkm_core::{Frontmatter, Link as PkmLink, Tag, TagSource};
    use std::path::PathBuf;

    fn make_note(path: &str, title: &str, _slug: &str, links: Vec<&str>, tags: Vec<&str>) -> Note {
        let path = PathBuf::from(path);
        let vault_root = PathBuf::from("/vault");
        let pkm_links: Vec<PkmLink> = links
            .iter()
            .enumerate()
            .map(|(i, t)| PkmLink {
                target: t.to_string(),
                display_text: None,
                resolved: false,
                line: i + 1,
            })
            .collect();
        let pkm_tags: Vec<Tag> = tags
            .iter()
            .map(|t| Tag {
                name: t.to_string(),
                source: TagSource::Frontmatter,
                line: 0,
            })
            .collect();

        Note::new(
            path,
            &vault_root,
            Frontmatter {
                title: Some(title.to_string()),
                tags: tags.iter().map(|s| s.to_string()).collect(),
                ..Default::default()
            },
            "Body content".to_string(),
            format!("---\ntitle: {}\n---\nBody content", title),
            pkm_links,
            pkm_tags,
            chrono::Utc::now(),
        )
    }

    fn build_test_graph() -> Graph {
        let mut graph = Graph::new();

        let note_a = make_note(
            "/vault/note-a.md",
            "Note A",
            "note-a",
            vec!["note-b", "note-c"],
            vec!["tag1"],
        );
        let note_b = make_note(
            "/vault/note-b.md",
            "Note B",
            "note-b",
            vec!["note-c"],
            vec!["tag2"],
        );
        let note_c = make_note(
            "/vault/note-c.md",
            "Note C",
            "note-c",
            vec![],
            vec!["tag1", "tag3"],
        );

        graph.add_node(&note_a);
        graph.add_node(&note_b);
        graph.add_node(&note_c);

        // Add edges from links
        for note in &[&note_a, &note_b] {
            for link in &note.links {
                graph.add_edge(&note.slug, &link.target, link.display_text.clone());
            }
        }

        graph
    }

    #[test]
    fn test_add_node_and_count() {
        let mut graph = Graph::new();
        let note = make_note("/vault/test.md", "Test", "test", vec![], vec![]);
        graph.add_node(&note);
        assert_eq!(graph.node_count(), 1);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_get_backlinks() {
        let graph = build_test_graph();
        let backlinks = graph.get_backlinks("note-c");
        assert_eq!(backlinks.len(), 2); // A and B both link to C

        let sources: Vec<String> = backlinks.iter().map(|b| b.source_title.clone()).collect();
        assert!(sources.contains(&"Note A".to_string()));
        assert!(sources.contains(&"Note B".to_string()));
    }

    #[test]
    fn test_get_outgoing_links() {
        let graph = build_test_graph();
        let edges = graph.get_outgoing_links("note-a");
        assert_eq!(edges.len(), 2);
        assert_eq!(edges[0].target, "note-b");
        assert_eq!(edges[1].target, "note-c");
    }

    #[test]
    fn test_orphaned_notes() {
        let mut graph = build_test_graph();
        // Note C has no outgoing links and is only a target
        let orphans = graph.find_orphaned_notes();
        assert!(!orphans.contains(&"note-c".to_string())); // C has incoming links

        // Add an orphan
        let orphan = make_note("/vault/orphan.md", "Orphan", "orphan", vec![], vec![]);
        graph.add_node(&orphan);
        let orphans = graph.find_orphaned_notes();
        assert!(orphans.contains(&"orphan".to_string()));
    }

    #[test]
    fn test_connected_components() {
        let graph = build_test_graph();
        let components = graph.connected_components();
        // All 3 notes are connected (A->B, A->C, B->C)
        assert_eq!(components.len(), 1);
        assert_eq!(components[0].len(), 3);
    }

    #[test]
    fn test_disconnected_components() {
        let mut graph = build_test_graph();
        let orphan = make_note("/vault/alone.md", "Alone", "alone", vec![], vec![]);
        graph.add_node(&orphan);

        let components = graph.connected_components();
        assert_eq!(components.len(), 2);

        let total: usize = components.iter().map(|c| c.len()).sum();
        assert_eq!(total, 4);
    }

    #[test]
    fn test_unlinked_mentions() {
        let mut graph = build_test_graph();

        // Create a note with body text mentioning another note's title but no link
        let note_with_mention = Note::new(
            PathBuf::from("/vault/mentioner.md"),
            &PathBuf::from("/vault"),
            Frontmatter {
                title: Some("Mentioner".to_string()),
                ..Default::default()
            },
            "This text mentions Note B but doesn't link it.".to_string(),
            "---\ntitle: Mentioner\n---\nThis text mentions Note B but doesn't link it.".to_string(),
            vec![],
            vec![],
            chrono::Utc::now(),
        );

        graph.add_node(&note_with_mention);
        let mentions = graph.find_unlinked_mentions(&note_with_mention);
        assert!(mentions.contains(&"Note B".to_string()));
    }

    #[test]
    fn test_node_tags() {
        let graph = build_test_graph();
        let node = graph.get_node("note-a").unwrap();
        assert!(node.tags.contains(&"tag1".to_string()));
        assert_eq!(node.slug, "note-a");
    }
}
