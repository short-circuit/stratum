//! Translation-key contract tests.
//!
//! The editor never carries English literals. [`EDITOR_I18N_KEYS`] is a release
//! gate: every key must exist in the shipped English catalog, and every slash
//! item must reference a member of that list.

use std::collections::HashSet;

use serde_json::Value;
use stratum_editor::menus::slash_items;
use stratum_editor::EDITOR_I18N_KEYS;

const EN_JSON: &str = include_str!("../../stratum-i18n/locales/en.json");

fn flatten(value: &Value, prefix: &str, out: &mut HashSet<String>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let dotted = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                flatten(child, &dotted, out);
            }
        }
        _ => {
            out.insert(prefix.to_string());
        }
    }
}

#[test]
fn editor_keys_exist_in_the_english_catalog() {
    let root: Value = serde_json::from_str(EN_JSON).expect("en.json parses");
    let status = root
        .get("_meta")
        .and_then(|meta| meta.get("status"))
        .and_then(Value::as_str);
    assert_eq!(
        status,
        Some("complete"),
        "en.json must be a complete catalog"
    );

    let mut keys = HashSet::new();
    flatten(&root, "", &mut keys);
    for key in EDITOR_I18N_KEYS {
        assert!(keys.contains(*key), "missing catalog key: {key}");
    }
}

#[test]
fn every_slash_item_key_is_in_the_gate_list() {
    let gate: HashSet<&str> = EDITOR_I18N_KEYS.iter().copied().collect();
    for item in slash_items(true, true) {
        assert!(
            gate.contains(item.title_key),
            "slash title key not gated: {}",
            item.title_key
        );
        if let Some(subtitle) = item.subtitle_key {
            assert!(
                gate.contains(subtitle),
                "slash subtitle key not gated: {subtitle}"
            );
        }
    }
}

#[test]
fn gate_list_has_no_duplicates() {
    let unique: HashSet<&str> = EDITOR_I18N_KEYS.iter().copied().collect();
    assert_eq!(unique.len(), EDITOR_I18N_KEYS.len());
}

#[test]
fn gate_list_only_contains_dotted_keys() {
    for key in EDITOR_I18N_KEYS {
        assert!(key.contains('.'), "key is not dotted: {key}");
        assert!(
            !key.chars().any(char::is_whitespace),
            "key contains whitespace: {key}"
        );
    }
}
