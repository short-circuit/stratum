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
///
/// Node identity is the vault-relative page path, NOT the basename slug: two
/// notes with the same basename in different folders are distinct nodes. The
/// `path`/`slug`/`title` resolution helpers that back `get_backlinks`,
/// `get_outgoing_links`, and `get_node` accept either a registered path or a
/// slug; a slug shared by multiple paths is treated as ambiguous (empty / None).
#[derive(Debug, Clone)]
pub struct Graph {
    nodes: HashMap<String, Node>,         // path -> Node
    outgoing: HashMap<String, Vec<Edge>>, // source path -> edges
    incoming: HashMap<String, Vec<Edge>>, // target path -> edges (backlinks)
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

    /// Resolve a slug-or-path argument to a unique registered node path.
    ///
    /// If the argument is already a registered path, it is used directly.
    /// Otherwise the set of nodes whose basename slug equals the argument's
    /// basename is collected; if exactly one matches it is returned, and if
    /// multiple share the slug the match is ambiguous and `None` is returned.
    fn resolve_key_to_path(&self, key: &str) -> Option<String> {
        if self.nodes.contains_key(key) {
            return Some(key.to_string());
        }
        let basename = std::path::Path::new(key)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(key);
        let mut matches = self
            .nodes
            .iter()
            .filter(|(_, n)| n.slug == basename)
            .map(|(p, _)| p.clone());
        let first = matches.next();
        match first {
            Some(p) if matches.next().is_none() => Some(p),
            _ => None,
        }
    }

    /// Resolve a `[[wiki-link]]` target to a unique node path, or `None` when
    /// ambiguous or dead. Precedence: exact path, path-minus-`.md`, suffix path
    /// match, slugified basename, exact basename slug, then title match.
    pub fn resolve_target(&self, target: &str) -> Option<String> {
        let mut raw: Vec<String> = Vec::new();
        let stripped_target = target.strip_suffix(".md").unwrap_or(target);

        // Exact path match (with .md) and path-minus-.md.
        for path in self.nodes.keys() {
            let p_stripped = path.strip_suffix(".md").unwrap_or(path);
            if path == target || p_stripped == target || path == stripped_target {
                raw.push(path.clone());
            }
        }
        // Suffix path match.
        let suffix = format!("/{stripped_target}");
        for path in self.nodes.keys() {
            let p_stripped = path.strip_suffix(".md").unwrap_or(path);
            if p_stripped.ends_with(&suffix) || p_stripped.ends_with(&format!("/{target}")) {
                raw.push(path.clone());
            }
        }
        // Unqualified targets also match by slug and title.
        if !target.contains('/') {
            let slugified = target.replace(' ', "-").to_lowercase();
            let title_lower = target.to_lowercase();
            for (path, node) in &self.nodes {
                if node.slug == slugified
                    || node.slug == target
                    || node.title.to_lowercase() == title_lower
                {
                    raw.push(path.clone());
                }
            }
        }

        let mut seen: HashSet<String> = HashSet::new();
        let mut candidates: Vec<String> = Vec::new();
        for p in raw {
            if seen.insert(p.clone()) {
                candidates.push(p);
            }
        }
        if candidates.len() == 1 {
            candidates.into_iter().next()
        } else {
            None
        }
    }

    /// Add or update a node in the graph, keyed by its vault-relative path.
    pub fn add_node(&mut self, note: &Note) {
        let rel_path = note.rel_path.to_string_lossy().to_string();
        let slug = note.slug.clone();
        let title = note.derive_title();
        let tags: Vec<String> = note.tags.iter().map(|t| t.name.clone()).collect();

        self.nodes.insert(
            rel_path.clone(),
            Node {
                path: rel_path.clone(),
                title,
                slug,
                tags,
            },
        );

        // Ensure edge lists exist
        self.outgoing.entry(rel_path.clone()).or_default();
        self.incoming.entry(rel_path).or_default();
    }

    /// Add an edge representing a wiki-link from source path to target path.
    ///
    /// Both arguments are page PATHS (the stable node identity).
    pub fn add_edge(&mut self, source: &str, target: &str, display_text: Option<String>) {
        let edge = Edge {
            source: source.to_string(),
            target: target.to_string(),
            display_text,
        };

        self.outgoing
            .entry(source.to_string())
            .or_default()
            .push(edge.clone());

        self.incoming
            .entry(target.to_string())
            .or_default()
            .push(edge);
    }

    /// Get backlinks (notes that link to the given target).
    ///
    /// `key` may be a registered path or a slug; ambiguous slugs return no
    /// backlinks.
    pub fn get_backlinks(&self, key: &str) -> Vec<Backlink> {
        let Some(target_path) = self.resolve_key_to_path(key) else {
            return Vec::new();
        };
        let mut backlinks = Vec::new();

        if let Some(edges) = self.incoming.get(&target_path) {
            for edge in edges {
                if let Some(source_node) = self.nodes.get(&edge.source) {
                    let target_node = self.nodes.get(&target_path);
                    backlinks.push(Backlink {
                        source: std::path::PathBuf::from(&source_node.path),
                        source_title: source_node.title.clone(),
                        target: target_node
                            .map(|n| std::path::PathBuf::from(&n.path))
                            .unwrap_or_else(|| std::path::PathBuf::from(&target_path)),
                        target_title: target_node.map(|n| n.title.clone()).unwrap_or_default(),
                        display_text: edge.display_text.clone(),
                        context_snippet: String::new(), // Would need full text to populate
                        line: 0,
                    });
                }
            }
        }

        backlinks
    }

    /// Get outgoing links from a source node.
    ///
    /// `key` may be a registered path or a slug; ambiguous slugs return no
    /// outgoing links.
    pub fn get_outgoing_links(&self, key: &str) -> Vec<Edge> {
        let Some(path) = self.resolve_key_to_path(key) else {
            return Vec::new();
        };
        self.outgoing.get(&path).cloned().unwrap_or_default()
    }

    /// Find text that matches a note title but isn't linked (unlinked mentions).
    /// Checks body text against all known note titles/slugs.
    pub fn find_unlinked_mentions(&self, note: &Note) -> Vec<String> {
        let body = note.body.to_lowercase();
        let existing_targets: HashSet<String> =
            note.links.iter().map(|l| l.target.to_lowercase()).collect();
        let note_path = note.rel_path.to_string_lossy().to_string();

        let mut mentions = Vec::new();

        for node in self.nodes.values() {
            // Skip self-references (by path; paths are the node identity).
            if node.path == note_path {
                continue;
            }

            // Check if the note title appears in the body
            let title_lower = node.title.to_lowercase();
            if !title_lower.is_empty() && body.contains(&title_lower) {
                // Check if it's already linked (by slug match or title match)
                let slug_lower = node.slug.to_lowercase();
                if !existing_targets.contains(&slug_lower)
                    && !existing_targets.contains(&title_lower)
                {
                    mentions.push(node.title.clone());
                }
            }
        }

        mentions
    }

    /// Compute connected components. Returns groups of connected node paths.
    pub fn connected_components(&self) -> Vec<Vec<String>> {
        let mut visited: HashSet<String> = HashSet::new();
        let mut components = Vec::new();

        for path in self.nodes.keys() {
            if visited.contains(path) {
                continue;
            }

            // BFS
            let mut component = Vec::new();
            let mut queue = VecDeque::new();
            queue.push_back(path.clone());
            visited.insert(path.clone());

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
    /// Returns the vault-relative PATHS of orphaned nodes.
    pub fn find_orphaned_notes(&self) -> Vec<String> {
        self.nodes
            .keys()
            .filter(|path| {
                let has_outgoing = self
                    .outgoing
                    .get(*path)
                    .map(|e| !e.is_empty())
                    .unwrap_or(false);
                let has_incoming = self
                    .incoming
                    .get(*path)
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

    /// Get a node by key (registered path, or a unique slug).
    pub fn get_node(&self, key: &str) -> Option<&Node> {
        let path = self.resolve_key_to_path(key)?;
        self.nodes.get(&path)
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

        // Add edges from links, resolving targets to node paths.
        for note in &[&note_a, &note_b] {
            let src = note.rel_path.to_string_lossy().to_string();
            for link in &note.links {
                if let Some(target) = graph.resolve_target(&link.target) {
                    graph.add_edge(&src, &target, link.display_text.clone());
                }
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
        // Edge endpoints are node paths, not slugs.
        assert_eq!(edges[0].target, "note-b.md");
        assert_eq!(edges[1].target, "note-c.md");
    }

    #[test]
    fn test_orphaned_notes() {
        let mut graph = build_test_graph();
        // Note C has no outgoing links and is only a target
        let orphans = graph.find_orphaned_notes();
        assert!(!orphans.contains(&"note-c.md".to_string())); // C has incoming links

        // Add an orphan
        let orphan = make_note("/vault/orphan.md", "Orphan", "orphan", vec![], vec![]);
        graph.add_node(&orphan);
        let orphans = graph.find_orphaned_notes();
        assert!(orphans.contains(&"orphan.md".to_string()));
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
            "---\ntitle: Mentioner\n---\nThis text mentions Note B but doesn't link it."
                .to_string(),
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
        // get_node resolves a slug to a unique path, so this still works.
        let node = graph.get_node("note-a").unwrap();
        assert!(node.tags.contains(&"tag1".to_string()));
        assert_eq!(node.slug, "note-a");
    }

    #[test]
    fn test_duplicate_slugs_distinct_nodes() {
        let mut graph = Graph::new();

        let note_x = make_note(
            "/vault/a/dup.md",
            "Dup A",
            "dup",
            vec!["b/dup"],
            vec!["tag1"],
        );
        let note_y = make_note("/vault/b/dup.md", "Dup B", "dup", vec![], vec!["tag2"]);
        graph.add_node(&note_x);
        graph.add_node(&note_y);

        // Both distinct paths are distinct nodes despite the same basename slug.
        assert_eq!(graph.node_count(), 2, "duplicate slugs are distinct nodes");

        // Path-qualified resolution disambiguates the target.
        assert_eq!(graph.resolve_target("b/dup").as_deref(), Some("b/dup.md"));
        graph.add_edge("a/dup.md", "b/dup.md", None);
        assert_eq!(graph.edge_count(), 1);

        // The bare slug "dup" is ambiguous -> no backlinks on either path.
        assert!(graph.get_backlinks("dup").is_empty());
        assert!(graph.get_backlinks("a/dup.md").is_empty()); // no incoming
        assert_eq!(graph.get_backlinks("b/dup.md").len(), 1); // incoming from a/dup.md

        // Tags are per-path.
        assert!(graph
            .get_node("a/dup.md")
            .unwrap()
            .tags
            .contains(&"tag1".to_string()));
        assert!(graph
            .get_node("b/dup.md")
            .unwrap()
            .tags
            .contains(&"tag2".to_string()));
    }
}
