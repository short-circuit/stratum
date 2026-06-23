use pkm_core::Link;

/// Extract wiki-style links `[[target]]`, `[[target|display]]`, etc. from text.
///
/// ## Format
/// - `[[Target]]` — simple link
/// - `[[Target|Display Text]]` — link with display text
/// - `[[Target|Display Text|extra]]` — last pipe separates display (display = "Display Text|extra")
///
/// Handles nested brackets by tracking `[[`/`]]` depth.
/// Handles escaped pipes `\|` inside the link content (not treated as separator).
pub fn extract_links(text: &str) -> Vec<Link> {
    let mut links = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        if i + 1 < len && chars[i] == '[' && chars[i + 1] == '[' {
            let link_start = i;
            i += 2;
            let mut depth: i32 = 2;

            // Track content start (inside [[ ... ]])
            let content_start = i;

            while i < len {
                if i + 1 < len && chars[i] == ']' && chars[i + 1] == ']' {
                    depth -= 2;
                    if depth == 0 {
                        // Found matching ]]
                        let inner: String = chars[content_start..i].iter().collect();
                        let line = text[..link_start].matches('\n').count() + 1;

                        if let Some(parsed) = parse_link_inner(&inner) {
                            links.push(Link {
                                target: parsed.0,
                                display_text: parsed.1,
                                resolved: false,
                                line,
                            });
                        }

                        i += 2; // skip ]]
                        break;
                    }
                    i += 2;
                } else if i + 1 < len && chars[i] == '[' && chars[i + 1] == '[' {
                    depth += 2;
                    i += 2;
                } else {
                    i += 1;
                }
            }
        } else {
            i += 1;
        }
    }

    links
}

/// Parse the inner content of a `[[...]]` link.
///
/// Handles escaped pipes `\|` (not treated as separators).
/// The last `|` separates target from display text.
fn parse_link_inner(inner: &str) -> Option<(String, Option<String>)> {
    // Replace escaped pipes with sentinel
    let sentinel = "\x00PIPE\x00";
    let processed = inner.replace("\\|", sentinel);

    // Find last pipe
    if let Some(pipe_pos) = processed.rfind('|') {
        let target_raw = &processed[..pipe_pos];
        let display_raw = &processed[pipe_pos + 1..];

        let target = target_raw.replace(sentinel, "|").trim().to_string();
        let display = display_raw.replace(sentinel, "|").trim().to_string();

        if target.is_empty() {
            return None;
        }

        Some((target, Some(display)))
    } else {
        // No pipe — entire content is the target
        let target = processed.replace(sentinel, "|").trim().to_string();
        Some((target, None))
    }
}

/// Resolve links against a set of known note slugs.
pub fn resolve_links(links: &mut [Link], known_slugs: &std::collections::HashSet<String>) {
    for link in links.iter_mut() {
        let slug = link.target.replace(' ', "-").to_lowercase();
        link.resolved = known_slugs.contains(&slug) || known_slugs.contains(&link.target);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_link() {
        let body = "See [[Quantum Computing]] for details.";
        let links = extract_links(body);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "Quantum Computing");
        assert!(links[0].display_text.is_none());
        assert_eq!(links[0].line, 1);
    }

    #[test]
    fn test_link_with_display_text() {
        let body = "See [[Note|Display Text]] here.";
        let links = extract_links(body);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "Note");
        assert_eq!(links[0].display_text, Some("Display Text".to_string()));
    }

    #[test]
    fn test_link_multiple_pipes_last_is_display() {
        // Last pipe separates display text (requirement: "last pipe = display")
        let body = "See [[Target|middle|Display]] here.";
        let links = extract_links(body);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "Target|middle");
        assert_eq!(links[0].display_text, Some("Display".to_string()));
    }

    #[test]
    fn test_escaped_pipe() {
        let body = "See [[Target\\|Name|Display]] here.";
        let links = extract_links(body);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "Target|Name");
        assert_eq!(links[0].display_text, Some("Display".to_string()));
    }

    #[test]
    fn test_multiple_links_on_same_line() {
        let body = "[[A]] and [[B]] and [[C]]";
        let links = extract_links(body);
        assert_eq!(links.len(), 3);
        assert_eq!(links[0].target, "A");
        assert_eq!(links[1].target, "B");
        assert_eq!(links[2].target, "C");
    }

    #[test]
    fn test_no_links() {
        let body = "Plain text without any wiki links.";
        let links = extract_links(body);
        assert!(links.is_empty());
    }

    #[test]
    fn test_malformed_unclosed_bracket() {
        let body = "This has [[unclosed bracket";
        let links = extract_links(body);
        assert!(links.is_empty());
    }

    #[test]
    fn test_link_with_line_numbers() {
        let body = "line1\nline2 [[Link]] line3";
        let links = extract_links(body);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].line, 2);
    }

    #[test]
    fn test_nested_brackets() {
        // [[Outer [[Inner]] rest]] should be parsed as one link
        let body = "See [[Outer [[Inner]] rest]] here.";
        let links = extract_links(body);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "Outer [[Inner]] rest");
        assert!(links[0].display_text.is_none());
    }

    #[test]
    fn test_empty_link_content() {
        let body = "Empty [[]] link.";
        let links = extract_links(body);
        // Empty content should not produce a link
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "");
    }

    #[test]
    fn test_resolve_links() {
        let mut links = vec![
            Link {
                target: "Note-A".to_string(),
                display_text: None,
                resolved: false,
                line: 1,
            },
            Link {
                target: "Note-B".to_string(),
                display_text: None,
                resolved: false,
                line: 2,
            },
        ];
        let mut known = std::collections::HashSet::new();
        known.insert("note-a".to_string());

        resolve_links(&mut links, &known);
        assert!(links[0].resolved);
        assert!(!links[1].resolved);
    }

    #[test]
    fn test_display_text_with_pipe() {
        // Escaped pipe in display text
        let body = "See [[Target|Display\\|Text]] here.";
        let links = extract_links(body);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "Target");
        assert_eq!(links[0].display_text, Some("Display|Text".to_string()));
    }
}
