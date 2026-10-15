//! Datalog query parser.
//!
//! Parses Logseq-compatible Datalog syntax using a simple EDN-like tokenizer.
//! Supports both EDN syntax and JSON syntax for queries.
//!
//! The `:where` clause accepts the extended Logseq-compatible forms in
//! addition to plain `[entity attribute value]` triples:
//! predicate lists (`[(not= ?m "DONE")]`, `[(clojure.string/includes? ?c "x")]`),
//! negation (`(not <clause>)`) and or-groups (`(or <clause> <clause>)`).
//! A top-level `:order-by` clause orders results by bound variables.

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
    pub r#where: Vec<WhereClause>,
    pub order_by: Vec<OrderBy>,
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

/// A single clause of a `:where` vector.
#[derive(Debug, Clone, PartialEq)]
pub enum WhereClause {
    /// Plain `[entity attribute value]` triple.
    Triple(Pattern),
    /// Predicate list form: `[(<op> <arg>...)]`, e.g. `[(not= ?m "DONE")]`.
    Pred(Predicate),
    /// Or-group: `(or <clause> <clause> ...)`.
    Or(Vec<WhereClause>),
    /// Negation: `(not <clause>)`.
    Not(Box<WhereClause>),
}

impl WhereClause {
    /// Returns the contained [`Pattern`] when this is a plain triple.
    pub fn as_triple(&self) -> Option<&Pattern> {
        match self {
            WhereClause::Triple(p) => Some(p),
            _ => None,
        }
    }
}

/// A predicate applied to already-bound variables, e.g. `(not= ?m "DONE")`.
#[derive(Debug, Clone, PartialEq)]
pub struct Predicate {
    /// The operator name: `=`, `!=`, `not=`, `>`, `<`, `>=`, `<=`,
    /// `contains?`, `includes?`, `clojure.string/includes?`,
    /// `starts-with?`, `clojure.string/starts-with?`, `present?`, `absent?`.
    pub op: String,
    /// Arguments as raw text: the bound variable first, then literal(s).
    pub args: Vec<String>,
}

/// A single `:order-by` entry: a bound variable and a sort direction.
#[derive(Debug, Clone, PartialEq)]
pub struct OrderBy {
    pub var: String,
    pub desc: bool,
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
