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
    let pairs: Vec<(usize, char)> = text.char_indices().collect();
    let len = pairs.len();
    let mut i = 0;

    while i < len {
        if i + 1 < len && pairs[i].1 == '[' && pairs[i + 1].1 == '[' {
            let link_start_byte = pairs[i].0;
            i += 2;
            let mut depth: i32 = 2;

            let content_start = i;

            while i < len {
                if i + 1 < len && pairs[i].1 == ']' && pairs[i + 1].1 == ']' {
                    depth -= 2;
                    if depth == 0 {
                        let inner: String =
                            pairs[content_start..i].iter().map(|(_, c)| c).collect();
                        let line = text[..link_start_byte].matches('\n').count() + 1;

                        if let Some(parsed) = parse_link_inner(&inner) {
                            let (target, display_text) =
                                unwrap_bracket_target(&parsed.0, parsed.1.as_deref());
                            links.push(Link {
                                target,
                                display_text,
                                resolved: false,
                                line,
                            });
                        }

                        i += 2;
                        break;
                    }
                    i += 2;
                } else if i + 1 < len && pairs[i].1 == '[' && pairs[i + 1].1 == '[' {
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

/// If `s` is exactly `[[<content>]]` (one full wrapping), return `<content>`.
fn fully_wrapped(s: &str) -> Option<&str> {
    if s.starts_with("[[") && s.ends_with("]]") && s.len() >= 4 {
        Some(&s[2..s.len() - 2])
    } else {
        None
    }
}

/// Collapse a bracket-wrapped wiki-link target to its bare inner target.
///
/// Corrupted/legacy content can leak `[[...]]` wrapping into a link target
/// (e.g. `[[[[Docker]]]]` is scanned as target `[[Docker]]`, and the piped
/// double-wrap `[[[[Alpha Project|display]]|display]]` as target
/// `[[Alpha Project|display]]`). This normalizes a parsed target so graph
/// resolution (`resolve_slug`) can match it, returning the normalized target
/// plus its (possibly re-derived) display text.
///
/// The unwrap is structural, not a naive char strip of every `[`/`]`: it only
/// collapses a target that is EXACTLY one full `[[...]]` wrapping whose middle
/// is itself a single parseable link (no stray leading/trailing text). This leaves
/// legitimately nested targets like `[[Outer [[Inner]] rest]]` untouched. The
/// piped double-wrap is only collapsed when the outer and inner displays match
/// (the duplicated-display corruption); a mismatched display (`[[[[A|b]]|c]]`)
/// keeps the wrapped target as-is, mirroring the frontend `normalizeContent`.
fn unwrap_bracket_target(target: &str, display: Option<&str>) -> (String, Option<String>) {
    let mut cur = target.to_string();
    let mut disp = display.map(|d| d.to_string());
    let mut mismatch = false;

    while let Some(middle) = fully_wrapped(&cur) {
        // A legitimately nested target (`[[Outer [[Inner]] rest]]`, scanned as
        // `Outer [[Inner]] rest`) is not a bare wrap — leave it untouched.
        if middle.contains("[[") {
            break;
        }
        // The middle must itself be parseable link content; an empty/whitespace
        // middle parses to `None`, so keep the current target.
        let Some((inner_target, inner_display)) = parse_link_inner(middle) else {
            break;
        };
        // Piped double-wrap `[[[[A|b]]|c]]`: outer and inner displays disagree, so
        // keep the wrapped target as-is (`[[A|b]]` raw). The frontend leaves this
        // `full` match unchanged too.
        if let (Some(d), Some(id)) = (disp.as_deref(), inner_display.as_deref()) {
            if d != id {
                mismatch = true;
                break;
            }
        }
        // Collapse one layer: adopt the inner target and display, then re-check —
        // the inner target may itself be one more wrap deep (e.g. `[[[[Docker]]]]`).
        cur = inner_target;
        disp = inner_display;
    }

    // Defensive edge-strip for odd bracket counts (e.g. `[[[Docker]]]` scans as
    // target `[Docker]`, which is not a full `[[...]]` wrap). Skipped when we kept
    // a wrapped target for a mismatched display, which must stay wrapped.
    if !mismatch {
        while cur.starts_with('[') {
            cur = cur[1..].to_string();
        }
        while cur.ends_with(']') {
            cur = cur[..cur.len() - 1].to_string();
        }
    }

    (cur, disp)
}

/// Resolve links against a set of known note slugs.
pub fn resolve_links(links: &mut [Link], known_slugs: &std::collections::HashSet<String>) {
    for link in links.iter_mut() {
        let slug = link.target.replace(' ', "-").to_lowercase();
        link.resolved = known_slugs.contains(&slug) || known_slugs.contains(&link.target);
    }
}

/// Whether a `[[...]]` target points at an asset rather than a note.
///
/// Asset targets begin with `assets/` (case-insensitive), matching both the inline
/// embed form `![[assets/...]]` and the file-link form `[[assets/...]]`.
pub fn is_asset_link_target(target: &str) -> bool {
    target.trim().to_lowercase().starts_with("assets/")
}

/// Extract the targets of every asset link (`![[assets/...]]` or `[[assets/...]]`) in text.
///
/// Reuses the existing [`extract_links`] scanner to find all `[[...]]` occurrences, then
/// keeps only those whose target starts with `assets/`. Plain `[[Note]]` links are
/// ignored. The leading `!` of an embed is outside the `[[` so it does not affect
/// extraction; both forms are captured identically.
pub fn extract_asset_links(text: &str) -> Vec<String> {
    extract_links(text)
        .into_iter()
        .filter(|l| is_asset_link_target(&l.target))
        .map(|l| l.target)
        .collect()
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

    #[test]
    fn test_is_asset_link_target() {
        assert!(is_asset_link_target("assets/foo.png"));
        assert!(is_asset_link_target("Assets/Foo.PNG"));
        assert!(!is_asset_link_target("pages/foo.png"));
        assert!(!is_asset_link_target("Note"));
        assert!(!is_asset_link_target("asset"));
    }

    #[test]
    fn test_extract_asset_links_embed() {
        let links = extract_asset_links("![[assets/foo.png]]");
        assert_eq!(links, vec!["assets/foo.png".to_string()]);
    }

    #[test]
    fn test_extract_asset_links_plain_link() {
        let links = extract_asset_links("[[assets/bar.pdf]]");
        assert_eq!(links, vec!["assets/bar.pdf".to_string()]);
    }

    #[test]
    fn test_extract_asset_links_mixed_ignores_notes() {
        let body = "See [[Note]] and ![[assets/foo.png]] and [[assets/x.pdf]]";
        let links = extract_asset_links(body);
        assert_eq!(
            links,
            vec!["assets/foo.png".to_string(), "assets/x.pdf".to_string()]
        );
    }

    #[test]
    fn test_extract_asset_links_no_assets() {
        let links = extract_asset_links("Just [[Note]] here");
        assert!(links.is_empty());
    }

    #[test]
    fn test_strip_bracket_wrapped_target_double() {
        let links = extract_links("See [[[[Docker]]]] for docs");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "Docker");
        assert!(links[0].display_text.is_none());
    }

    #[test]
    fn test_strip_bracket_wrapped_target_triple() {
        let links = extract_links("[[[Docker]]]");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "Docker");
        assert!(links[0].display_text.is_none());
    }

    #[test]
    fn test_strip_bracket_wrapped_target_piped_double_wrap() {
        let links = extract_links("[[[[Alpha Project|display]]|display]]");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "Alpha Project");
        assert_eq!(links[0].display_text.as_deref(), Some("display"));
    }
}
