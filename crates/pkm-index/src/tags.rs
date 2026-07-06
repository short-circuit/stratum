use pkm_core::Note;
use std::collections::HashMap;

/// Tag aggregation across notes.
#[derive(Debug, Clone)]
pub struct TagAggregator {
    /// tag_name -> count of notes having this tag
    counts: HashMap<String, usize>,
    /// tag_name -> list of note slugs having this tag
    notes_by_tag: HashMap<String, Vec<String>>,
    /// slug -> list of tag names for that note (reverse mapping for decrement)
    note_tags: HashMap<String, Vec<String>>,
}

impl TagAggregator {
    /// Create a new empty aggregator.
    pub fn new() -> Self {
        Self {
            counts: HashMap::new(),
            notes_by_tag: HashMap::new(),
            note_tags: HashMap::new(),
        }
    }

    /// Aggregate tags from a collection of notes.
    pub fn aggregate(&mut self, notes: &[Note]) {
        for note in notes {
            let slug = note.slug.clone();
            let unique_tags: std::collections::HashSet<String> =
                note.tags.iter().map(|t| t.name.clone()).collect();

            for tag_name in &unique_tags {
                *self.counts.entry(tag_name.clone()).or_insert(0) += 1;
                self.notes_by_tag
                    .entry(tag_name.clone())
                    .or_default()
                    .push(slug.clone());
            }

            // Store reverse mapping for later decrement
            self.note_tags.insert(slug, unique_tags.into_iter().collect());
        }
    }

    /// Get notes that have a specific tag.
    pub fn filter_by_tag(&self, tag_name: &str) -> Vec<String> {
        self.notes_by_tag.get(tag_name).cloned().unwrap_or_default()
    }

    /// Get the tag cloud: list of (tag, count) sorted by count descending.
    pub fn get_tag_cloud(&self) -> Vec<(String, usize)> {
        let mut cloud: Vec<(String, usize)> = self.counts.clone().into_iter().collect();
        cloud.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        cloud
    }

    /// Get all tags with their counts.
    pub fn all_tags(&self) -> &HashMap<String, usize> {
        &self.counts
    }

    /// Get total number of unique tags.
    pub fn unique_tag_count(&self) -> usize {
        self.counts.len()
    }

    /// Decrement tag counts for a note that was removed or had its tags changed.
    ///
    /// Removes the note slug from `notes_by_tag`, decrements each tag's count,
    /// and cleans up any tag that reaches zero.
    pub fn decrement_note(&mut self, slug: &str) {
        let old_tags = match self.note_tags.remove(slug) {
            Some(tags) => tags,
            None => return, // No tags to decrement
        };

        for tag_name in old_tags {
            // Remove slug from the per-tag note list
            if let Some(notes) = self.notes_by_tag.get_mut(&tag_name) {
                notes.retain(|n| n != slug);
                if notes.is_empty() {
                    self.notes_by_tag.remove(&tag_name);
                }
            }

            // Decrement the count, cleaning up if it reaches zero
            if let Some(count) = self.counts.get_mut(&tag_name) {
                *count = count.saturating_sub(1);
                if *count == 0 {
                    self.counts.remove(&tag_name);
                }
            }
        }
    }
}

/// Hierarchical tag tree view (tags can have `/` separators).
#[derive(Debug, Clone)]
pub struct TagTree {
    root: TagNode,
}

#[derive(Debug, Clone)]
pub struct TagNode {
    name: String,
    count: usize,
    children: Vec<TagNode>,
}

impl TagTree {
    /// Build a hierarchical tree from tag aggregator data.
    pub fn build(aggregator: &TagAggregator) -> Self {
        let mut root = TagNode {
            name: "".to_string(),
            count: 0,
            children: Vec::new(),
        };

        let mut sorted_tags = aggregator.get_tag_cloud();
        sorted_tags.sort_by(|a, b| a.0.cmp(&b.0));

        for (tag_name, count) in &sorted_tags {
            let parts: Vec<&str> = tag_name.split('/').collect();
            Self::insert_path(&mut root, &parts, *count);
        }

        Self { root }
    }

    /// Insert a tag path into the tree.
    fn insert_path(node: &mut TagNode, parts: &[&str], count: usize) {
        if parts.is_empty() {
            return;
        }

        let part = parts[0].to_string();
        if let Some(child) = node.children.iter_mut().find(|c| c.name == part) {
            child.count += count;
            Self::insert_path(child, &parts[1..], count);
        } else {
            let mut child = TagNode {
                name: part,
                count,
                children: Vec::new(),
            };
            Self::insert_path(&mut child, &parts[1..], count);
            node.children.push(child);
        }
    }

    /// Render the tree as a list of indented strings.
    pub fn render(&self) -> Vec<String> {
        let mut lines = Vec::new();
        for child in &self.root.children {
            Self::render_node(child, 0, &mut lines);
        }
        lines
    }

    fn render_node(node: &TagNode, depth: usize, lines: &mut Vec<String>) {
        let indent = "  ".repeat(depth);
        if node.children.is_empty() {
            lines.push(format!("{}{} ({})", indent, node.name, node.count));
        } else {
            lines.push(format!("{}{}/ ({} total)", indent, node.name, node.count));
            for child in &node.children {
                Self::render_node(child, depth + 1, lines);
            }
        }
    }

    /// Get the root node for custom traversal.
    pub fn root(&self) -> &TagNode {
        &self.root
    }
}

impl Default for TagAggregator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pkm_core::{Frontmatter, Tag, TagSource};
    use std::path::PathBuf;

    fn make_note(slug: &str, title: &str, tag_names: Vec<&str>) -> Note {
        let tags: Vec<Tag> = tag_names
            .iter()
            .map(|t| Tag {
                name: t.to_string(),
                source: TagSource::Frontmatter,
                line: 0,
            })
            .collect();

        Note::new(
            PathBuf::from(format!("/vault/{}.md", slug)),
            &PathBuf::from("/vault"),
            Frontmatter {
                title: Some(title.to_string()),
                tags: tag_names.iter().map(|s| s.to_string()).collect(),
                ..Default::default()
            },
            "Body content".to_string(),
            format!("---\ntitle: {}\n---\nBody content", title),
            vec![],
            tags,
            chrono::Utc::now(),
        )
    }

    #[test]
    fn test_aggregate_tags() {
        let mut agg = TagAggregator::new();
        let notes = vec![
            make_note("note-1", "Note 1", vec!["tag1", "tag2"]),
            make_note("note-2", "Note 2", vec!["tag2", "tag3"]),
            make_note("note-3", "Note 3", vec!["tag1"]),
        ];
        agg.aggregate(&notes);

        assert_eq!(agg.unique_tag_count(), 3);
        let cloud = agg.get_tag_cloud();
        assert_eq!(cloud[0].0, "tag1"); // appears 2 times
        assert_eq!(cloud[0].1, 2);
        assert_eq!(cloud[1].0, "tag2"); // appears 2 times
        assert_eq!(cloud[1].1, 2);
    }

    #[test]
    fn test_filter_by_tag() {
        let mut agg = TagAggregator::new();
        let notes = vec![
            make_note("note-1", "Note 1", vec!["tag1"]),
            make_note("note-2", "Note 2", vec!["tag1", "tag2"]),
            make_note("note-3", "Note 3", vec!["tag2"]),
        ];
        agg.aggregate(&notes);

        let tagged_with_1 = agg.filter_by_tag("tag1");
        assert_eq!(tagged_with_1.len(), 2);
        assert!(tagged_with_1.contains(&"note-1".to_string()));
        assert!(tagged_with_1.contains(&"note-2".to_string()));

        let tagged_with_nonexistent = agg.filter_by_tag("nonexistent");
        assert!(tagged_with_nonexistent.is_empty());
    }

    #[test]
    fn test_tag_cloud() {
        let mut agg = TagAggregator::new();
        let notes = vec![
            make_note("note-1", "Note 1", vec!["alpha", "beta"]),
            make_note("note-2", "Note 2", vec!["beta"]),
            make_note("note-3", "Note 3", vec!["gamma"]),
        ];
        agg.aggregate(&notes);

        let cloud = agg.get_tag_cloud();
        // beta appears twice, alpha and gamma once
        assert_eq!(cloud[0].0, "beta");
        assert_eq!(cloud[0].1, 2);
        // alpha and gamma are tied at 1, sorted alphabetically
        assert_eq!(cloud[1].0, "alpha");
        assert_eq!(cloud[2].0, "gamma");
    }

    #[test]
    fn test_hierarchy_parsing() {
        let mut agg = TagAggregator::new();
        let notes = vec![
            make_note("note-1", "Note 1", vec!["project/stratum/core"]),
            make_note("note-2", "Note 2", vec!["project/stratum/ui"]),
            make_note("note-3", "Note 3", vec!["project/other"]),
        ];
        agg.aggregate(&notes);

        let tree = TagTree::build(&agg);
        let rendered = tree.render();
        assert!(!rendered.is_empty());
        // Should have top-level 'project' with children
        assert!(rendered.iter().any(|r| r.contains("project/")));
    }

    #[test]
    fn test_empty_aggregator() {
        let agg = TagAggregator::new();
        assert_eq!(agg.unique_tag_count(), 0);
        assert!(agg.get_tag_cloud().is_empty());
        assert!(agg.filter_by_tag("anything").is_empty());
    }

    #[test]
    fn test_tag_tree_render() {
        let mut agg = TagAggregator::new();
        let notes = vec![
            make_note("a", "A", vec!["science/physics/quantum"]),
            make_note("b", "B", vec!["science/physics/classical"]),
            make_note("c", "C", vec!["science/biology"]),
        ];
        agg.aggregate(&notes);

        let tree = TagTree::build(&agg);
        let rendered = tree.render();
        // Verify hierarchical output
        assert!(rendered.len() >= 2);
        // The first line should contain the top-level science tag
        assert!(rendered.iter().any(|r| r.contains("science")));
    }

    #[test]
    fn test_decrement_note_remove() {
        let mut agg = TagAggregator::new();
        let notes = vec![
            make_note("note-1", "Note 1", vec!["tag1", "tag2"]),
            make_note("note-2", "Note 2", vec!["tag2", "tag3"]),
            make_note("note-3", "Note 3", vec!["tag1"]),
        ];
        agg.aggregate(&notes);
        assert_eq!(agg.unique_tag_count(), 3);
        assert_eq!(agg.counts.get("tag1"), Some(&2));
        assert_eq!(agg.counts.get("tag2"), Some(&2));
        assert_eq!(agg.counts.get("tag3"), Some(&1));

        // Remove note-1 (had tag1, tag2)
        agg.decrement_note("note-1");
        assert_eq!(agg.unique_tag_count(), 3); // all 3 tags still present
        assert_eq!(agg.counts.get("tag1"), Some(&1)); // note-3 still has it
        assert_eq!(agg.counts.get("tag2"), Some(&1)); // note-2 still has it
        assert_eq!(agg.filter_by_tag("tag1"), vec!["note-3"]);
        assert!(agg.filter_by_tag("tag2").iter().any(|n| n == "note-2"));

        // Remove note-3 (had tag1) — tag1 should be cleaned up
        agg.decrement_note("note-3");
        assert_eq!(agg.unique_tag_count(), 2); // tag1 removed
        assert!(agg.counts.get("tag1").is_none());
        assert!(agg.filter_by_tag("tag1").is_empty());
    }

    #[test]
    fn test_decrement_note_update_tags() {
        let mut agg = TagAggregator::new();
        let notes = vec![
            make_note("note-1", "Note 1", vec!["tag1", "tag2"]),
            make_note("note-2", "Note 2", vec!["tag2"]),
        ];
        agg.aggregate(&notes);
        assert_eq!(agg.counts.get("tag2"), Some(&2));

        // Simulate an update: note-1 changes tags from [tag1, tag2] to [tag3]
        agg.decrement_note("note-1");
        // Now note-1's old tags are removed
        assert_eq!(agg.counts.get("tag1"), None); // only note-1 had it
        assert_eq!(agg.counts.get("tag2"), Some(&1)); // only note-2 remains
        assert!(agg.note_tags.get("note-1").is_none());

        // Now add new tags (simulating the new version of the note)
        let updated_note = make_note("note-1", "Note 1", vec!["tag3"]);
        agg.aggregate(std::slice::from_ref(&updated_note));
        assert_eq!(agg.counts.get("tag3"), Some(&1));
        assert_eq!(agg.note_tags.get("note-1"), Some(&vec!["tag3".to_string()]));
        assert_eq!(agg.unique_tag_count(), 2); // tag2, tag3
    }

    #[test]
    fn test_decrement_nonexistent_note() {
        let mut agg = TagAggregator::new();
        // Should not panic
        agg.decrement_note("never-indexed");
        assert_eq!(agg.unique_tag_count(), 0);
    }

    #[test]
    fn test_decrement_tag_to_zero_removes_entry() {
        let mut agg = TagAggregator::new();
        let notes = vec![make_note("lone", "Lone", vec!["lonely-tag"])];
        agg.aggregate(&notes);
        assert_eq!(agg.unique_tag_count(), 1);

        agg.decrement_note("lone");
        assert_eq!(agg.unique_tag_count(), 0);
        assert!(agg.counts.is_empty());
        assert!(agg.notes_by_tag.is_empty());
    }
}
