//! Datalog → SQL compiler against the blocks SQLite schema.

use crate::parser::{FindSpec, Query};
use std::collections::{HashMap, HashSet};

const ATTR_MAP: &[(&str, &str, &str)] = &[
    (":block/id", "b", "id"),
    (":block/content", "b", "content"),
    (":block/page", "b", "page_path"),
    (":block/parent", "b", "parent_id"),
    (":block/left", "b", "left_id"),
    (":block/marker", "b", "marker"),
    (":block/priority", "b", "priority"),
    (":block/collapsed", "b", "collapsed"),
    (":block/heading", "b", "heading_level"),
    (":block/created", "b", "created_at"),
    (":block/modified", "b", "modified_at"),
    (":page/path", "p", "path"),
    (":page/title", "p", "title"),
];

fn map_attr(attr: &str) -> Option<(&'static str, &'static str)> {
    ATTR_MAP
        .iter()
        .find(|(a, _, _)| *a == attr)
        .map(|(_, p, c)| (*p, *c))
}

#[derive(Debug)]
pub enum CompileError {
    UnknownAttribute(String),
    UnresolvedVariable(String),
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownAttribute(a) => write!(f, "Unknown attribute: {}", a),
            Self::UnresolvedVariable(v) => write!(f, "Unresolved variable: {}", v),
        }
    }
}
impl std::error::Error for CompileError {}

pub struct CompiledQuery {
    pub sql: String,
    pub params: Vec<String>,
}

pub fn compile(query: &Query) -> Result<CompiledQuery, CompileError> {
    let mut var_map: HashMap<String, (String, String)> = HashMap::new();
    let mut aliases: HashSet<String> = HashSet::new();
    let mut conditions: Vec<String> = Vec::new();
    let mut params: Vec<String> = Vec::new();

    for pattern in &query.r#where {
        let (alias, col) = map_attr(&pattern.attribute)
            .ok_or_else(|| CompileError::UnknownAttribute(pattern.attribute.clone()))?;
        let alias = alias.to_string();
        let col = col.to_string();

        aliases.insert(alias.clone());

        if pattern.entity.starts_with('?') {
            if let Some((prev_alias, _)) = var_map.get(&pattern.entity) {
                if *prev_alias != alias {
                    // Cross-table reference: join via foreign key
                    if (*prev_alias == "b" && alias == "p") || (*prev_alias == "p" && alias == "b")
                    {
                        conditions.push("b.page_path = p.path".to_string());
                    }
                }
            }
            var_map.insert(pattern.entity.clone(), (alias.clone(), col.clone()));
        }

        if pattern.value.starts_with('?') {
            if let Some((prev_alias, prev_col)) = var_map.get(&pattern.value) {
                if *prev_alias != alias {
                    conditions.push(format!("{}.{} = {}.{}", prev_alias, prev_col, alias, col));
                }
            }
            var_map.insert(pattern.value.clone(), (alias.clone(), col.clone()));
        } else if pattern.value != "_" {
            conditions.push(format!("{}.{} = ?{}", alias, col, params.len() + 1));
            params.push(pattern.value.clone());
        }
    }

    let find_vars = match &query.find {
        FindSpec::Vars(vars) => vars.clone(),
        FindSpec::Pull { var, attrs } => {
            let mut vars = vec![var.clone()];
            vars.extend(attrs.clone());
            vars
        }
    };

    let mut select_cols = Vec::new();
    for var in &find_vars {
        if let Some((alias, col)) = var_map.get(var) {
            select_cols.push(format!("{}.{} AS \"{}\"", alias, col, var));
        } else if var.starts_with('?') {
            return Err(CompileError::UnresolvedVariable(var.clone()));
        } else if let Some((alias, col)) = map_attr(var) {
            select_cols.push(format!("{}.{} AS \"{}\"", alias, col, var));
            aliases.insert(alias.to_string());
        }
    }

    let mut tables = Vec::new();
    for alias in &aliases {
        match alias.as_str() {
            "b" => tables.push("blocks b".to_string()),
            "p" => tables.push("pages p".to_string()),
            _ => {}
        }
    }

    if aliases.contains("b") && aliases.contains("p") {
        let join_cond = "b.page_path = p.path".to_string();
        if !conditions.contains(&join_cond) {
            conditions.push(join_cond);
        }
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let sql = format!(
        "SELECT {} FROM {} {}",
        select_cols.join(", "),
        tables.join(", "),
        where_clause,
    );

    Ok(CompiledQuery {
        sql: sql.trim().to_string(),
        params,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_query;

    #[test]
    fn test_compile_simple() {
        let q = parse_query(r#"{:query [:find ?b :where [?b :block/marker "TODO"]]}"#).unwrap();
        let c = compile(&q).unwrap();
        assert!(c.sql.contains("SELECT"), "SQL: {}", c.sql);
        assert!(
            c.params.contains(&"TODO".to_string()),
            "params: {:?}",
            c.params
        );
    }

    #[test]
    fn test_compile_multi_pattern() {
        let q = parse_query(r#"{:query [:find ?b ?content :where [?b :block/marker "TODO"] [?b :block/content ?content]]}"#).unwrap();
        let c = compile(&q).unwrap();
        assert!(c.sql.contains("b.content"));
        assert!(c.sql.contains("b.marker"));
    }

    #[test]
    fn test_compile_page_join() {
        let q = parse_query(r#"{:query [:find ?title :where [?b :block/marker "TODO"] [?b :block/page ?p] [?p :page/title ?title]]}"#).unwrap();
        let c = compile(&q).unwrap();
        assert!(c.sql.contains("pages p"));
    }

    #[test]
    fn test_unknown_attr() {
        let q = parse_query(r#"{:query [:find ?b :where [?b :unknown/attr "x"]]}"#).unwrap();
        assert!(compile(&q).is_err());
    }
}
