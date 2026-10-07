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
    assert_eq!(query.r#where[0].attribute, ":block/marker");
    assert_eq!(query.r#where[0].value, "TODO");
}

#[test]
fn test_parse_simple_query_json() {
    let input = r#"{"query": [":find", ["?b"], ":where", ["?b", ":block/marker", "TODO"]]}"#;
    let query = parse_query(input).unwrap();
    assert_eq!(query.r#where.len(), 1);
    assert_eq!(query.r#where[0].attribute, ":block/marker");
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
}

#[test]
fn test_parse_with_placeholder() {
    let input = r#"{:query [:find ?title :where [?p :page/title ?title] [_ :block/page ?p]]}"#;
    let query = parse_query(input).unwrap();
    assert_eq!(query.r#where.len(), 2);
    assert_eq!(query.r#where[1].entity, "_");
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
