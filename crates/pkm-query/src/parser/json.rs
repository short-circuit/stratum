//! JSON-format query parser.
//!
//! Parses JSON-formatted Datalog queries into [`super::Query`] values.

use serde_json::Value as JsonValue;

use super::{FindSpec, ParseError, Pattern, Query};

pub(crate) fn parse_json_query(val: &JsonValue) -> Result<Query, ParseError> {
    let query_val = val
        .get("query")
        .or_else(|| val.get(":query"))
        .ok_or_else(|| ParseError::MissingClause(":query".into()))?;

    let query_arr = query_val
        .as_array()
        .ok_or_else(|| ParseError::ExpectedVector(":query must be array".into()))?;

    let mut i = 0;
    let mut find = None;
    let mut where_patterns = Vec::new();

    while i < query_arr.len() {
        let keyword = query_arr[i]
            .as_str()
            .ok_or_else(|| ParseError::ExpectedKeyword(format!("{:?}", query_arr[i])))?;

        match keyword {
            ":find" => {
                i += 1;
                if i >= query_arr.len() {
                    return Err(ParseError::MissingClause(":find value".into()));
                }
                find = Some(parse_json_find(&query_arr[i])?);
                i += 1;
            }
            ":where" => {
                i += 1;
                while i < query_arr.len() {
                    if query_arr[i]
                        .as_str()
                        .map(|s| s.starts_with(':'))
                        .unwrap_or(false)
                    {
                        break;
                    }
                    let pat = query_arr[i].as_array().ok_or_else(|| {
                        ParseError::ExpectedVector("Pattern must be array".into())
                    })?;
                    if pat.len() != 3 {
                        return Err(ParseError::InvalidSyntax(
                            "Pattern must have 3 elements".into(),
                        ));
                    }
                    let entity = json_val_to_string(&pat[0])?;
                    let attr = json_val_to_string(&pat[1])?;
                    let val = json_val_to_string(&pat[2])?;
                    where_patterns.push(Pattern {
                        entity,
                        attribute: attr,
                        value: val,
                    });
                    i += 1;
                }
            }
            ":in" => {
                i += 1;
                while i < query_arr.len()
                    && !query_arr[i]
                        .as_str()
                        .map(|s| s.starts_with(':'))
                        .unwrap_or(true)
                {
                    i += 1;
                }
            }
            _ => {
                return Err(ParseError::ExpectedKeyword(format!(
                    "Unknown clause: {}",
                    keyword
                )))
            }
        }
    }

    Ok(Query {
        find: find.ok_or_else(|| ParseError::MissingClause(":find".into()))?,
        r#where: where_patterns,
    })
}

fn parse_json_find(val: &JsonValue) -> Result<FindSpec, ParseError> {
    match val {
        JsonValue::Array(items) => {
            if let Some(first) = items.first().and_then(|s| s.as_str()) {
                if first == "pull" && items.len() >= 3 {
                    let var = json_val_to_string(&items[1])?;
                    let attrs: Vec<String> = items[2]
                        .as_array()
                        .ok_or_else(|| ParseError::ExpectedVector("pull attrs".into()))?
                        .iter()
                        .map(json_val_to_string)
                        .collect::<Result<Vec<_>, _>>()?;
                    return Ok(FindSpec::Pull { var, attrs });
                }
            }
            let vars: Vec<String> = items
                .iter()
                .map(json_val_to_string)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(FindSpec::Vars(vars))
        }
        _ => Err(ParseError::ExpectedVector(":find must be array".into())),
    }
}

fn json_val_to_string(val: &JsonValue) -> Result<String, ParseError> {
    match val {
        JsonValue::String(s) => Ok(s.clone()),
        JsonValue::Number(n) => Ok(n.to_string()),
        JsonValue::Bool(b) => Ok(b.to_string()),
        _ => Err(ParseError::InvalidSyntax(format!(
            "Expected scalar, got: {:?}",
            val
        ))),
    }
}
