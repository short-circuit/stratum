//! Datalog query parser.
//!
//! Parses Logseq-compatible Datalog syntax using a simple EDN-like tokenizer.
//! Supports both EDN syntax and JSON syntax for queries.

use serde_json::Value as JsonValue;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Keyword(String), // :keyword
    String(String),  // "string" or ?var or symbol
    Vector(Vec<Value>),
    Map(Vec<(String, Value)>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Query {
    pub find: FindSpec,
    pub r#where: Vec<Pattern>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FindSpec {
    Vars(Vec<String>),
    Pull { var: String, attrs: Vec<String> },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Pattern {
    pub entity: String,
    pub attribute: String,
    pub value: String,
}

#[derive(Debug)]
pub enum ParseError {
    InvalidSyntax(String),
    ExpectedKeyword(String),
    ExpectedVector(String),
    MissingClause(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSyntax(s) => write!(f, "Invalid syntax: {}", s),
            Self::ExpectedKeyword(s) => write!(f, "Expected keyword, got: {}", s),
            Self::ExpectedVector(s) => write!(f, "Expected vector, got: {}", s),
            Self::MissingClause(s) => write!(f, "Missing clause: {}", s),
        }
    }
}
impl std::error::Error for ParseError {}

/// Parse a Datalog query string.
/// First tries JSON, falls back to EDN-like syntax.
pub fn parse_query(input: &str) -> Result<Query, ParseError> {
    if let Ok(val) = serde_json::from_str::<JsonValue>(input) {
        return json::parse_json_query(&val);
    }

    let tokens = edn::tokenize(input)?;
    let val = edn::parse_edn(&tokens, &mut 0)?;
    edn::parse_edn_query(&val)
}

mod edn;
mod json;
#[cfg(test)]
mod tests;
