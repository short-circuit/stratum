//! Unit tests for the visual Datalog builder port.
//!
//! Every test exercises behaviour that the shipped TypeScript implementation
//! documents: default emission, each operator family, validation errors,
//! escaping, round-trip parsing and parser rejection rules.

use super::*;
use crate::query::builder::fields::FilterSpec;

fn spec(field: FieldId, op: OpId, value: &str, combiner: Combiner) -> FilterSpec {
    FilterSpec {
        id: "p0".to_string(),
        field,
        op,
        value: value.to_string(),
        combiner,
    }
}

#[test]
fn empty_input_emits_the_default_query() {
    let outcome = build_datalog(&[]);
    assert_eq!(outcome.error, None);
    assert_eq!(
        outcome.datalog,
        "{:query [:find ?b ?content :where [?b :block/marker \"TODO\"] [?b :block/content ?content]]}"
    );
}

#[test]
fn equality_on_content_emits_a_triple() {
    let outcome = build_datalog(&[spec(FieldId::Content, OpId::Eq, "hello", Combiner::And)]);
    assert_eq!(outcome.error, None);
    assert_eq!(
        outcome.datalog,
        "{:query [:find ?b ?content :where [?b :block/content ?content] [?b :block/content \"hello\"]]}"
    );
}

#[test]
fn contains_emits_a_bind_triple_and_predicate() {
    let outcome = build_datalog(&[spec(
        FieldId::Content,
        OpId::Contains,
        "note",
        Combiner::And,
    )]);
    assert_eq!(outcome.error, None);
    assert_eq!(
        outcome.datalog,
        "{:query [:find ?b ?content :where [?b :block/content ?content] [?b :block/content ?v0] [(clojure.string/includes? ?v0 \"note\")]]}"
    );
}

#[test]
fn not_has_emits_a_top_level_negation() {
    let outcome = build_datalog(&[spec(FieldId::Tags, OpId::NotHas, "x", Combiner::And)]);
    assert_eq!(outcome.error, None);
    assert_eq!(
        outcome.datalog,
        "{:query [:find ?b ?content :where [?b :block/content ?content] (not [?b :block/tags \"x\"])]}"
    );
}

#[test]
fn two_or_rows_form_an_or_group() {
    let specs = vec![
        spec(FieldId::Content, OpId::Eq, "a", Combiner::And),
        spec(FieldId::Content, OpId::Eq, "b", Combiner::Or),
    ];
    let outcome = build_datalog(&specs);
    assert_eq!(outcome.error, None);
    assert_eq!(
        outcome.datalog,
        "{:query [:find ?b ?content :where [?b :block/content ?content] (or [?b :block/content \"a\"] [?b :block/content \"b\"])]}"
    );
}

#[test]
fn lone_or_row_is_rejected() {
    let outcome = build_datalog(&[spec(FieldId::Content, OpId::Eq, "a", Combiner::Or)]);
    assert_eq!(outcome.error, Some(BuildError::OrNeedsTwo));
    assert_eq!(outcome.datalog, "");
    assert_eq!(outcome.error.unwrap().key(), "query.builder_or_needs_two");
}

#[test]
fn value_needing_operator_rejects_an_empty_value() {
    let outcome = build_datalog(&[spec(FieldId::Content, OpId::Contains, "   ", Combiner::And)]);
    assert_eq!(outcome.error, Some(BuildError::RequiresValue));
    assert_eq!(outcome.error.unwrap().key(), "query.builder_requires_value");
}

#[test]
fn page_field_injects_the_page_binding() {
    let outcome = build_datalog(&[spec(FieldId::PageTitle, OpId::Eq, "Home", Combiner::And)]);
    assert_eq!(outcome.error, None);
    assert_eq!(
        outcome.datalog,
        "{:query [:find ?b ?content :where [?b :block/content ?content] [?b :block/page ?p] [?p :page/title \"Home\"]]}"
    );
}

#[test]
fn edn_escaping_covers_backslashes_and_quotes() {
    let outcome = build_datalog(&[spec(FieldId::Content, OpId::Eq, "a\\b\"c", Combiner::And)]);
    let clause = r#"[?b :block/content "a\\b\"c"]"#;
    let expected =
        format!("{{:query [:find ?b ?content :where [?b :block/content ?content] {clause}]}}");
    assert_eq!(outcome.error, None);
    assert_eq!(outcome.datalog, expected);
}

#[test]
fn present_and_absent_emit_the_predicate_form() {
    let present = build_datalog(&[spec(FieldId::Content, OpId::Present, "", Combiner::And)]);
    assert_eq!(present.error, None);
    assert_eq!(
        present.datalog,
        "{:query [:find ?b ?content :where [?b :block/content ?content] [?b :block/content ?v0] [(present? ?v0)]]}"
    );

    let absent = build_datalog(&[spec(FieldId::Content, OpId::Absent, "", Combiner::And)]);
    assert_eq!(absent.error, None);
    assert_eq!(
        absent.datalog,
        "{:query [:find ?b ?content :where [?b :block/content ?content] [?b :block/content ?v0] [(absent? ?v0)]]}"
    );
}

#[test]
fn multi_row_build_parse_build_round_trips() {
    let specs = vec![
        spec(FieldId::Content, OpId::Contains, "alpha", Combiner::And),
        spec(FieldId::Marker, OpId::Eq, "TODO", Combiner::And),
        spec(FieldId::PageTitle, OpId::StartsWith, "Proj", Combiner::And),
        spec(FieldId::Tags, OpId::Has, "project", Combiner::And),
    ];
    let first = build_datalog(&specs);
    assert_eq!(first.error, None);
    let parsed = parse_datalog_to_rows(&first.datalog).expect("round-trip parse");
    assert_eq!(parsed.len(), 4);
    let second = build_datalog(&parsed);
    assert_eq!(second.error, None);
    assert_eq!(second.datalog, first.datalog);
}

#[test]
fn parser_rejects_pull_find_specs() {
    let datalog = "{:query [:find (pull ?b [*]) :where [?b :block/content ?content]]}";
    assert_eq!(parse_datalog_to_rows(datalog), None);
}

#[test]
fn parser_rejects_json_syntax() {
    let datalog = "{\"query\": [:find ?b :where [?b :block/content \"x\"]]}";
    assert_eq!(parse_datalog_to_rows(datalog), None);
}

#[test]
fn parser_rejects_unknown_attributes() {
    let datalog = "{:query [:find ?b :where [?b :block/unknown \"x\"]]}";
    assert_eq!(parse_datalog_to_rows(datalog), None);
}

#[test]
fn parser_rejects_malformed_input() {
    assert_eq!(parse_datalog_to_rows(""), None);
    assert_eq!(
        parse_datalog_to_rows("{:query [:find ?b :where [?b]]}"),
        None
    );
    assert_eq!(
        parse_datalog_to_rows("{:query [:find ?b]}"),
        Some(Vec::new())
    );
}
