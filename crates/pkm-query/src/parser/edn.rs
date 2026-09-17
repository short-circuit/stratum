//! EDN-like tokenizer and parser.
//!
//! Parses Logseq-compatible Datalog EDN syntax into [`super::Query`] values.

use super::{FindSpec, ParseError, Pattern, Query, Value};

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
            c if c.is_alphanumeric()
                || c == '?'
                || c == '_'
                || c == '-'
                || c == '.'
                || c == '/'
                || c == '!' =>
            {
                let mut s = String::new();
                while i < chars.len()
                    && (chars[i].is_alphanumeric()
                        || chars[i] == '?'
                        || chars[i] == '_'
                        || chars[i] == '-'
                        || chars[i] == '.'
                        || chars[i] == '/'
                        || chars[i] == '!')
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
    let mut where_patterns = Vec::new();

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
                        Value::Vector(v) if v.len() == 3 => {
                            let entity = value_to_string(&v[0])?;
                            let attr = value_to_string(&v[1])?;
                            let val = value_to_string(&v[2])?;
                            where_patterns.push(Pattern {
                                entity,
                                attribute: attr,
                                value: val,
                            });
                        }
                        Value::Keyword(_) => break,
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
        r#where: where_patterns,
    })
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
