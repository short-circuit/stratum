//! JSON-format query parser.
//!
//! Parses JSON-formatted Datalog queries into [`super::Query`] values.
//! Mirrors the EDN parser's extended forms: predicate lists, or-groups,
//! negation and top-level `:order-by`.

use serde_json::Value as JsonValue;

use super::{FindSpec, OrderBy, ParseError, Pattern, Predicate, Query, WhereClause};

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
    let mut where_clauses = Vec::new();
    let mut order_by = Vec::new();

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
                    where_clauses.extend(parse_json_where_items(&query_arr[i])?);
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
            ":order-by" => {
                i += 1;
                if i >= query_arr.len() {
                    return Err(ParseError::MissingClause(":order-by value".into()));
                }
                order_by = parse_json_order_by(&query_arr[i])?;
                i += 1;
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
        r#where: where_clauses,
        order_by,
    })
}

fn parse_json_where_items(val: &JsonValue) -> Result<Vec<WhereClause>, ParseError> {
    let arr = val
        .as_array()
        .ok_or_else(|| ParseError::ExpectedVector("Pattern must be array".into()))?;

    // (or ...) / (not ...)
    if let Some(first) = arr.first().and_then(|v| v.as_str()) {
        match first {
            "or" => {
                if arr.len() < 3 {
                    return Err(ParseError::InvalidSyntax(
                        "(or ...) needs at least 2 clauses".into(),
                    ));
                }
                let mut clauses = Vec::new();
                for sub in &arr[1..] {
                    clauses.push(parse_json_where_subclause(sub)?);
                }
                return Ok(vec![WhereClause::Or(clauses)]);
            }
            "not" => {
                if arr.len() != 2 {
                    return Err(ParseError::InvalidSyntax(
                        "(not ...) needs exactly 1 clause".into(),
                    ));
                }
                return Ok(vec![WhereClause::Not(Box::new(
                    parse_json_where_subclause(&arr[1])?,
                ))]);
            }
            _ => {}
        }
    }

    // Predicate list: an array whose elements are all arrays starting with an
    // operator symbol (`[["not=", "?m", "DONE"]]`).
    if !arr.is_empty()
        && arr.iter().all(|el| {
            el.as_array()
                .and_then(|inner| inner.first())
                .and_then(|f| f.as_str())
                .map(|s| !s.starts_with('?') && !s.starts_with(':'))
                .unwrap_or(false)
        })
    {
        let mut preds = Vec::new();
        for el in arr {
            if let Some(inner) = el.as_array() {
                preds.push(parse_json_predicate(inner)?);
            }
        }
        return Ok(preds.into_iter().map(WhereClause::Pred).collect());
    }

    // Plain triple: [entity, :attribute, value]
    if arr.len() == 3 {
        let entity = json_val_to_string(&arr[0])?;
        let attr = json_val_to_string(&arr[1])?;
        let val = json_val_to_string(&arr[2])?;
        return Ok(vec![WhereClause::Triple(Pattern {
            entity,
            attribute: attr,
            value: val,
        })]);
    }

    Err(ParseError::ExpectedVector("Pattern must be array".into()))
}

fn parse_json_where_subclause(val: &JsonValue) -> Result<WhereClause, ParseError> {
    let arr = val
        .as_array()
        .ok_or_else(|| ParseError::ExpectedVector("Clause must be array".into()))?;

    // Predicate list: `[["not=", "?m", "DONE"]]` or a flat predicate `["=", "?m", "x"]`.
    if arr.len() == 1 {
        if let Some(inner) = arr[0].as_array() {
            return Ok(WhereClause::Pred(parse_json_predicate(inner)?));
        }
    }
    if arr.len() == 3 && arr[1].as_str().map(|s| s.starts_with(':')).unwrap_or(false) {
        let entity = json_val_to_string(&arr[0])?;
        let attr = json_val_to_string(&arr[1])?;
        let val = json_val_to_string(&arr[2])?;
        return Ok(WhereClause::Triple(Pattern {
            entity,
            attribute: attr,
            value: val,
        }));
    }
    if let Some(op) = arr.first().and_then(|v| v.as_str()) {
        if !op.starts_with('?') && !op.starts_with(':') && arr.len() >= 2 {
            return Ok(WhereClause::Pred(parse_json_predicate(arr)?));
        }
    }
    Err(ParseError::InvalidSyntax(
        "Clause must be a triple or predicate".into(),
    ))
}

fn parse_json_predicate(arr: &[JsonValue]) -> Result<Predicate, ParseError> {
    if arr.is_empty() {
        return Err(ParseError::InvalidSyntax("Empty predicate".into()));
    }
    let op = json_val_to_string(&arr[0])?;
    let mut args = Vec::new();
    for arg in &arr[1..] {
        args.push(json_val_to_string(arg)?);
    }
    Ok(Predicate { op, args })
}

fn parse_json_order_by(val: &JsonValue) -> Result<Vec<OrderBy>, ParseError> {
    let entries = val
        .as_array()
        .ok_or_else(|| ParseError::ExpectedVector(":order-by must be array".into()))?;
    let mut out = Vec::new();
    for entry in entries {
        let pair = entry
            .as_array()
            .ok_or_else(|| ParseError::InvalidSyntax(":order-by entry must be array".into()))?;
        if pair.len() != 2 {
            return Err(ParseError::InvalidSyntax(
                ":order-by entry must be [var direction]".into(),
            ));
        }
        let var = json_val_to_string(&pair[0])?;
        let dir = json_val_to_string(&pair[1])?;
        let desc = match dir.as_str() {
            ":asc" => false,
            ":desc" => true,
            other => {
                return Err(ParseError::InvalidSyntax(format!(
                    "Unknown order-by direction: {}",
                    other
                )))
            }
        };
        out.push(OrderBy { var, desc });
    }
    Ok(out)
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
