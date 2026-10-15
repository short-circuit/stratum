//! Tests for the Datalog query parser.

use super::*;

#[test]
fn test_parse_simple_query_edn() {
    let input = r#"{:query [:find ?b :where [?b :block/marker "TODO"]]}"#;
    let query = parse_query(input).unwrap();
    match &query.find {
        FindSpec::Vars(v) => assert!(v.contains(&"?b".to_string())),
        _ => panic!("Expected Vars"),
    }
    assert_eq!(query.r#where.len(), 1);
    let triple = query.r#where[0].as_triple().expect("expected triple");
    assert_eq!(triple.attribute, ":block/marker");
    assert_eq!(triple.value, "TODO");
}

#[test]
fn test_parse_simple_query_json() {
    let input = r#"{"query": [":find", ["?b"], ":where", ["?b", ":block/marker", "TODO"]]}"#;
    let query = parse_query(input).unwrap();
    assert_eq!(query.r#where.len(), 1);
    let triple = query.r#where[0].as_triple().expect("expected triple");
    assert_eq!(triple.attribute, ":block/marker");
}

#[test]
fn test_parse_multi_pattern_edn() {
    let input = r#"{:query [:find ?b ?content :where [?b :block/marker "TODO"] [?b :block/content ?content]]}"#;
    let query = parse_query(input).unwrap();
    match &query.find {
        FindSpec::Vars(v) => assert_eq!(v.len(), 2),
        _ => panic!("Expected Vars"),
    }
    assert_eq!(query.r#where.len(), 2);
    assert!(query.r#where.iter().all(|c| c.as_triple().is_some()));
}

#[test]
fn test_parse_with_placeholder() {
    let input = r#"{:query [:find ?title :where [?p :page/title ?title] [_ :block/page ?p]]}"#;
    let query = parse_query(input).unwrap();
    assert_eq!(query.r#where.len(), 2);
    let triple = query.r#where[1].as_triple().expect("expected triple");
    assert_eq!(triple.entity, "_");
}

#[test]
fn test_parse_invalid() {
    assert!(parse_query("not datalog").is_err());
}

#[test]
fn test_parse_pull_edn() {
    let input = r#"{:query [:find (pull ?b [:block/content :block/marker]) :where [?b :block/marker "TODO"]]}"#;
    let query = parse_query(input).unwrap();
    match &query.find {
        FindSpec::Pull { var, attrs } => {
            assert_eq!(var, "?b");
            assert!(attrs.contains(&":block/content".to_string()));
            assert!(attrs.contains(&":block/marker".to_string()));
        }
        _ => panic!("Expected Pull"),
    }
}

#[test]
fn test_parse_predicate_edn() {
    let input = r#"{:query [:find ?b :where [?b :block/marker ?m] [(not= ?m "DONE")]]}"#;
    let query = parse_query(input).unwrap();
    assert_eq!(query.r#where.len(), 2);
    match &query.r#where[1] {
        WhereClause::Pred(p) => {
            assert_eq!(p.op, "not=");
            assert_eq!(p.args, vec!["?m", "DONE"]);
        }
        _ => panic!("Expected predicate"),
    }
}

#[test]
fn test_parse_predicate_symbols_edn() {
    let input = r#"{:query [:find ?b :where [?b :block/content ?c] [(clojure.string/includes? ?c "foo")]]}"#;
    let query = parse_query(input).unwrap();
    match &query.r#where[1] {
        WhereClause::Pred(p) => {
            assert_eq!(p.op, "clojure.string/includes?");
            assert_eq!(p.args, vec!["?c", "foo"]);
        }
        _ => panic!("Expected predicate"),
    }
}

#[test]
fn test_parse_or_group_edn() {
    let input =
        r#"{:query [:find ?b :where (or [?b :block/marker "TODO"] [?b :block/marker "DOING"])]}"#;
    let query = parse_query(input).unwrap();
    assert_eq!(query.r#where.len(), 1);
    match &query.r#where[0] {
        WhereClause::Or(clauses) => {
            assert_eq!(clauses.len(), 2);
            assert!(clauses.iter().all(|c| c.as_triple().is_some()));
        }
        _ => panic!("Expected or-group"),
    }
}

#[test]
fn test_parse_not_group_edn() {
    let input = r#"{:query [:find ?b :where (not [?b :block/tags "project"])]}"#;
    let query = parse_query(input).unwrap();
    assert_eq!(query.r#where.len(), 1);
    match &query.r#where[0] {
        WhereClause::Not(inner) => {
            assert!(inner.as_triple().is_some());
        }
        _ => panic!("Expected or-group"),
    }
}

#[test]
fn test_parse_order_by_edn() {
    let input = r#"{:query [:find ?page ?title ?modified :where [?page :page/title ?title] [?page :page/modified ?modified] :order-by [[?modified :desc]]]}"#;
    let query = parse_query(input).unwrap();
    assert_eq!(query.order_by.len(), 1);
    assert_eq!(query.order_by[0].var, "?modified");
    assert!(query.order_by[0].desc);
}

#[test]
fn test_parse_predicate_json() {
    let input = r#"{"query": [":find", ["?b"], ":where", ["?b", ":block/marker", "?m"], [["not=", "?m", "DONE"]]]}"#;
    let query = parse_query(input).unwrap();
    assert_eq!(query.r#where.len(), 2);
    match &query.r#where[1] {
        WhereClause::Pred(p) => {
            assert_eq!(p.op, "not=");
            assert_eq!(p.args, vec!["?m", "DONE"]);
        }
        _ => panic!("Expected predicate"),
    }
}

#[test]
fn test_parse_or_group_json() {
    let input = r#"{"query": [":find", ["?b"], ":where", ["or", ["?b", ":block/marker", "TODO"], ["?b", ":block/marker", "DOING"]]]}"#;
    let query = parse_query(input).unwrap();
    assert_eq!(query.r#where.len(), 1);
    match &query.r#where[0] {
        WhereClause::Or(clauses) => assert_eq!(clauses.len(), 2),
        _ => panic!("Expected or-group"),
    }
}

#[test]
fn test_parse_order_by_json() {
    let input = r#"{"query": [":find", ["?b"], ":where", ["?b", ":block/marker", "TODO"], ":order-by", [["?b", ":desc"]]]}"#;
    let query = parse_query(input).unwrap();
    assert_eq!(query.order_by.len(), 1);
    assert!(query.order_by[0].desc);
}
