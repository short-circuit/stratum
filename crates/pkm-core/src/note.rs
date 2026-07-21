use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// A parsed note with frontmatter metadata and body content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    /// Absolute or vault-relative path to the .md file.
    pub path: PathBuf,
    /// Vault-relative path.
    pub rel_path: PathBuf,
    /// File name (without extension).
    pub slug: String,
    /// YAML frontmatter metadata.
    pub frontmatter: Frontmatter,
    /// Raw markdown body (without frontmatter).
    pub body: String,
    /// Full raw content (frontmatter + body).
    pub raw: String,
    /// Extracted wiki-style links.
    pub links: Vec<Link>,
    /// Tags extracted from frontmatter + body.
    pub tags: Vec<Tag>,
    /// File size in bytes.
    pub size_bytes: u64,
    /// Last modified timestamp from filesystem.
    pub modified_at: DateTime<Utc>,
}

/// YAML frontmatter metadata for a note.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Frontmatter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_yaml::Value>,
}

/// A wiki-style link `[[Note Name]]` or `[[Note Name|Display Text]]`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Link {
    /// The target note name (the slug or title to link to).
    pub target: String,
    /// Optional display text (after the `|` in `[[target|display]]`).
    pub display_text: Option<String>,
    /// Whether the target note actually exists in the vault.
    pub resolved: bool,
    /// Line number in the source file.
    pub line: usize,
}

/// A tag extracted from frontmatter or inline `#tag` syntax.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Tag {
    /// The tag name (e.g. "quantum", "computing").
    pub name: String,
    /// Whether this tag came from frontmatter or inline body.
    pub source: TagSource,
    /// Line number (0 for frontmatter tags).
    pub line: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TagSource {
    Frontmatter,
    Inline,
}

impl Note {
    #[allow(clippy::too_many_arguments)]
    /// Create a new note with the given parameters.
    pub fn new(
        path: PathBuf,
        vault_root: &Path,
        frontmatter: Frontmatter,
        body: String,
        raw: String,
        links: Vec<Link>,
        tags: Vec<Tag>,
        modified_at: DateTime<Utc>,
    ) -> Self {
        let rel_path = path.strip_prefix(vault_root).unwrap_or(&path).to_path_buf();
        let slug = Self::path_to_slug(&path);

        let size_bytes = raw.len() as u64;

        Note {
            path,
            rel_path,
            slug,
            frontmatter,
            body,
            raw,
            links,
            tags,
            size_bytes,
            modified_at,
        }
    }

    /// Derive a slug from a file path.
    pub fn path_to_slug(path: &Path) -> String {
        path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("untitled")
            .to_string()
    }

    /// Derive a title from the path, falling back to the slug.
    pub fn derive_title(&self) -> String {
        self.frontmatter
            .title
            .clone()
            .unwrap_or_else(|| self.slug.replace('-', " "))
    }

    /// Get the display name for the note (title or slug).
    pub fn display_name(&self) -> String {
        self.derive_title()
    }

    /// Check if this note has a specific tag.
    pub fn has_tag(&self, tag_name: &str) -> bool {
        self.tags.iter().any(|t| t.name == tag_name)
    }

    /// Check if this note links to a specific target.
    pub fn links_to(&self, target: &str) -> bool {
        self.links.iter().any(|l| l.target == target)
    }

    /// Get all tags as a set of strings.
    pub fn tag_names(&self) -> HashSet<String> {
        self.tags.iter().map(|t| t.name.clone()).collect()
    }

    /// Get all link targets.
    pub fn link_targets(&self) -> HashSet<String> {
        self.links.iter().map(|l| l.target.clone()).collect()
    }
}

/// A reference from one note to another (backlink).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Backlink {
    /// The note that contains the link.
    pub source: PathBuf,
    /// The source note's title.
    pub source_title: String,
    /// The target note path.
    pub target: PathBuf,
    /// The target note's title.
    pub target_title: String,
    /// The display text used in the link (if any).
    pub display_text: Option<String>,
    /// Context snippet around the link.
    pub context_snippet: String,
    /// Line number of the link in the source.
    pub line: usize,
}

/// Metadata about the vault state.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VaultMeta {
    pub note_count: usize,
    pub total_size_bytes: u64,
    pub last_indexed: Option<DateTime<Utc>>,
    pub last_synced: Option<DateTime<Utc>>,
    pub version: String,
}

impl VaultMeta {
    pub fn new() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_test_note() -> Note {
        let path = PathBuf::from("/vault/quantum-computing.md");
        let vault_root = PathBuf::from("/vault");
        let fm = Frontmatter {
            title: Some("Quantum Computing".to_string()),
            tags: vec!["quantum".to_string(), "computing".to_string()],
            aliases: vec!["QC".to_string()],
            ..Default::default()
        };
        Note::new(
            path,
            &vault_root,
            fm,
            "Body content [[Superposition]] here #physics".to_string(),
            "---\ntitle: Quantum Computing\ntags: [quantum, computing]\n---\nBody content [[Superposition]] here #physics".to_string(),
            vec![Link {
                target: "Superposition".to_string(),
                display_text: None,
                resolved: false,
                line: 1,
            }],
            vec![
                Tag { name: "quantum".to_string(), source: TagSource::Frontmatter, line: 0 },
                Tag { name: "computing".to_string(), source: TagSource::Frontmatter, line: 0 },
                Tag { name: "physics".to_string(), source: TagSource::Inline, line: 1 },
            ],
            Utc::now(),
        )
    }

    #[test]
    fn test_note_creation() {
        let note = make_test_note();
        assert_eq!(note.slug, "quantum-computing");
        assert_eq!(note.display_name(), "Quantum Computing");
        assert_eq!(note.rel_path, PathBuf::from("quantum-computing.md"));
    }

    #[test]
    fn test_path_to_slug() {
        assert_eq!(
            Note::path_to_slug(Path::new("/vault/my-note.md")),
            "my-note"
        );
        assert_eq!(Note::path_to_slug(Path::new("readme.md")), "readme");
    }

    #[test]
    fn test_has_tag() {
        let note = make_test_note();
        assert!(note.has_tag("quantum"));
        assert!(note.has_tag("computing"));
        assert!(note.has_tag("physics")); // inline tag
        assert!(!note.has_tag("nonexistent"));
    }

    #[test]
    fn test_links_to() {
        let note = make_test_note();
        assert!(note.links_to("Superposition"));
        assert!(!note.links_to("Entanglement"));
    }

    #[test]
    fn test_tag_names() {
        let note = make_test_note();
        let names = note.tag_names();
        assert!(names.contains("quantum"));
        assert!(names.contains("computing"));
        assert!(names.contains("physics"));
        assert_eq!(names.len(), 3);
    }

    #[test]
    fn test_link_targets() {
        let note = make_test_note();
        let targets = note.link_targets();
        assert!(targets.contains("Superposition"));
        assert_eq!(targets.len(), 1);
    }

    #[test]
    fn test_frontmatter_default() {
        let fm = Frontmatter::default();
        assert!(fm.title.is_none());
        assert!(fm.tags.is_empty());
        assert!(fm.aliases.is_empty());
    }

    #[test]
    fn test_vault_meta() {
        let meta = VaultMeta::new();
        assert_eq!(meta.note_count, 0);
        assert!(!meta.version.is_empty());
        assert!(meta.last_indexed.is_none());
    }

    #[test]
    fn test_link_display_text() {
        let link = Link {
            target: "Note".to_string(),
            display_text: Some("Custom Text".to_string()),
            resolved: true,
            line: 3,
        };
        assert_eq!(link.display_text.as_deref(), Some("Custom Text"));
        assert!(link.resolved);
    }

    #[test]
    fn test_tag_source_equality() {
        assert_eq!(TagSource::Frontmatter, TagSource::Frontmatter);
        assert_ne!(TagSource::Frontmatter, TagSource::Inline);
    }
}
