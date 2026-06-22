use pkm_core::{Frontmatter, Note};

/// Render a `Note` back to its markdown string representation.
///
/// The output is semantically equivalent to the original input that was parsed.
/// Frontmatter is reconstructed from structured data, and the body is preserved.
pub fn render_note(note: &Note) -> String {
    let mut output = String::new();

    if has_frontmatter(&note.frontmatter) {
        output.push_str(&render_frontmatter(&note.frontmatter));
    }

    output.push_str(&note.body);
    output
}

/// Render raw components (frontmatter + body) back to markdown.
pub fn render(frontmatter: &Frontmatter, body: &str) -> String {
    let mut output = String::new();

    if has_frontmatter(frontmatter) {
        output.push_str(&render_frontmatter(frontmatter));
    }

    output.push_str(body);
    output
}

/// Render the frontmatter portion as YAML between `---` delimiters.
///
/// Only includes non-empty fields and extra fields. The output is valid YAML
/// that can be parsed back by `serde_yaml`.
pub fn render_frontmatter(fm: &Frontmatter) -> String {
    let mut yaml = String::from("---\n");

    // Standard fields in canonical order
    if let Some(ref title) = fm.title {
        yaml.push_str(&format!("title: {:?}\n", title));
    }

    if let Some(ref created) = fm.created {
        yaml.push_str(&format!("created: {:?}\n", created));
    }

    if let Some(ref modified) = fm.modified {
        yaml.push_str(&format!("modified: {:?}\n", modified));
    }

    if !fm.tags.is_empty() {
        yaml.push_str("tags: [");
        for (i, tag) in fm.tags.iter().enumerate() {
            if i > 0 {
                yaml.push_str(", ");
            }
            yaml.push_str(&format!("{:?}", tag));
        }
        yaml.push_str("]\n");
    }

    if !fm.aliases.is_empty() {
        yaml.push_str("aliases: [");
        for (i, alias) in fm.aliases.iter().enumerate() {
            if i > 0 {
                yaml.push_str(", ");
            }
            yaml.push_str(&format!("{:?}", alias));
        }
        yaml.push_str("]\n");
    }

    // Extra fields (flattened)
    for (key, value) in &fm.extra {
        // Serialize the YAML value back to a string representation
        let value_str = serde_yaml::to_string(value)
            .unwrap_or_default()
            .trim()
            .to_string();
        yaml.push_str(&format!("{}: {}\n", key, value_str));
    }

    yaml.push_str("---\n");
    yaml
}

/// Check if a frontmatter has any meaningful content to render.
fn has_frontmatter(fm: &Frontmatter) -> bool {
    fm.title.is_some()
        || fm.created.is_some()
        || fm.modified.is_some()
        || !fm.tags.is_empty()
        || !fm.aliases.is_empty()
        || !fm.extra.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn sample_frontmatter() -> Frontmatter {
        Frontmatter {
            title: Some("Test Note".to_string()),
            created: Some("2024-01-15".to_string()),
            tags: vec!["rust".to_string(), "pkm".to_string()],
            aliases: vec!["test".to_string()],
            extra: {
                let mut m = HashMap::new();
                m.insert(
                    "custom_field".to_string(),
                    serde_yaml::Value::String("value".to_string()),
                );
                m
            },
            ..Default::default()
        }
    }

    #[test]
    fn test_render_frontmatter_preserves_title() {
        let yaml = render_frontmatter(&sample_frontmatter());
        assert!(yaml.contains("title:"));
        assert!(yaml.contains("Test Note"));
        assert!(yaml.starts_with("---\n"));
        assert!(yaml.ends_with("---\n"));
    }

    #[test]
    fn test_render_frontmatter_preserves_tags() {
        let yaml = render_frontmatter(&sample_frontmatter());
        assert!(yaml.contains("rust"));
        assert!(yaml.contains("pkm"));
    }

    #[test]
    fn test_render_frontmatter_preserves_aliases() {
        let yaml = render_frontmatter(&sample_frontmatter());
        assert!(yaml.contains("test"));
    }

    #[test]
    fn test_render_frontmatter_preserves_extra() {
        let yaml = render_frontmatter(&sample_frontmatter());
        assert!(yaml.contains("custom_field"));
        assert!(yaml.contains("value"));
    }

    #[test]
    fn test_render_empty_frontmatter() {
        let fm = Frontmatter::default();
        let result = render_frontmatter(&fm);
        assert_eq!(result, "---\n---\n");
    }

    #[test]
    fn test_render_full_note_with_frontmatter() {
        let fm = sample_frontmatter();
        let body = "Body content here.";
        let output = render(&fm, body);
        assert!(output.contains("---"));
        assert!(output.contains("Test Note"));
        assert!(output.contains("Body content here."));
    }

    #[test]
    fn test_render_full_note_without_frontmatter() {
        let fm = Frontmatter::default();
        let body = "Just body content.";
        let output = render(&fm, body);
        assert_eq!(output, "Just body content.");
    }

    #[test]
    fn test_render_round_trip() {
        // Parse a document, then render it, then parse again — should be semantically equivalent
        let original = "---\ntitle: Round Trip\ntags: [test]\n---\n\nBody with [[link]] and #tag.";
        let parsed = crate::parser::parse_raw(original);
        let rendered = render(&parsed.frontmatter, &parsed.body);
        let reparsed = crate::parser::parse_raw(&rendered);

        assert_eq!(reparsed.frontmatter.title, parsed.frontmatter.title);
        assert_eq!(reparsed.frontmatter.tags, parsed.frontmatter.tags);
        assert_eq!(reparsed.body.trim(), parsed.body.trim());
    }

    #[test]
    fn test_render_round_trip_no_frontmatter() {
        let original = "Just body with #tag and [[link]].";
        let parsed = crate::parser::parse_raw(original);
        let rendered = render(&parsed.frontmatter, &parsed.body);
        assert_eq!(rendered, original);
    }

    #[test]
    fn test_render_handles_empty_body() {
        let fm = sample_frontmatter();
        let output = render(&fm, "");
        assert!(output.contains("---"));
        assert!(output.ends_with("---\n"));
    }

    #[test]
    fn test_render_handles_special_characters() {
        let fm = Frontmatter {
            title: Some("Note with \"quotes\"".to_string()),
            tags: vec!["special-chars".to_string()],
            ..Default::default()
        };
        let body = "Body with *markdown* **syntax** and `code`.";
        let output = render(&fm, body);
        assert!(output.contains("quotes"));
        assert!(output.contains("*markdown*"));
        assert!(output.contains("`code`"));
    }

    #[test]
    fn test_render_frontmatter_only_title() {
        let fm = Frontmatter {
            title: Some("Minimal".to_string()),
            ..Default::default()
        };
        let yaml = render_frontmatter(&fm);
        assert!(yaml.contains("title:"));
        assert!(!yaml.contains("tags:"));
        assert!(!yaml.contains("aliases:"));
    }

    #[test]
    fn test_render_frontmatter_created_and_modified() {
        let fm = Frontmatter {
            title: Some("Dated".to_string()),
            created: Some("2024-01-01".to_string()),
            modified: Some("2024-06-15".to_string()),
            ..Default::default()
        };
        let yaml = render_frontmatter(&fm);
        assert!(yaml.contains("created:"));
        assert!(yaml.contains("modified:"));
    }
}
