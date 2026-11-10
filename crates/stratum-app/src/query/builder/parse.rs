//! Best-effort EDN scanner and round-trip parser for builder-generated Datalog
//! queries.
//!
//! Accepts only the subset the visual builder emits (triples, predicate lists,
//! `(or ...)` groups, `(not ...)` negation, whitelisted attributes/operators).
//! Returns `None` for pull find specs, unknown attributes/operators, JSON syntax
//! or malformed input, mirroring `src/lib/datalogParser.ts`.

use super::fields::{field_by_attr, Combiner, FieldId, FilterSpec, OpId};

#[derive(Debug, Clone)]
enum Tok {
    Delim(char),
    Str(String),
    Kw(String),
    Sym(String),
}

fn is_symbol_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '?' | '_' | '-' | '.' | '/' | '!' | '=' | '<' | '>')
}

fn tokenize(input: &str) -> Option<Vec<Tok>> {
    let chars: Vec<char> = input.chars().collect();
    let mut toks = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if matches!(c, '{' | '}' | '[' | ']' | '(' | ')') {
            toks.push(Tok::Delim(c));
            i += 1;
        } else if c == '"' {
            i += 1;
            let mut s = String::new();
            while i < chars.len() && chars[i] != '"' {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    s.push(chars[i + 1]);
                    i += 2;
                } else {
                    s.push(chars[i]);
                    i += 1;
                }
            }
            if i >= chars.len() {
                return None;
            }
            i += 1;
            toks.push(Tok::Str(s));
        } else if c == ':' {
            i += 1;
            let mut s = String::new();
            while i < chars.len() && is_symbol_char(chars[i]) && chars[i] != '?' {
                s.push(chars[i]);
                i += 1;
            }
            if s.is_empty() {
                return None;
            }
            toks.push(Tok::Kw(s));
        } else if c == ';' {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else if c.is_whitespace() {
            i += 1;
        } else if is_symbol_char(c) {
            let mut s = String::new();
            while i < chars.len() && is_symbol_char(chars[i]) {
                s.push(chars[i]);
                i += 1;
            }
            toks.push(Tok::Sym(s));
        } else {
            return None;
        }
    }
    Some(toks)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NodeKind {
    Str,
    Kw,
    Sym,
    Vec,
}

#[derive(Debug, Clone)]
struct Node {
    kind: NodeKind,
    text: String,
    kids: Option<Vec<Node>>,
}

fn scan(toks: &[Tok], from: usize) -> Option<(Vec<Node>, usize)> {
    let mut nodes = Vec::new();
    let mut i = from;
    while i < toks.len() {
        match &toks[i] {
            Tok::Delim(c) => {
                if *c == '[' || *c == '(' {
                    let (kids, next) = scan(toks, i + 1)?;
                    nodes.push(Node {
                        kind: NodeKind::Vec,
                        text: String::new(),
                        kids: Some(kids),
                    });
                    i = next;
                } else if *c == ']' || *c == ')' {
                    return Some((nodes, i + 1));
                } else {
                    i += 1;
                }
            }
            Tok::Kw(text) => {
                nodes.push(Node {
                    kind: NodeKind::Kw,
                    text: format!(":{text}"),
                    kids: None,
                });
                i += 1;
            }
            Tok::Str(text) => {
                nodes.push(Node {
                    kind: NodeKind::Str,
                    text: text.clone(),
                    kids: None,
                });
                i += 1;
            }
            Tok::Sym(text) => {
                nodes.push(Node {
                    kind: NodeKind::Sym,
                    text: text.clone(),
                    kids: None,
                });
                i += 1;
            }
        }
    }
    Some((nodes, i))
}

fn node_text(node: &Node) -> Option<&str> {
    match node.kind {
        NodeKind::Kw | NodeKind::Str | NodeKind::Sym => Some(node.text.as_str()),
        NodeKind::Vec => None,
    }
}

fn parse_query_items(datalog: &str) -> Option<Vec<Node>> {
    let toks = tokenize(datalog)?;
    let (root, next) = scan(&toks, 0)?;
    if next != toks.len() {
        return None;
    }

    let query_vec = root.windows(2).find_map(|pair| {
        if pair[0].kind == NodeKind::Kw && pair[0].text == ":query" && pair[1].kind == NodeKind::Vec
        {
            Some(pair[1].clone())
        } else {
            None
        }
    });
    let query_vec = query_vec?;
    let kids = query_vec.kids?;

    let mut where_items = Vec::new();
    let mut found_find = false;
    let mut i = 0;
    while i < kids.len() {
        let k = &kids[i];
        if k.kind != NodeKind::Kw {
            return None;
        }
        if k.text == ":find" {
            let mut j = i + 1;
            let mut count = 0;
            while j < kids.len() && kids[j].kind != NodeKind::Kw {
                if kids[j].kind != NodeKind::Sym || !kids[j].text.starts_with('?') {
                    return None;
                }
                count += 1;
                j += 1;
            }
            if count == 0 {
                return None;
            }
            found_find = true;
            i = j;
        } else if k.text == ":where" {
            let mut j = i + 1;
            while j < kids.len() && kids[j].kind != NodeKind::Kw {
                if kids[j].kind != NodeKind::Vec {
                    return None;
                }
                where_items.push(kids[j].clone());
                j += 1;
            }
            i = j;
        } else if k.text == ":order-by" || k.text == ":in" {
            if i + 1 < kids.len() {
                i += 2;
            } else {
                i += 1;
            }
        } else {
            return None;
        }
    }
    if !found_find {
        return None;
    }
    Some(where_items)
}

#[derive(Debug, Clone)]
enum ParsedItem {
    Triple {
        entity: String,
        attr: String,
        value: String,
    },
    Pred {
        op: String,
        args: Vec<String>,
    },
    Or(Vec<ParsedItem>),
    Not(Box<ParsedItem>),
}

fn parse_predicate_node(node: &Node) -> Option<(String, Vec<String>)> {
    let kids = node.kids.as_ref()?;
    if node.kind != NodeKind::Vec || kids.is_empty() {
        return None;
    }
    let op = node_text(&kids[0])?;
    if op.starts_with('?') || op.starts_with(':') {
        return None;
    }
    let mut args = Vec::new();
    for kid in &kids[1..] {
        args.push(node_text(kid)?.to_string());
    }
    Some((op.to_string(), args))
}

fn parse_where_item(node: &Node) -> Option<ParsedItem> {
    if node.kind != NodeKind::Vec {
        return None;
    }
    let kids = node.kids.as_ref()?;
    let first = kids.first()?;

    if first.kind == NodeKind::Sym && first.text == "or" {
        let mut items = Vec::new();
        for sub in &kids[1..] {
            let parsed = parse_where_item(sub)?;
            if matches!(parsed, ParsedItem::Or(_) | ParsedItem::Not(_)) {
                return None;
            }
            items.push(parsed);
        }
        return Some(ParsedItem::Or(items));
    }

    if first.kind == NodeKind::Sym && first.text == "not" {
        if kids.len() != 2 {
            return None;
        }
        let inner = parse_where_item(&kids[1])?;
        if matches!(inner, ParsedItem::Or(_) | ParsedItem::Not(_)) {
            return None;
        }
        return Some(ParsedItem::Not(Box::new(inner)));
    }

    if first.kind == NodeKind::Sym && first.text.starts_with('?') && kids.len() == 3 {
        let attr = node_text(&kids[1])?;
        let value = node_text(&kids[2])?;
        if !attr.starts_with(':') {
            return None;
        }
        return Some(ParsedItem::Triple {
            entity: first.text.clone(),
            attr: attr.to_string(),
            value: value.to_string(),
        });
    }

    if first.kind == NodeKind::Vec {
        if kids.len() != 1 {
            return None;
        }
        let (op, args) = parse_predicate_node(&kids[0])?;
        return Some(ParsedItem::Pred { op, args });
    }

    None
}

fn op_from_edn(op: &str) -> Option<OpId> {
    match op {
        "=" => Some(OpId::Eq),
        "not=" | "!=" => Some(OpId::Ne),
        ">" => Some(OpId::Gt),
        "<" => Some(OpId::Lt),
        ">=" => Some(OpId::Gte),
        "<=" => Some(OpId::Lte),
        "contains?" | "includes?" | "clojure.string/includes?" => Some(OpId::Contains),
        "starts-with?" | "clojure.string/starts-with?" => Some(OpId::StartsWith),
        "present?" => Some(OpId::Present),
        "absent?" => Some(OpId::Absent),
        _ => None,
    }
}

struct ParserState {
    counter: usize,
    specs: Vec<FilterSpec>,
}

fn push_row(
    state: &mut ParserState,
    field: FieldId,
    op: OpId,
    value: String,
    group_mode: bool,
    first_in_group: &mut bool,
) {
    let combiner = if group_mode && !*first_in_group {
        Combiner::Or
    } else {
        Combiner::And
    };
    *first_in_group = false;
    state.specs.push(FilterSpec {
        id: format!("p{}", state.counter),
        field,
        op,
        value,
        combiner,
    });
    state.counter += 1;
}

fn items_to_specs(items: &[ParsedItem], state: &mut ParserState, group_mode: bool) -> bool {
    let mut pending: Option<(FieldId, String)> = None;
    let mut first_in_group = true;

    for item in items {
        match item {
            ParsedItem::Pred { op, args } => {
                let Some(op_id) = op_from_edn(op) else {
                    return false;
                };
                let Some((field, var_name)) = pending.take() else {
                    return false;
                };
                if args.first() != Some(&var_name) {
                    return false;
                }
                let value = args.get(1).cloned().unwrap_or_default();
                push_row(state, field, op_id, value, group_mode, &mut first_in_group);
            }
            ParsedItem::Triple {
                entity,
                attr,
                value,
            } => {
                if attr == ":block/page" && value.starts_with('?') {
                    continue;
                }
                if attr == ":block/content" && value == "?content" && entity.starts_with('?') {
                    continue;
                }
                let Some(field) = field_by_attr(attr) else {
                    return false;
                };
                if value.starts_with('?') {
                    pending = Some((field, value.clone()));
                } else {
                    pending = None;
                    let op = if matches!(field, FieldId::Tags | FieldId::HasWikilink) {
                        OpId::Has
                    } else {
                        OpId::Eq
                    };
                    push_row(
                        state,
                        field,
                        op,
                        value.clone(),
                        group_mode,
                        &mut first_in_group,
                    );
                }
            }
            ParsedItem::Or(sub_items) => {
                pending = None;
                if !items_to_specs(sub_items, state, true) {
                    return false;
                }
                first_in_group = false;
            }
            ParsedItem::Not(boxed) => {
                pending = None;
                let ParsedItem::Triple { attr, value, .. } = boxed.as_ref() else {
                    return false;
                };
                let Some(field) = field_by_attr(attr) else {
                    return false;
                };
                let op = if matches!(field, FieldId::Tags | FieldId::HasWikilink) {
                    OpId::NotHas
                } else {
                    OpId::Ne
                };
                push_row(
                    state,
                    field,
                    op,
                    value.clone(),
                    group_mode,
                    &mut first_in_group,
                );
            }
        }
    }
    true
}

/// Parse builder-generated EDN back into filter rows.
///
/// Returns `None` when the query is outside the accepted subset. Generated row
/// ids use the `p{counter}` scheme.
pub fn parse_datalog_to_rows(datalog: &str) -> Option<Vec<FilterSpec>> {
    let where_nodes = parse_query_items(datalog)?;
    let mut items = Vec::with_capacity(where_nodes.len());
    for node in &where_nodes {
        items.push(parse_where_item(node)?);
    }
    let mut state = ParserState {
        counter: 0,
        specs: Vec::new(),
    };
    if !items_to_specs(&items, &mut state, false) {
        return None;
    }
    Some(state.specs)
}
