/// Detects if the given content is a fenced code block and returns the language and code.
///
/// Recognizes both ` ``` ` and `~~~` fences with optional language identifier.
/// Returns `Some((language, code))` where language is `""` if not specified.
pub fn detect_fenced_code_block(content: &str) -> Option<(&str, &str)> {
    let trimmed = content.trim();

    // Match ```lang or ```
    if let Some(after_open) = trimmed.strip_prefix("```") {
        let closing = after_open.rfind("```")?;
        let first_line_end = after_open.find('\n').unwrap_or(after_open.len());
        let language = after_open[..first_line_end].trim();
        let code_start = first_line_end + 1;
        if code_start > closing {
            return None;
        }
        let code = after_open[code_start..closing].trim_end();
        return Some((language, code));
    }

    // Match ~~~lang or ~~~
    if let Some(after_open) = trimmed.strip_prefix("~~~") {
        let closing = after_open.rfind("~~~")?;
        let first_line_end = after_open.find('\n').unwrap_or(after_open.len());
        let language = after_open[..first_line_end].trim();
        let code_start = first_line_end + 1;
        if code_start > closing {
            return None;
        }
        let code = after_open[code_start..closing].trim_end();
        return Some((language, code));
    }

    None
}

/// Returns true if the content is a fenced code block with the mermaid language.
pub fn is_mermaid_block(content: &str) -> bool {
    detect_fenced_code_block(content).is_some_and(|(lang, _)| lang == "mermaid")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_mermaid_block() {
        let content = "```mermaid\ngraph TD;\n  A-->B;\n```";
        let result = detect_fenced_code_block(content);
        assert_eq!(result, Some(("mermaid", "graph TD;\n  A-->B;")));
    }

    #[test]
    fn test_detect_plain_code_block() {
        let content = "```\nsome code\n```";
        let result = detect_fenced_code_block(content);
        assert_eq!(result, Some(("", "some code")));
    }

    #[test]
    fn test_detect_tilde_fence() {
        let content = "~~~mermaid\ngraph TD;\n  A-->B;\n~~~";
        let result = detect_fenced_code_block(content);
        assert_eq!(result, Some(("mermaid", "graph TD;\n  A-->B;")));
    }

    #[test]
    fn test_is_mermaid_block() {
        assert!(is_mermaid_block("```mermaid\ngraph TD;\n```"));
        assert!(!is_mermaid_block("```rust\nfn main() {}\n```"));
        assert!(!is_mermaid_block("just some text"));
    }

    #[test]
    fn test_not_a_code_block() {
        assert_eq!(detect_fenced_code_block("just some text"), None);
        assert_eq!(detect_fenced_code_block(""), None);
    }

    #[test]
    fn test_mermaid_without_newline() {
        let content = "```mermaid\ngraph TD;\n  A-->B;\n```";
        assert!(is_mermaid_block(content));
    }
}
