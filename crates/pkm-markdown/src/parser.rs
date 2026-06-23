use chrono::{DateTime, Utc};
use pkm_core::{Frontmatter, Link, Note, PkmError, PkmResult, Tag};
use std::path::Path;

/// A parsed markdown document with extracted metadata, links, and tags.
///
/// This is the intermediate result from parsing raw markdown content,
/// before path/context information is added to create a full `Note`.
#[derive(Debug, Clone)]
pub struct ParsedDocument {
    pub frontmatter: Frontmatter,
    pub body: String,
    pub raw: String,
    pub links: Vec<Link>,
    pub tags: Vec<Tag>,
}

/// Parse a markdown file from disk into a `Note`.
pub fn parse_file(path: &Path, vault_root: &Path) -> PkmResult<Note> {
    let raw = std::fs::read_to_string(path).map_err(PkmError::Io)?;
    let metadata = std::fs::metadata(path).map_err(PkmError::Io)?;
    let modified_at: DateTime<Utc> = metadata.modified().map_err(PkmError::Io)?.into();
    let parsed = parse_raw(&raw);

    Ok(Note::new(
        path.to_path_buf(),
        vault_root,
        parsed.frontmatter,
        parsed.body,
        parsed.raw,
        parsed.links,
        parsed.tags,
        modified_at,
    ))
}

/// Parse raw markdown content (without reading from disk) into a `Note`.
pub fn parse_content(
    raw: &str,
    path: &Path,
    vault_root: &Path,
    modified_at: DateTime<Utc>,
) -> PkmResult<Note> {
    let parsed = parse_raw(raw);

    Ok(Note::new(
        path.to_path_buf(),
        vault_root,
        parsed.frontmatter,
        parsed.body,
        parsed.raw,
        parsed.links,
        parsed.tags,
        modified_at,
    ))
}

/// Parse raw markdown content into a `ParsedDocument`.
///
/// This is the core parsing function. It:
/// 1. Splits frontmatter (YAML between `---` delimiters) from body
/// 2. Parses frontmatter YAML into `Frontmatter`
/// 3. Extracts wiki links via the linker module
/// 4. Extracts tags via the tagger module
pub fn parse_raw(raw: &str) -> ParsedDocument {
    let (frontmatter, body) = parse_frontmatter(raw);
    let links = super::linker::extract_links(&body);
    let tags = super::tagger::extract_tags(raw, &frontmatter);

    ParsedDocument {
        frontmatter,
        body,
        raw: raw.to_string(),
        links,
        tags,
    }
}

/// Extract YAML frontmatter and body from raw markdown.
///
/// Recognizes frontmatter delimited by `---` at the start of the content
/// (after optional whitespace). If no valid frontmatter is found, returns
/// an empty/default frontmatter and treats the entire input as body.
pub fn parse_frontmatter(raw: &str) -> (Frontmatter, String) {
    let trimmed = raw.trim_start();

    if trimmed.starts_with("---") {
        let after_opener = trimmed.strip_prefix("---").unwrap();

        if let Some(end) = find_closing_delimiter(after_opener) {
            let yaml_str = &after_opener[..end];
            let body = after_opener[end + 3..].trim_start().to_string();

            let fm: Frontmatter = match serde_yaml::from_str(yaml_str) {
                Ok(fm) => fm,
                Err(e) => {
                    tracing::warn!("Failed to parse frontmatter YAML: {}", e);
                    Frontmatter::default()
                }
            };

            return (fm, body);
        }
    }

    (Frontmatter::default(), raw.to_string())
}

/// Find the closing `---` delimiter in frontmatter content.
///
/// The closing delimiter must be at the start of a line (after `\n` or at string start).
fn find_closing_delimiter(s: &str) -> Option<usize> {
    let mut search_start = 0;

    loop {
        if let Some(pos) = s[search_start..].find("\n---") {
            let delimiter_start = search_start + pos;
            let after_delim = delimiter_start + 4; // len of "\n---"

            if after_delim >= s.len() {
                return Some(delimiter_start + 1);
            }

            let next_char = s[after_delim..].chars().next().unwrap_or(' ');
            if next_char == '\n' || next_char == '\r' || next_char.is_whitespace() {
                return Some(delimiter_start + 1);
            }

            search_start = delimiter_start + 1;
        } else {
            return None;
        }
    }
}

/// Walk the body to find code block ranges.
/// Uses pulldown_cmark to identify fenced code blocks, indented code blocks,
/// and inline code spans, returning their byte ranges in the body.
pub fn find_code_block_ranges(body: &str) -> Vec<std::ops::Range<usize>> {
    use pulldown_cmark::{Event, Tag, TagEnd};

    let mut ranges = Vec::new();
    let mut in_code_block = false;
    let mut code_block_start = 0usize;

    for (event, range) in pulldown_cmark::Parser::new(body).into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(_)) => {
                in_code_block = true;
                code_block_start = range.start;
            }
            Event::End(TagEnd::CodeBlock) => {
                if in_code_block {
                    ranges.push(code_block_start..range.end);
                    in_code_block = false;
                }
            }
            Event::Code(_code) if range.start < range.end => {
                ranges.push(range.start..range.end);
            }
            _ => {}
        }
    }

    ranges
}

/// Check if a byte position falls within any of the given ranges.
pub fn is_in_code_block(pos: usize, code_ranges: &[std::ops::Range<usize>]) -> bool {
    code_ranges.iter().any(|r| r.contains(&pos))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_parse_frontmatter_with_tags() {
        let raw = "---\ntitle: Test Note\ntags: [tag1, tag2]\n---\n\nBody content here.";
        let (fm, body) = parse_frontmatter(raw);
        assert_eq!(fm.title, Some("Test Note".to_string()));
        assert_eq!(fm.tags, vec!["tag1", "tag2"]);
        assert_eq!(body, "Body content here.");
    }

    #[test]
    fn test_parse_frontmatter_no_frontmatter() {
        let raw = "Just body text.";
        let (fm, body) = parse_frontmatter(raw);
        assert!(fm.title.is_none());
        assert_eq!(body, "Just body text.");
    }

    #[test]
    fn test_parse_frontmatter_empty() {
        let raw = "---\n---\nBody.";
        let (fm, body) = parse_frontmatter(raw);
        assert!(fm.title.is_none());
        assert_eq!(body, "Body.");
    }

    #[test]
    fn test_parse_frontmatter_extra_fields() {
        let raw = "---\ntitle: Extras\ntags: [test]\ncustom_field: value\nanother: 42\n---\nBody.";
        let (fm, body) = parse_frontmatter(raw);
        assert_eq!(fm.title, Some("Extras".to_string()));
        assert!(fm.extra.contains_key("custom_field"));
        assert!(fm.extra.contains_key("another"));
        assert_eq!(body, "Body.");
    }

    #[test]
    fn test_parse_frontmatter_empty_document() {
        let raw = "";
        let (fm, body) = parse_frontmatter(raw);
        assert!(fm.title.is_none());
        assert_eq!(body, "");
    }

    #[test]
    fn test_parse_frontmatter_whitespace_before_delimiter() {
        let raw = "  \n---\ntitle: Whitespace\n---\nBody.";
        let (fm, body) = parse_frontmatter(raw);
        assert_eq!(fm.title, Some("Whitespace".to_string()));
        assert_eq!(body, "Body.");
    }

    #[test]
    fn test_parse_frontmatter_with_aliases() {
        let raw = "---\ntitle: Main Note\naliases: [Alias1, Alias2]\n---\nBody.";
        let (fm, _) = parse_frontmatter(raw);
        assert_eq!(fm.title, Some("Main Note".to_string()));
        assert_eq!(fm.aliases, vec!["Alias1", "Alias2"]);
    }

    #[test]
    fn test_parse_raw_full_document() {
        let raw = "---\ntitle: Full Test\ntags: [test, demo]\n---\n\nBody with [[Wiki Link]] and #inline tag.";
        let doc = parse_raw(raw);
        assert_eq!(doc.frontmatter.title, Some("Full Test".to_string()));
        assert!(!doc.body.is_empty());
        assert_eq!(doc.raw, raw);
        assert!(doc.links.iter().any(|l| l.target == "Wiki Link"));
        assert!(doc.tags.iter().any(|t| t.name == "test"));
        assert!(doc.tags.iter().any(|t| t.name == "demo"));
        assert!(doc.tags.iter().any(|t| t.name == "inline"));
    }

    #[test]
    fn test_parse_raw_no_frontmatter() {
        let raw = "Just body with #tag and [[link]].";
        let doc = parse_raw(raw);
        assert!(doc.frontmatter.title.is_none());
        assert!(doc.frontmatter.tags.is_empty());
        assert_eq!(doc.body, raw);
        assert_eq!(doc.links.len(), 1);
        assert_eq!(doc.tags.len(), 1);
    }

    #[test]
    fn test_parse_raw_empty() {
        let raw = "";
        let doc = parse_raw(raw);
        assert!(doc.frontmatter.title.is_none());
        assert_eq!(doc.body, "");
        assert!(doc.links.is_empty());
        assert!(doc.tags.is_empty());
    }

    #[test]
    fn test_find_code_block_ranges_fenced() {
        let body = "Some text\n```\ncode here\n#notatag\n```\nMore text.";
        let ranges = find_code_block_ranges(body);
        assert!(
            !ranges.is_empty(),
            "Should find at least one code block range"
        );
    }

    #[test]
    fn test_find_closing_delimiter() {
        // "\n---" is at position 0, function returns position of "---" = 1
        assert_eq!(find_closing_delimiter("\n---\nbody"), Some(1));
        assert_eq!(find_closing_delimiter("\n---\n"), Some(1));

        // Find "\n---" somewhere in the middle
        let s = "title: Test\ntags: [a]\n---\nbody";
        // The "\n---" starts after "title: Test\ntags: [a]"
        let expected = s.find("\n---").unwrap() + 1;
        assert_eq!(find_closing_delimiter(s), Some(expected));

        // No close
        assert_eq!(find_closing_delimiter("title: Test"), None);
    }

    #[test]
    fn test_parse_file_nonexistent() {
        let path = PathBuf::from("/nonexistent/file.md");
        let vault = PathBuf::from("/");
        let result = parse_file(&path, &vault);
        assert!(result.is_err());
    }
}
