//! EDN-like tokenizer and parser.
//!
//! Parses Logseq-compatible Datalog EDN syntax into [`super::Query`] values.
//!
//! Besides plain `[entity attribute value]` triples, the `:where` clause
//! supports Logseq-style predicate lists (`[(not= ?m "DONE")]`), or-groups
//! (`(or <clause> <clause>)`) and negation (`(not <clause>)`), plus a
//! top-level `:order-by` clause.

use super::{FindSpec, OrderBy, ParseError, Pattern, Predicate, Query, Value, WhereClause};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Token {
    OpenBrace,
    CloseBrace,
    OpenBracket,
    CloseBracket,
    OpenParen,
    CloseParen,
    Keyword(String),
    String(String),
    Symbol(String),
}

pub(crate) fn tokenize(input: &str) -> Result<Vec<Token>, ParseError> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        match chars[i] {
            '{' => {
                tokens.push(Token::OpenBrace);
                i += 1;
            }
            '}' => {
                tokens.push(Token::CloseBrace);
                i += 1;
            }
            '[' => {
                tokens.push(Token::OpenBracket);
                i += 1;
            }
            ']' => {
                tokens.push(Token::CloseBracket);
                i += 1;
            }
            '(' => {
                tokens.push(Token::OpenParen);
                i += 1;
            }
            ')' => {
                tokens.push(Token::CloseParen);
                i += 1;
            }
            '"' => {
                i += 1;
                let mut s = String::new();
                while i < chars.len() && chars[i] != '"' {
                    if chars[i] == '\\' {
                        i += 1;
                    }
                    if i < chars.len() {
                        s.push(chars[i]);
                        i += 1;
                    }
                }
                if i < chars.len() {
                    i += 1;
                } // closing quote
                tokens.push(Token::String(s));
            }
            ':' => {
                i += 1;
                let mut s = String::new();
                while i < chars.len()
                    && (chars[i].is_alphanumeric()
                        || chars[i] == '-'
                        || chars[i] == '_'
                        || chars[i] == '/'
                        || chars[i] == '.')
                {
                    s.push(chars[i]);
                    i += 1;
                }
                if s.is_empty() {
                    return Err(ParseError::InvalidSyntax("Empty keyword".into()));
                }
                tokens.push(Token::Keyword(s));
            }
            ';' => {
                // Comment until end of line
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
            }
            c if c.is_whitespace() => {
                i += 1;
            }
            // Symbols may include comparison operators so Logseq-style
            // predicates (`not=`, `!=`, `>=`, `<=`, `>`, `<`) tokenize as a
            // single symbol.
            c if c.is_alphanumeric()
                || c == '?'
                || c == '_'
                || c == '-'
                || c == '.'
                || c == '/'
                || c == '!'
                || c == '='
                || c == '<'
                || c == '>' =>
            {
                let mut s = String::new();
                while i < chars.len()
                    && (chars[i].is_alphanumeric()
                        || chars[i] == '?'
                        || chars[i] == '_'
                        || chars[i] == '-'
                        || chars[i] == '.'
                        || chars[i] == '/'
                        || chars[i] == '!'
                        || chars[i] == '='
                        || chars[i] == '<'
                        || chars[i] == '>')
                {
                    s.push(chars[i]);
                    i += 1;
                }
                tokens.push(Token::Symbol(s));
            }
            c => {
                return Err(ParseError::InvalidSyntax(format!(
                    "Unexpected char: '{}'",
                    c
                )));
            }
        }
    }

    Ok(tokens)
}

pub(crate) fn parse_edn(tokens: &[Token], pos: &mut usize) -> Result<Value, ParseError> {
    if *pos >= tokens.len() {
        return Err(ParseError::InvalidSyntax("Unexpected end of input".into()));
    }

    match &tokens[*pos] {
        Token::OpenBrace => {
            *pos += 1;
            let mut pairs = Vec::new();
            while *pos < tokens.len() && !matches!(tokens[*pos], Token::CloseBrace) {
                let key = match &tokens[*pos] {
                    Token::Keyword(k) => {
                        *pos += 1;
                        format!(":{}", k)
                    }
                    Token::String(s) => {
                        *pos += 1;
                        s.clone()
                    }
                    Token::Symbol(s) => {
                        *pos += 1;
                        s.clone()
                    }
                    _ => {
                        return Err(ParseError::InvalidSyntax(
                            "Expected keyword or string key".into(),
                        ))
                    }
                };
                let val = parse_edn(tokens, pos)?;
                pairs.push((key, val));
            }
            if *pos < tokens.len() {
                *pos += 1;
            } // skip }
            Ok(Value::Map(pairs))
        }
        Token::OpenBracket => {
            *pos += 1;
            let mut items = Vec::new();
            while *pos < tokens.len() && !matches!(tokens[*pos], Token::CloseBracket) {
                items.push(parse_edn(tokens, pos)?);
            }
            if *pos < tokens.len() {
                *pos += 1;
            } // skip ]
            Ok(Value::Vector(items))
        }
        Token::OpenParen => {
            // Treat parens as vectors (list syntax)
            *pos += 1;
            let mut items = Vec::new();
            while *pos < tokens.len() && !matches!(tokens[*pos], Token::CloseParen) {
                items.push(parse_edn(tokens, pos)?);
            }
            if *pos < tokens.len() {
                *pos += 1;
            } // skip )
            Ok(Value::Vector(items))
        }
        Token::Keyword(k) => {
            *pos += 1;
            Ok(Value::Keyword(format!(":{}", k)))
        }
        Token::String(s) => {
            *pos += 1;
            Ok(Value::String(s.clone()))
        }
        Token::Symbol(s) => {
            *pos += 1;
            Ok(Value::String(s.clone()))
        }
        Token::CloseBrace | Token::CloseBracket | Token::CloseParen => Err(
            ParseError::InvalidSyntax("Unexpected closing bracket".into()),
        ),
    }
}

pub(crate) fn parse_edn_query(val: &Value) -> Result<Query, ParseError> {
    let map = match val {
        Value::Map(pairs) => pairs,
        _ => return Err(ParseError::InvalidSyntax("Query must be a map".into())),
    };

    let query_val = map
        .iter()
        .find(|(k, _)| k == ":query")
        .map(|(_, v)| v)
        .ok_or_else(|| ParseError::MissingClause(":query".into()))?;

    let query_vec = match query_val {
        Value::Vector(items) => items,
        _ => return Err(ParseError::ExpectedVector(":query must be a vector".into())),
    };

    parse_query_vec(query_vec)
}

fn parse_query_vec(items: &[Value]) -> Result<Query, ParseError> {
    let mut i = 0;
    let mut find = None;
    let mut where_clauses = Vec::new();
    let mut order_by = Vec::new();

    while i < items.len() {
        let keyword = match &items[i] {
            Value::Keyword(k) => k.clone(),
            Value::String(s) if s.starts_with(':') => s.clone(),
            _ => return Err(ParseError::ExpectedKeyword(format!("{:?}", items[i]))),
        };

        match keyword.as_str() {
            ":find" => {
                i += 1;
                if i >= items.len() {
                    return Err(ParseError::MissingClause(":find value".into()));
                }
                // Collect find vars until we hit the next keyword
                let mut find_vars = Vec::new();
                while i < items.len() && !matches!(&items[i], Value::Keyword(_)) {
                    match &items[i] {
                        Value::Vector(v) => {
                            // Check for (pull ...) expression
                            if let Some(Value::String(s)) = v.first() {
                                if s == "pull" {
                                    find = Some(parse_pull_from_vec(v)?);
                                    i += 1;
                                    break;
                                }
                            }
                            // Otherwise it's a nested vector (shouldn't happen)
                            return Err(ParseError::InvalidSyntax(
                                "Unexpected vector in :find".into(),
                            ));
                        }
                        Value::String(s) => {
                            find_vars.push(s.clone());
                        }
                        _ => break,
                    }
                    i += 1;
                }
                if find.is_none() {
                    find = Some(FindSpec::Vars(find_vars));
                }
            }
            ":where" => {
                i += 1;
                while i < items.len() {
                    match &items[i] {
                        Value::Keyword(_) => break,
                        Value::Vector(_) => {
                            where_clauses.extend(parse_where_items(&items[i])?);
                        }
                        _ => {
                            return Err(ParseError::ExpectedVector(
                                "Pattern must be [e a v]".into(),
                            ))
                        }
                    }
                    i += 1;
                }
            }
            ":in" => {
                i += 1;
                while i < items.len() && !matches!(&items[i], Value::Keyword(_)) {
                    i += 1;
                }
            }
            ":order-by" => {
                i += 1;
                if i >= items.len() {
                    return Err(ParseError::MissingClause(":order-by value".into()));
                }
                order_by = parse_order_by(&items[i])?;
                i += 1;
            }
            _ => {
                return Err(ParseError::ExpectedKeyword(format!(
                    "Unknown clause: {}",
                    keyword
                )));
            }
        }
    }

    Ok(Query {
        find: find.ok_or_else(|| ParseError::MissingClause(":find".into()))?,
        r#where: where_clauses,
        order_by,
    })
}

/// Parse one `:where` entry into zero or more [`WhereClause`]s.
///
/// Accepted shapes (Logseq-compatible):
/// * `[entity attribute value]` — a triple (`Value::Vector` of length 3)
/// * `[(<op> <arg>...)]` — a predicate list (`Value::Vector` of length 1
///   whose sole element is another vector starting with an operator symbol)
/// * `[(<op> <arg>...) ...]` — several predicate lists in one vector
/// * `(or <clause> <clause> ...)` — an or-group
/// * `(not <clause>)` — a negation
fn parse_where_items(value: &Value) -> Result<Vec<WhereClause>, ParseError> {
    let items = match value {
        Value::Vector(v) => v,
        _ => return Err(ParseError::ExpectedVector("Pattern must be [e a v]".into())),
    };

    // (or ...) / (not ...) — parenthesized lists parse to a vector whose
    // first element is the plain symbol `or` / `not`.
    if let Some(Value::String(head)) = items.first() {
        match head.as_str() {
            "or" => {
                if items.len() < 3 {
                    return Err(ParseError::InvalidSyntax(
                        "(or ...) needs at least 2 clauses".into(),
                    ));
                }
                let mut clauses = Vec::new();
                for sub in &items[1..] {
                    clauses.push(parse_where_subclause(sub)?);
                }
                return Ok(vec![WhereClause::Or(clauses)]);
            }
            "not" => {
                if items.len() != 2 {
                    return Err(ParseError::InvalidSyntax(
                        "(not ...) needs exactly 1 clause".into(),
                    ));
                }
                return Ok(vec![WhereClause::Not(Box::new(parse_where_subclause(
                    &items[1],
                )?))]);
            }
            _ => {}
        }
    }

    // Predicate list(s): a vector whose elements are all vectors starting with
    // an operator symbol (`(not= ?m "DONE")` etc.).
    if !items.is_empty()
        && items.iter().all(|el| {
            matches!(el, Value::Vector(inner) if inner.first().map(|f| matches!(f, Value::String(s) if !s.starts_with('?') && !s.starts_with(':'))).unwrap_or(false))
        })
    {
        let mut preds = Vec::new();
        for el in items {
            if let Value::Vector(inner) = el {
                preds.push(parse_predicate(inner)?);
            }
        }
        return Ok(preds.into_iter().map(WhereClause::Pred).collect());
    }

    // Plain triple.
    if items.len() == 3 {
        let entity = value_to_string(&items[0])?;
        let attr = value_to_string(&items[1])?;
        let val = value_to_string(&items[2])?;
        return Ok(vec![WhereClause::Triple(Pattern {
            entity,
            attribute: attr,
            value: val,
        })]);
    }

    Err(ParseError::ExpectedVector("Pattern must be [e a v]".into()))
}

/// Parse a clause inside `(or ...)` / `(not ...)`: a triple or a predicate.
fn parse_where_subclause(value: &Value) -> Result<WhereClause, ParseError> {
    let items = match value {
        Value::Vector(v) => v,
        _ => {
            return Err(ParseError::ExpectedVector(
                "Clause must be a triple or predicate".into(),
            ))
        }
    };
    // Predicate list: `[(not= ?m "DONE")]` or a flat predicate `(> ?n "5")`.
    if items.len() == 1 {
        if let Value::Vector(inner) = &items[0] {
            return Ok(WhereClause::Pred(parse_predicate(inner)?));
        }
    }
    // A triple has a `:attribute` keyword in the SECOND slot.
    if items.len() == 3
        && (matches!(&items[1], Value::String(a) if a.starts_with(':'))
            || matches!(&items[1], Value::Keyword(_)))
    {
        let entity = value_to_string(&items[0])?;
        let attr = value_to_string(&items[1])?;
        let val = value_to_string(&items[2])?;
        return Ok(WhereClause::Triple(Pattern {
            entity,
            attribute: attr,
            value: val,
        }));
    }
    // A predicate written as a flat list `(> ?n "5")` (no extra bracket).
    if let Some(Value::String(op)) = items.first() {
        if !op.starts_with('?') && !op.starts_with(':') && items.len() >= 2 {
            return Ok(WhereClause::Pred(parse_predicate(items)?));
        }
    }
    Err(ParseError::InvalidSyntax(
        "Clause must be a triple or predicate".into(),
    ))
}

fn parse_predicate(items: &[Value]) -> Result<Predicate, ParseError> {
    if items.is_empty() {
        return Err(ParseError::InvalidSyntax("Empty predicate".into()));
    }
    let op = value_to_string(&items[0])?;
    let mut args = Vec::new();
    for arg in &items[1..] {
        args.push(value_to_string(arg)?);
    }
    Ok(Predicate { op, args })
}

fn parse_order_by(value: &Value) -> Result<Vec<OrderBy>, ParseError> {
    let entries = match value {
        Value::Vector(v) => v,
        _ => {
            return Err(ParseError::ExpectedVector(
                ":order-by must be a vector".into(),
            ))
        }
    };
    let mut out = Vec::new();
    for entry in entries {
        let pair = match entry {
            Value::Vector(v) => v,
            _ => {
                return Err(ParseError::InvalidSyntax(
                    ":order-by entry must be [var direction]".into(),
                ))
            }
        };
        if pair.len() != 2 {
            return Err(ParseError::InvalidSyntax(
                ":order-by entry must be [var direction]".into(),
            ));
        }
        let var = value_to_string(&pair[0])?;
        let dir = value_to_string(&pair[1])?;
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

fn parse_pull_from_vec(v: &[Value]) -> Result<FindSpec, ParseError> {
    // (pull ?var [:attr1 :attr2])
    if v.len() < 3 {
        return Err(ParseError::InvalidSyntax(
            "Pull needs (pull var attrs)".into(),
        ));
    }
    let var = value_to_string(&v[1])?;
    let attrs = match &v[2] {
        Value::Vector(av) => av
            .iter()
            .map(value_to_string)
            .collect::<Result<Vec<_>, _>>()?,
        _ => {
            return Err(ParseError::ExpectedVector(
                "pull attrs must be vector".into(),
            ))
        }
    };
    Ok(FindSpec::Pull { var, attrs })
}

fn value_to_string(value: &Value) -> Result<String, ParseError> {
    match value {
        Value::String(s) => Ok(s.clone()),
        Value::Keyword(k) => Ok(k.clone()),
        _ => Err(ParseError::InvalidSyntax(format!(
            "Expected string or keyword, got: {:?}",
            value
        ))),
    }
}
