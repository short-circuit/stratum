//! Generic Markdown folder importer.
//!
//! A generic vault is imported verbatim: every `.md` file is copied byte-for-byte
//! with no transformation (no frontmatter re-serialization, no wiki-link or
//! marker rewriting). This is the safe fallback for any folder that is not a
//! recognized Obsidian/Logseq vault.

/// Return the source content unchanged — the generic importer performs no
/// transformation at all.
pub fn convert(source_content: &str) -> String {
    source_content.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generic_is_byte_identical() {
        let content = "Plain note.\n\nWith a paragraph.\n";
        assert_eq!(convert(content), content);
    }
}
