//! GFM pipe-table parsing and canonical serialization for block content.
//!
//! Tables are stored as plain Markdown inside a block's `content`; this module
//! turns that text into [`pkm_block::Table`] and back. The parser is deliberately
//! strict so a paragraph that happens to contain a `|` is never misclassified:
//! every line must be pipe-delimited, there must be at least two lines, and the
//! second line must be a valid delimiter row. That mirrors the editor's
//! `is_table` classification.
//!
//! # Canonical form
//!
//! [`serialize_table`] emits one line per row using the canonical spaced GFM
//! form (`| cell | cell |`, delimiter cells `| --- |`, `| :--- |`, `| :---: |`,
//! `| ---: |`) with no trailing newline. The contract is
//! `serialize_table(parse_table(canonical)) == canonical`: a table that is
//! already in canonical form round-trips byte-for-byte, while non-canonical
//! spacing is normalized on the first pass.
//!
//! # Limitations
//!
//! Escaped pipes (`\|`) are not handled in this version: a cell is produced by a
//! naive `split('|')`, so an escaped pipe is treated as a cell boundary. Cells
//! are also plain text here; inline emphasis inside a cell is not decorated by
//! the editor yet.

use pkm_block::{Table, TableAlign};

/// Parse a GFM pipe table from block content.
///
/// Returns `Some(table)` only when every line starts and ends with `|`, there are
/// at least two lines, the second line is a valid delimiter row, and every line
/// has the same number of cells (at least one). Otherwise returns `None`, which
/// is what keeps ordinary paragraphs containing a pipe out of the table kind.
pub fn parse_table(content: &str) -> Option<Table> {
    let lines: Vec<&str> = content.lines().collect();
    if lines.len() < 2 {
        return None;
    }

    let mut rows: Vec<Vec<String>> = Vec::with_capacity(lines.len());
    for (index, raw) in lines.iter().enumerate() {
        let line = raw.trim();
        if !line.starts_with('|') || !line.ends_with('|') {
            return None;
        }
        // A well-formed pipe row needs a leading and a trailing pipe, so at
        // least two `|` characters.
        if line.matches('|').count() < 2 {
            return None;
        }
        let segments: Vec<&str> = line.split('|').collect();
        // `segments[0]` and the last segment are the empty text outside the
        // leading/trailing pipes; the cells are everything between them.
        if segments.len() < 3 {
            return None;
        }
        let cells: Vec<String> = segments[1..segments.len() - 1]
            .iter()
            .map(|cell| cell.trim().to_string())
            .collect();
        if cells.is_empty() {
            return None;
        }
        if index == 1 && !cells.iter().all(|cell| is_delimiter_cell(cell)) {
            return None;
        }
        rows.push(cells);
    }

    let columns = rows[0].len();
    if columns == 0 || rows.iter().any(|row| row.len() != columns) {
        return None;
    }

    let alignments: Vec<TableAlign> = rows[1]
        .iter()
        .map(|cell| alignment_for(cell))
        .collect::<Option<Vec<_>>>()?;

    Some(Table {
        alignments,
        header: rows[0].clone(),
        rows: rows[2..].to_vec(),
    })
}

/// Serialize a table into canonical spaced GFM pipe-table Markdown.
///
/// The output has no trailing newline so a canonical table round-trips
/// byte-for-byte through [`parse_table`] and back.
pub fn serialize_table(table: &Table) -> String {
    let mut lines = Vec::with_capacity(table.rows.len() + 2);
    lines.push(row_line(&table.header));
    let delimiter: Vec<&str> = table.alignments.iter().map(alignment_marker).collect();
    lines.push(row_line_owned(&delimiter));
    for row in &table.rows {
        lines.push(row_line(row));
    }
    lines.join("\n")
}

/// Whether `content` is a GFM pipe table (thin wrapper over [`parse_table`]).
pub fn is_table(content: &str) -> bool {
    parse_table(content).is_some()
}

/// The empty two-column, one-body-row table inserted by the `table` action.
pub fn table_skeleton() -> String {
    serialize_table(&Table::skeleton(2, 1))
}

/// Whether `cell` is a GFM delimiter cell (`:?-{3,}:?`).
fn is_delimiter_cell(cell: &str) -> bool {
    let trimmed = cell.trim();
    let without_leading = trimmed.strip_prefix(':').unwrap_or(trimmed);
    let body = without_leading.strip_suffix(':').unwrap_or(without_leading);
    body.len() >= 3 && body.chars().all(|c| c == '-')
}

/// Map a delimiter cell onto its alignment variant.
fn alignment_for(cell: &str) -> Option<TableAlign> {
    if !is_delimiter_cell(cell) {
        return None;
    }
    let trimmed = cell.trim();
    let left = trimmed.starts_with(':');
    let right = trimmed.ends_with(':');
    Some(match (left, right) {
        (true, true) => TableAlign::Center,
        (true, false) => TableAlign::Left,
        (false, true) => TableAlign::Right,
        (false, false) => TableAlign::None,
    })
}

/// Canonical delimiter marker for an alignment.
fn alignment_marker(align: &TableAlign) -> &'static str {
    match align {
        TableAlign::None => "---",
        TableAlign::Left => ":---",
        TableAlign::Center => ":---:",
        TableAlign::Right => "---:",
    }
}

/// Canonical line for a row of borrowed cells.
fn row_line_owned(cells: &[&str]) -> String {
    format!("| {} |", cells.join(" | "))
}

/// Canonical line for a row of owned cells.
fn row_line(cells: &[String]) -> String {
    let borrowed: Vec<&str> = cells.iter().map(String::as_str).collect();
    row_line_owned(&borrowed)
}

#[cfg(test)]
mod tests;
