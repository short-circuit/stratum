//! Structural model for GFM pipe tables carried inside a block's `content`.
//!
//! A block stores a table as plain Markdown text in [`crate::Block::content`];
//! this module only provides the plain value model, not a parser or serializer.
//! Parsing and serialization live in `pkm_markdown` (`parse_table` /
//! `serialize_table`) so the on-disk representation stays plain Markdown and the
//! block model never depends on a Markdown crate.
//!
//! The model mirrors the GFM pipe-table shape: a header row, an optional body and
//! one alignment value per column. [`Table::skeleton`] builds an empty table used
//! by the editor's `table` slash action.

/// Per-column alignment of a GFM pipe table.
///
/// `None` means the delimiter cell is a bare `---` (no alignment marker);
/// the other variants correspond to `:---`, `:---:` and `---:`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableAlign {
    /// No alignment marker (`---`).
    None,
    /// Left-aligned column (`:---`).
    Left,
    /// Centre-aligned column (`:---:`).
    Center,
    /// Right-aligned column (`---:`).
    Right,
}

/// A plain GFM pipe table: per-column alignments, a header and body rows.
///
/// Every vector is expected to have one entry per column; callers construct the
/// table through [`Table::skeleton`] or the `pkm_markdown` parser, both of which
/// keep the shape consistent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    /// Alignment marker for each column.
    pub alignments: Vec<TableAlign>,
    /// Header cell texts.
    pub header: Vec<String>,
    /// Body rows; each row has one cell per column.
    pub rows: Vec<Vec<String>>,
}

impl Table {
    /// Build an empty table with `columns` columns and `body_rows` body rows.
    ///
    /// Every cell is empty and every alignment is [`TableAlign::None`].
    pub fn skeleton(columns: usize, body_rows: usize) -> Self {
        Self {
            alignments: vec![TableAlign::None; columns],
            header: vec![String::new(); columns],
            rows: vec![vec![String::new(); columns]; body_rows],
        }
    }

    /// Number of columns in the header row.
    pub fn column_count(&self) -> usize {
        self.header.len()
    }

    /// Number of body rows.
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }
}
