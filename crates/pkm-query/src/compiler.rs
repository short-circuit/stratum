//! Datalog → SQL compiler against the blocks SQLite schema.
//!
//! Attributes documented in `docs/guide/datalog-queries.md` are all supported.
//! Plain scalar attributes compile to `alias.column` comparisons; set/map
//! attributes (`:page/tags`, `:block/tags`, `:page/links`, `:page/backlinks`,
//! `:block/properties`) compile to SQL subqueries / JSON predicates so the
//! documented queries return results instead of a "Unknown attribute" error.

use crate::parser::{FindSpec, Query};
use std::collections::{HashMap, HashSet};

/// Resolved attribute describing how to compile a datalog attribute.
enum Attr {
    /// Plain column on a table alias: (alias, column).
    Col(&'static str, &'static str),
    /// Page frontmatter tag set (JSON array in `pages.frontmatter`).
    PageTags,
    /// Block-level tag set (inline `#tag` in content / block properties "tags").
    BlockTags,
    /// Outgoing wiki-link targets of a page (from `links` table).
    PageLinks,
    /// Incoming wiki-link sources of a page (from `links` table).
    PageBacklinks,
    /// Block custom properties map (JSON object in `blocks.properties`).
    BlockProperties,
}

fn resolve_attr(attr: &str) -> Option<Attr> {
    Some(match attr {
        ":block/id" => Attr::Col("b", "id"),
        ":block/content" => Attr::Col("b", "content"),
        ":block/page" => Attr::Col("b", "page_path"),
        ":block/parent" => Attr::Col("b", "parent_id"),
        ":block/left" => Attr::Col("b", "left_id"),
        ":block/marker" => Attr::Col("b", "marker"),
        ":block/priority" => Attr::Col("b", "priority"),
        ":block/collapsed" => Attr::Col("b", "collapsed"),
        ":block/heading" => Attr::Col("b", "heading_level"),
        ":block/created" => Attr::Col("b", "created_at"),
        ":block/modified" => Attr::Col("b", "modified_at"),
        ":block/properties" => Attr::BlockProperties,
        ":block/tags" => Attr::BlockTags,
        ":page/path" => Attr::Col("p", "path"),
        ":page/title" => Attr::Col("p", "title"),
        ":page/tags" => Attr::PageTags,
        ":page/block_count" => Attr::Col("p", "block_count"),
        ":page/links" => Attr::PageLinks,
        ":page/backlinks" => Attr::PageBacklinks,
        ":page/modified" => Attr::Col("p", "modified_at"),
        ":page/created" => Attr::Col("p", "created_at"),
        _ => return None,
    })
}

/// Alias used by each special (non-plain-column) attribute, for table inclusion.
fn special_alias(attr: &Attr) -> &'static str {
    match attr {
        Attr::Col(a, _) => a,
        Attr::PageTags | Attr::PageLinks | Attr::PageBacklinks => "p",
        Attr::BlockTags | Attr::BlockProperties => "b",
    }
}

/// SQL expression selecting the attribute's value for `:find`/render purposes.
fn select_expr(attr: &Attr) -> String {
    match attr {
        Attr::Col(a, c) => format!("{a}.{c}"),
        Attr::PageTags => "json_extract(p.frontmatter, '$.tags')".to_string(),
        Attr::BlockTags => "b.properties".to_string(),
        Attr::PageLinks | Attr::PageBacklinks => "NULL".to_string(),
        Attr::BlockProperties => "b.properties".to_string(),
    }
}

/// Build a WHERE predicate for a special (set/map) attribute matched against a
/// literal value. Returns (predicate_sql, params_to_push).
fn special_where(
    attr: &Attr,
    value: &str,
    param_pos: usize,
) -> Result<(String, Vec<String>), CompileError> {
    match attr {
        Attr::PageTags => {
            // The page's frontmatter tag array contains `value`.
            Ok((
                format!(
                    "EXISTS (SELECT 1 FROM json_each(json_extract(p.frontmatter, '$.tags')) \
                     WHERE json_each.value = ?{param_pos})"
                ),
                vec![value.to_string()],
            ))
        }
        Attr::BlockTags => {
            // A block is "tagged" when its content carries `#tag` or its
            // properties carry a `tags` entry equal to the value.
            Ok((
                format!(
                    "(b.content LIKE ?{param_pos} \
                     OR EXISTS (SELECT 1 FROM json_each(b.properties) \
                                WHERE json_each.key = 'tags' AND json_each.value = ?{param_pos}))"
                ),
                vec![format!("#{value}"), value.to_string()],
            ))
        }
        Attr::BlockProperties => {
            // `:block/properties "key value"` — treat the literal as a
            // `key:value` pair or as a substring of the serialized map.
            if let Some((k, v)) = value.split_once(':') {
                Ok((
                    format!(
                        "json_extract(b.properties, ?{param_pos}) = ?{param_pos1}",
                        param_pos = param_pos,
                        param_pos1 = param_pos + 1
                    ),
                    vec![format!("$.{k}"), v.trim().to_string()],
                ))
            } else {
                Ok((
                    format!("b.properties LIKE ?{param_pos}"),
                    vec![format!("%{value}%")],
                ))
            }
        }
        Attr::PageLinks => {
            // Page `p` has an outgoing wiki-link targeting `value` (matched via
            // the links table, which stores the canonical target path or the
            // raw target string — both are matched).
            Ok((
                format!(
                    "EXISTS (SELECT 1 FROM links l \
                     JOIN blocks sb ON sb.id = l.source_block \
                     WHERE sb.page_path = p.path \
                       AND (l.target_page = ?{param_pos} \
                            OR l.target_page = ?{param_pos1}))",
                    param_pos = param_pos,
                    param_pos1 = param_pos + 1
                ),
                vec![value.to_string(), value.to_string()],
            ))
        }
        Attr::PageBacklinks => {
            // Page `p` is the target of an incoming wiki-link from the page
            // matching `value` (its path or the value as a raw target).
            Ok((
                format!(
                    "EXISTS (SELECT 1 FROM links l \
                     JOIN blocks sb ON sb.id = l.source_block \
                     WHERE l.target_page = p.path \
                       AND (sb.page_path = ?{param_pos} \
                            OR l.target_page = ?{param_pos1}))",
                    param_pos = param_pos,
                    param_pos1 = param_pos + 1
                ),
                vec![value.to_string(), value.to_string()],
            ))
        }
        _ => Err(CompileError::UnknownAttribute(String::new())),
    }
}

#[derive(Debug)]
pub enum CompileError {
    UnknownAttribute(String),
    UnresolvedVariable(String),
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownAttribute(a) => write!(f, "Unknown attribute: {a}"),
            Self::UnresolvedVariable(v) => write!(f, "Unresolved variable: {v}"),
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
        let attr = resolve_attr(&pattern.attribute)
            .ok_or_else(|| CompileError::UnknownAttribute(pattern.attribute.clone()))?;
        let alias = special_alias(&attr).to_string();

        // Cross-table reference handling for plain columns: if the entity var
        // is already bound to the other table, join via the page foreign key.
        if let Attr::Col(_a, c) = &attr {
            if pattern.entity.starts_with('?') {
                if let Some((prev_alias, _)) = var_map.get(&pattern.entity) {
                    let cross_table = (*prev_alias == "b" && alias == "p")
                        || (*prev_alias == "p" && alias == "b");
                    if cross_table && *prev_alias != alias {
                        conditions.push("b.page_path = p.path".to_string());
                    }
                }
                var_map.insert(pattern.entity.clone(), (alias.clone(), c.to_string()));
            }
            aliases.insert(alias.clone());
        } else {
            aliases.insert(alias.clone());
            if pattern.entity.starts_with('?') {
                var_map.insert(pattern.entity.clone(), (alias.clone(), select_expr(&attr)));
            }
        }

        if pattern.value == "_" {
            continue;
        }

        if let Attr::Col(a, c) = &attr {
            if pattern.value.starts_with('?') {
                if let Some((prev_alias, prev_col)) = var_map.get(&pattern.value) {
                    if prev_alias.as_str() != *a {
                        conditions.push(format!("{prev_alias}.{prev_col} = {a}.{c}"));
                    }
                }
                var_map.insert(pattern.value.clone(), (a.to_string(), c.to_string()));
            } else if pattern.value != "_" {
                conditions.push(format!("{a}.{c} = ?{}", params.len() + 1));
                params.push(pattern.value.clone());
            }
        } else {
            // Special (set/map) attribute with a literal value → containment test.
            let (pred, mut to_push) = special_where(&attr, &pattern.value, params.len() + 1)
                .map_err(|_| CompileError::UnknownAttribute(pattern.attribute.clone()))?;
            conditions.push(pred);
            params.append(&mut to_push);
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
            if col.starts_with('(') || col.starts_with("json_") {
                select_cols.push(format!("{col} AS \"{var}\""));
            } else {
                select_cols.push(format!("{alias}.{col} AS \"{var}\""));
            }
        } else if var.starts_with('?') {
            return Err(CompileError::UnresolvedVariable(var.clone()));
        } else if let Some(attr) = resolve_attr(var) {
            if let Attr::Col(a, c) = &attr {
                select_cols.push(format!("{a}.{c} AS \"{var}\""));
                aliases.insert(a.to_string());
            } else {
                select_cols.push(format!("\"{var}\" AS \"{var}\""));
                aliases.insert(special_alias(&attr).to_string());
            }
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
        let q = parse_query(
            r#"{:query [:find ?b ?content :where [?b :block/marker "TODO"] [?b :block/content ?content]]}"#,
        )
        .unwrap();
        let c = compile(&q).unwrap();
        assert!(c.sql.contains("b.content"));
        assert!(c.sql.contains("b.marker"));
    }

    #[test]
    fn test_compile_page_join() {
        let q = parse_query(
            r#"{:query [:find ?title :where [?b :block/marker "TODO"] [?b :block/page ?p] [?p :page/title ?title]]}"#,
        )
        .unwrap();
        let c = compile(&q).unwrap();
        assert!(c.sql.contains("pages p"));
        assert!(c.sql.contains("b.page_path = p.path"));
    }

    #[test]
    fn test_documented_page_modified_compiles() {
        // Regression for QY-02: the documented example previously errored with
        // "Unknown attribute: :page/modified".
        let q = parse_query(
            r#"{:query [:find ?page ?title ?modified :where [?page :page/title ?title] [?page :page/modified ?modified]]}"#,
        )
        .unwrap();
        let c = compile(&q).unwrap();
        assert!(c.sql.contains("p.modified_at"), "SQL: {}", c.sql);
    }

    #[test]
    fn test_documented_block_tags_compiles_and_filters() {
        let q = parse_query(
            r#"{:query [:find ?block ?content :where [?block :block/tags "project"] [?block :block/content ?content]]}"#,
        )
        .unwrap();
        let c = compile(&q).unwrap();
        assert!(c.sql.contains("json_each"), "SQL: {}", c.sql);
        assert!(c.params.contains(&"#project".to_string()));
    }

    #[test]
    fn test_documented_page_tags_compiles() {
        let q = parse_query(r#"{:query [:find ?page :where [?page :page/tags "rust"]]}"#).unwrap();
        let c = compile(&q).unwrap();
        // Page tags come from frontmatter — requires the pages table.
        assert!(c.sql.contains("json_extract(p.frontmatter, '$.tags')"));
    }

    #[test]
    fn test_documented_block_count_compiles() {
        let q = parse_query(
            r#"{:query [:find ?page ?title ?count :where [?page :page/title ?title] [?page :page/block_count ?count]]}"#,
        )
        .unwrap();
        let c = compile(&q).unwrap();
        assert!(c.sql.contains("p.block_count"), "SQL: {}", c.sql);
    }

    #[test]
    fn test_unknown_attr() {
        let q = parse_query(r#"{:query [:find ?b :where [?b :unknown/attr "x"]]}"#).unwrap();
        assert!(compile(&q).is_err());
    }
}
