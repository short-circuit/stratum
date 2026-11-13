//! Unit tests for GFM pipe-table parsing and canonical serialization.

use pkm_block::TableAlign;

use super::{is_table, parse_table, serialize_table, table_skeleton};

#[test]
fn canonical_table_round_trips_byte_identically() {
    let canonical = "| Task | Owner |\n| --- | --- |\n| ship | anon |";
    let table = parse_table(canonical).expect("canonical table parses");
    assert_eq!(table.column_count(), 2);
    assert_eq!(table.row_count(), 1);
    assert_eq!(serialize_table(&table), canonical);
}

#[test]
fn alignment_row_parses_to_the_right_alignments() {
    let input = "| A | B | C | D |\n| :--- | :---: | ---: | --- |\n| 1 | 2 | 3 | 4 |";
    let table = parse_table(input).expect("aligned table parses");
    assert_eq!(
        table.alignments,
        vec![
            TableAlign::Left,
            TableAlign::Center,
            TableAlign::Right,
            TableAlign::None,
        ]
    );
    let output = serialize_table(&table);
    assert!(
        output.contains("| :--- | :---: | ---: | --- |"),
        "colons must survive serialization:\n{output}"
    );
}

#[test]
fn non_canonical_spacing_normalizes_to_the_canonical_form() {
    // Pipe-dense input without spaces around cells or the delimiter row.
    let input = "|a|b|\n|---|---|\n|c|d|";
    let canonical = "| a | b |\n| --- | --- |\n| c | d |";
    let table = parse_table(input).expect("non-canonical table parses");
    assert_eq!(serialize_table(&table), canonical);
}

#[test]
fn a_paragraph_containing_a_pipe_is_not_a_table() {
    assert_eq!(parse_table("a | b"), None);
    assert!(!is_table("a | b"));
}

#[test]
fn a_single_pipe_line_is_not_a_table() {
    assert_eq!(parse_table("|"), None);
    assert!(!is_table("|"));
}

#[test]
fn a_mismatched_delimiter_column_count_is_rejected() {
    assert_eq!(parse_table("| a | b |\n| --- |"), None);
}

#[test]
fn a_missing_delimiter_row_is_rejected() {
    assert_eq!(parse_table("| a | b |\n| c | d |"), None);
}

#[test]
fn skeleton_parses_back_to_two_columns_and_one_body_row() {
    let skeleton = table_skeleton();
    let table = parse_table(&skeleton).expect("skeleton parses back");
    assert_eq!(table.column_count(), 2);
    assert_eq!(table.row_count(), 1);
}
