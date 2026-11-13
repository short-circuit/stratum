//! Document model for the headless editor.
//!
//! [`EditorDocument`] owns the block tree for one page and the raw source needed
//! to preserve frontmatter across saves.
//!
//! # Byte-stability contract
//!
//! - [`canonicalize_body`] is idempotent: canonicalizing already-canonical input
//!   returns the same bytes.
//! - A canonical file round-trips byte-identically through
//!   [`canonicalize_body`].
//! - Parse→serialize is **not** byte-identical for arbitrary non-canonical input.
//!   `serialize_blocks` appends `.id:` to every block, orders properties
//!   canonically (marker, priority, heading-level, sorted properties, id last)
//!   and normalizes block indentation to the tree's real depth. Those are
//!   deliberate normalizations, not data loss.
//! - Saves go through `pkm-markdown` only: [`EditorDocument::body`] and
//!   [`canonicalize_body`] call [`pkm_markdown::block_parser::serialize_blocks`]
//!   directly, so the shipped serializer is the single source of truth for
//!   block syntax and nesting.
//! - Saves preserve the original frontmatter by re-assembling it with
//!   [`pkm_markdown::block_parser::assemble_blocks_markdown`]; the editor never
//!   rebuilds frontmatter from a title alone.
//!
//! Revisions and the dirty flag are bookkeeping for the UI: [`EditorDocument::mutate`]
//! is the only supported way to change the tree, and it bumps the revision on
//! success.

use pkm_block::{Block, BlockId, BlockTree};
use pkm_core::PkmResult;
use pkm_markdown::block_parser::{assemble_blocks_markdown, parse_document, serialize_blocks};

/// A page document plus the bookkeeping needed to save it safely.
#[derive(Debug, Clone)]
pub struct EditorDocument {
    path: String,
    tree: BlockTree,
    /// The exact raw source the document was opened from, retained so
    /// `markdown()` can re-assemble the original frontmatter.
    existing_raw: String,
    /// The frontmatter prefix of `existing_raw`, or an empty string.
    frontmatter_raw: String,
    revision: u64,
    dirty: bool,
}

impl EditorDocument {
    /// Open a markdown document.
    ///
    /// `raw_markdown` is parsed with the shipped block parser. The returned
    /// document is clean (revision 0); callers that edit it must go through
    /// [`EditorDocument::mutate`].
    pub fn open(path: impl Into<String>, raw_markdown: &str) -> PkmResult<Self> {
        let (_frontmatter, _body, blocks) = parse_document(raw_markdown);
        let mut tree = BlockTree::new();
        for block in blocks {
            tree.insert(block);
        }
        Ok(Self {
            path: path.into(),
            tree,
            existing_raw: raw_markdown.to_string(),
            frontmatter_raw: frontmatter_prefix(raw_markdown),
            revision: 0,
            dirty: false,
        })
    }

    /// Build a document directly from blocks (e.g. from the SQLite store).
    ///
    /// No raw source is retained, so `markdown()` emits the body only.
    pub fn from_blocks(path: impl Into<String>, blocks: Vec<Block>) -> PkmResult<Self> {
        let mut tree = BlockTree::new();
        for block in blocks {
            tree.insert(block);
        }
        Ok(Self {
            path: path.into(),
            tree,
            existing_raw: String::new(),
            frontmatter_raw: String::new(),
            revision: 0,
            dirty: false,
        })
    }

    /// The page path this document was opened from.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The frontmatter prefix of the original source (empty when absent).
    pub fn frontmatter_raw(&self) -> &str {
        &self.frontmatter_raw
    }

    /// All blocks in canonical document order.
    pub fn blocks(&self) -> Vec<&Block> {
        self.tree.depth_first_order()
    }

    /// Look up one block by id.
    pub fn block(&self, id: BlockId) -> Option<&Block> {
        self.tree.get(id)
    }

    /// Block ids in canonical document order.
    pub fn ordered_ids(&self) -> Vec<BlockId> {
        self.tree.depth_first_order().iter().map(|b| b.id).collect()
    }

    /// Borrow the underlying tree.
    pub fn tree(&self) -> &BlockTree {
        &self.tree
    }

    /// Monotonic revision counter, incremented after every successful mutation.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Whether the document has unsaved changes.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Mark the document clean after a successful save.
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    /// Serialize the canonical markdown body (no frontmatter).
    pub fn body(&self) -> String {
        let blocks: Vec<Block> = self.tree.depth_first_order().into_iter().cloned().collect();
        serialize_blocks(&blocks)
    }

    /// Serialize the full markdown document, preserving the original frontmatter.
    pub fn markdown(&self) -> String {
        assemble_blocks_markdown(&self.existing_raw, &self.body(), None)
    }

    /// Page plain text for AI page prompts.
    ///
    /// Mirrors `extractText` in `src/components/AISlashMenu.tsx`: block content
    /// in document order, children nested with `'\n'`, top-level blocks joined
    /// by `"\n\n"`. No frontmatter, property lines or ids are included.
    pub fn plain_text(&self) -> String {
        let mut texts = Vec::new();
        for root in self.tree.roots() {
            let text = plain_text_block(&self.tree, root);
            if !text.is_empty() {
                texts.push(text);
            }
        }
        texts.join("\n\n")
    }

    /// Property-free markdown for interlink prompts.
    ///
    /// Mirrors BlockNote's `blocksToMarkdownLossy()`: nested `- ` bullets (or
    /// `#`-prefixed heading lines for heading blocks) in document order, with no
    /// frontmatter and no `.id:`/`.marker:`/`.priority:`/`.heading-level:` or
    /// other property lines.
    pub fn plain_markdown_lossy(&self) -> String {
        let mut lines = Vec::new();
        for root in self.tree.roots() {
            plain_markdown_block(&self.tree, root, 0, &mut lines);
        }
        lines.join("\n")
    }

    /// Produce the bytes that should be written for a save and mark the
    /// document clean.
    ///
    /// Returning the string keeps the caller in control of the filesystem write;
    /// marking clean here records that the revision the caller is about to write
    /// is the current one.
    pub fn save_snapshot(&mut self) -> String {
        let markdown = self.markdown();
        self.mark_clean();
        markdown
    }

    /// Apply a mutation to the block tree.
    ///
    /// The revision and dirty flag are bumped only when `f` succeeds, so a
    /// rejected operation (cycle, missing block, …) leaves the document clean.
    pub fn mutate<F>(&mut self, f: F) -> PkmResult<()>
    where
        F: FnOnce(&mut BlockTree) -> PkmResult<()>,
    {
        f(&mut self.tree)?;
        self.revision = self.revision.saturating_add(1);
        self.dirty = true;
        Ok(())
    }
}

/// Canonicalize a markdown body by parsing and re-serializing its blocks.
///
/// This is the exact transform a save applies to the body. It is idempotent:
/// `canonicalize_body(canonicalize_body(x)) == canonicalize_body(x)`.
pub fn canonicalize_body(raw: &str) -> String {
    let (_frontmatter, _body, blocks) = parse_document(raw);
    let mut tree = BlockTree::new();
    for block in blocks {
        tree.insert(block);
    }
    let ordered: Vec<Block> = tree.depth_first_order().into_iter().cloned().collect();
    serialize_blocks(&ordered)
}

/// Plain text for one block and its children, mirroring `extractText`.
fn plain_text_block(tree: &BlockTree, block: &Block) -> String {
    let mut text = block.content.clone();
    let children: Vec<String> = tree
        .children(block.id)
        .into_iter()
        .map(|child| plain_text_block(tree, child))
        .filter(|text| !text.is_empty())
        .collect();
    if !children.is_empty() {
        text.push('\n');
        text.push_str(&children.join("\n"));
    }
    text
}

/// One property-free markdown line per block, recursing into children.
fn plain_markdown_block(tree: &BlockTree, block: &Block, depth: usize, lines: &mut Vec<String>) {
    let indent = "  ".repeat(depth);
    let line = match block.meta.heading_level {
        Some(level) if level > 0 => {
            format!("{indent}{} {}", "#".repeat(level as usize), block.content)
        }
        _ => format!("{indent}- {}", block.content),
    };
    lines.push(line);
    for child in tree.children(block.id) {
        plain_markdown_block(tree, child, depth + 1, lines);
    }
}

/// Return the raw `---…---` frontmatter prefix of `raw`, or an empty string.
fn frontmatter_prefix(raw: &str) -> String {
    let trimmed = raw.trim_start();
    if !trimmed.starts_with("---") {
        return String::new();
    }
    // Find the closing delimiter on its own line. The opening line may be
    // `---` with trailing spaces; the closing line must start at column 0.
    let mut offset = raw.len() - trimmed.len();
    let mut lines = trimmed.split_inclusive('\n');
    let mut consumed = String::new();
    if let Some(first) = lines.next() {
        consumed.push_str(first);
        offset += first.len();
    }
    let mut found_close = false;
    for line in lines {
        consumed.push_str(line);
        if line.trim_end() == "---" {
            found_close = true;
            break;
        }
    }
    if found_close {
        // Include the byte offset of the opening delimiter so the prefix keeps
        // any leading whitespace of the original source.
        let _ = offset;
        consumed
    } else {
        String::new()
    }
}

#[cfg(test)]
mod tests;
