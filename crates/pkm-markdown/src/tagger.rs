use pkm_core::{Frontmatter, Tag, TagSource};
use regex::Regex;

/// Extract tags from raw markdown (frontmatter tags + inline #tags).
///
/// Frontmatter tags come from `Frontmatter.tags` (already parsed YAML).
/// Inline tags are `#word` patterns found in the body, excluding:
/// - Tags inside code blocks (fenced, indented) and inline code spans
/// - Tags preceded by a non-whitespace character (e.g., URLs `#section`)
/// - `##` heading markers
pub fn extract_tags(raw: &str, frontmatter: &Frontmatter) -> Vec<Tag> {
    let mut tags = Vec::new();

    // Frontmatter tags
    for tag_name in &frontmatter.tags {
        tags.push(Tag {
            name: tag_name.clone(),
            source: TagSource::Frontmatter,
            line: 0,
        });
    }

    // Extract body portion (skip frontmatter delimiter region)
    let body = extract_body(raw);

    // Use pulldown_cmark to find code block ranges in the body
    let code_ranges = find_code_block_ranges(body);

    // Inline #tags from body
    // Match #tag that starts with a letter, followed by word chars, hyphens, underscores, forward slashes
    // Must be preceded by whitespace or start-of-string (to avoid matching in URLs like example.com/#section)
    // or preceded by ( to allow tags in parenthetical contexts
    let re = match Regex::new(r"(?:^|[\s(])#([a-zA-Z][a-zA-Z0-9_\-/]*)") {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Failed to compile tag regex: {}", e);
            return tags;
        }
    };

    for cap in re.captures_iter(body) {
        let full_match = cap.get(0).unwrap();
        let tag_name = cap.get(1).unwrap().as_str();

        // Skip if the # is at position 0 but matched by lookahead
        // The match starts with the whitespace/start before #
        let hash_pos = full_match.start()
            + if full_match.as_str().starts_with('#') {
                0
            } else {
                1
            };

        // Skip if the tag is inside a code block
        if is_in_code_block(hash_pos, &code_ranges) {
            continue;
        }

        // Calculate line number based on position in body
        let line = body[..hash_pos].matches('\n').count() + 1;

        tags.push(Tag {
            name: tag_name.to_string(),
            source: TagSource::Inline,
            line,
        });
    }

    tags
}

/// Extract the body portion from raw markdown (strip frontmatter).
fn extract_body(raw: &str) -> &str {
    let trimmed = raw.trim_start();
    if trimmed.starts_with("---") {
        if let Some(end) = trimmed.strip_prefix("---").unwrap().find("\n---") {
            let body_start = 3 + end + 3;
            return trimmed[body_start..].trim_start();
        }
    }
    raw
}

/// Use pulldown_cmark to find code block ranges in text.
fn find_code_block_ranges(text: &str) -> Vec<std::ops::Range<usize>> {
    use pulldown_cmark::{Event, Tag, TagEnd};

    let mut ranges = Vec::new();
    let mut in_code_block = false;
    let mut code_block_start = 0usize;

    for (event, range) in pulldown_cmark::Parser::new(text).into_offset_iter() {
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
            Event::Code(_) if range.start < range.end => {
                ranges.push(range.start..range.end);
            }
            _ => {}
        }
    }

    ranges
}

/// Check if a byte position falls within any of the given ranges.
fn is_in_code_block(pos: usize, ranges: &[std::ops::Range<usize>]) -> bool {
    ranges.iter().any(|r| r.contains(&pos))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_fm(tags: Vec<&str>) -> Frontmatter {
        Frontmatter {
            tags: tags.iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn test_frontmatter_tags() {
        let raw = "---\ntags: [quantum, physics]\n---\nBody text.";
        let fm = make_fm(vec!["quantum", "physics"]);
        let tags = extract_tags(raw, &fm);
        assert!(tags.iter().any(|t| t.name == "quantum"));
        assert!(tags.iter().any(|t| t.name == "physics"));
    }

    #[test]
    fn test_inline_tags() {
        let raw = "Body text with #quantum and #physics tags.";
        let fm = Frontmatter::default();
        let tags = extract_tags(raw, &fm);
        assert!(tags
            .iter()
            .any(|t| t.name == "quantum" && t.source == TagSource::Inline));
        assert!(tags
            .iter()
            .any(|t| t.name == "physics" && t.source == TagSource::Inline));
    }

    #[test]
    fn test_mixed_tags() {
        let raw = "---\ntags: [frontmatter-tag]\n---\nBody with #inline-tag and [[link]].";
        let fm = make_fm(vec!["frontmatter-tag"]);
        let tags = extract_tags(raw, &fm);
        assert!(tags.iter().any(|t| t.name == "frontmatter-tag"));
        assert!(tags.iter().any(|t| t.name == "inline-tag"));
        assert_eq!(tags.len(), 2);
    }

    #[test]
    fn test_no_tags() {
        let raw = "Plain text with no tags.";
        let fm = Frontmatter::default();
        let tags = extract_tags(raw, &fm);
        assert!(tags.is_empty());
    }

    #[test]
    fn test_skip_headings() {
        let raw = "## Not a tag\n#also not a tag\nThis is #atag though.";
        let fm = Frontmatter::default();
        let tags = extract_tags(raw, &fm);
        // #atag is inline, #also starts a line but is still a valid tag
        assert!(tags.iter().any(|t| t.name == "atag"));
        assert!(tags.iter().any(|t| t.name == "also"));
        // ## is not a valid tag (doesn't start with [a-zA-Z])
        assert!(!tags.iter().any(|t| t.name == "Not"));
        assert_eq!(tags.len(), 2);
    }

    #[test]
    fn test_hierarchical_tags() {
        let raw = "Tag with #project/stratum/backend path.";
        let fm = Frontmatter::default();
        let tags = extract_tags(raw, &fm);
        assert!(tags.iter().any(|t| t.name == "project/stratum/backend"));
    }

    #[test]
    fn test_tags_with_hyphens_and_underscores() {
        let raw = "Tags: #my-tag and #another_tag and #tag/with-path.";
        let fm = Frontmatter::default();
        let tags = extract_tags(raw, &fm);
        assert!(tags.iter().any(|t| t.name == "my-tag"));
        assert!(tags.iter().any(|t| t.name == "another_tag"));
        assert!(tags.iter().any(|t| t.name == "tag/with-path"));
    }

    #[test]
    fn test_tags_in_code_block() {
        let raw = "Outside #tag1\n```\ninside #tag2 not captured\n#tag3\n```\nOutside again #tag4";
        let fm = Frontmatter::default();
        let tags = extract_tags(raw, &fm);
        assert!(tags.iter().any(|t| t.name == "tag1"));
        assert!(tags.iter().any(|t| t.name == "tag4"));
        // tag2 and tag3 are inside code block — should NOT be captured
        assert!(!tags.iter().any(|t| t.name == "tag2"));
        assert!(!tags.iter().any(|t| t.name == "tag3"));
        assert_eq!(tags.len(), 2);
    }

    #[test]
    fn test_tags_in_inline_code() {
        let raw = "Outside #visible and `inside #hidden` and #visible2.";
        let fm = Frontmatter::default();
        let tags = extract_tags(raw, &fm);
        assert!(tags.iter().any(|t| t.name == "visible"));
        assert!(tags.iter().any(|t| t.name == "visible2"));
        // #hidden inside inline code — should NOT be captured
        assert!(!tags.iter().any(|t| t.name == "hidden"));
        assert_eq!(tags.len(), 2);
    }

    #[test]
    fn test_tags_skip_url_fragments() {
        let raw = "Visit https://example.com/page#section for info.";
        let fm = Frontmatter::default();
        let tags = extract_tags(raw, &fm);
        // #section is preceded by 'e' (non-whitespace), so it should NOT match
        assert!(tags.is_empty());
    }

    #[test]
    fn test_tags_line_numbers() {
        let raw = "first line\nsecond line #mytag\nthird line";
        let fm = Frontmatter::default();
        let tags = extract_tags(raw, &fm);
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].line, 2);
    }

    #[test]
    fn test_tags_at_start_of_line() {
        let raw = "#tag at start of line";
        let fm = Frontmatter::default();
        let tags = extract_tags(raw, &fm);
        assert!(tags.iter().any(|t| t.name == "tag"));
    }
}
