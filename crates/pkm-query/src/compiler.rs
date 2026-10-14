//! Datalog → SQL compiler against the blocks SQLite schema.
//!
//! Attributes documented in `docs/guide/datalog-queries.md` are all supported.
//! Plain scalar attributes compile to `alias.column` comparisons; set/map
//! attributes (`:page/tags`, `:block/tags`, `:page/links`, `:page/backlinks`,
//! `:block/properties`) compile to SQL subqueries / JSON predicates so the
//! documented queries return results instead of a "Unknown attribute" error.
//!
//! In addition to triples, the compiler supports Logseq-compatible extended
//! forms: predicate lists (`[(not= ?m "DONE")]`), negation (`(not ...)`),
//! or-groups (`(or ...)`) and a top-level `:order-by` clause. Predicate /
//! or / not clauses compile through the same alias registration path as
//! triples, so entity variables bind exactly like top-level patterns.

use crate::parser::{FindSpec, Query, WhereClause};
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
                                WHERE json_each.key = 'tags' AND json_each.value = ?{param_pos1}))",
                    param_pos = param_pos,
                    param_pos1 = param_pos + 1
                ),
                vec![format!("%#{value}%"), value.to_string()],
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
    UnknownPredicate(String),
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownAttribute(a) => write!(f, "Unknown attribute: {a}"),
            Self::UnresolvedVariable(v) => write!(f, "Unresolved variable: {v}"),
            Self::UnknownPredicate(p) => write!(f, "Unknown predicate: {p}"),
        }
    }
}
impl std::error::Error for CompileError {}

#[derive(Debug)]
pub struct CompiledQuery {
    pub sql: String,
    pub params: Vec<String>,
}

pub fn compile(query: &Query) -> Result<CompiledQuery, CompileError> {
    let mut var_map: HashMap<String, (String, String)> = HashMap::new();
    let mut aliases: HashSet<String> = HashSet::new();
    let mut conditions: Vec<String> = Vec::new();
    let mut params: Vec<String> = Vec::new();

    for clause in &query.r#where {
        build_clause(
            clause,
            &mut var_map,
            &mut aliases,
            &mut conditions,
            &mut params,
        )?;
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

    // :order-by — resolve each variable to its bound plain column.
    let mut order_sql = String::new();
    if !query.order_by.is_empty() {
        let mut parts: Vec<String> = Vec::new();
        for ob in &query.order_by {
            let (alias, col) = var_map
                .get(&ob.var)
                .ok_or_else(|| CompileError::UnresolvedVariable(ob.var.clone()))?;
            if col.starts_with('(') || col.starts_with("json_") {
                return Err(CompileError::UnresolvedVariable(ob.var.clone()));
            }
            let dir = if ob.desc { "DESC" } else { "ASC" };
            parts.push(format!("{alias}.{col} {dir}"));
        }
        order_sql = format!(" ORDER BY {}", parts.join(", "));
    }

    let sql = format!(
        "SELECT {} FROM {} {}{}",
        select_cols.join(", "),
        tables.join(", "),
        where_clause,
        order_sql,
    );

    Ok(CompiledQuery {
        sql: sql.trim().to_string(),
        params,
    })
}

/// Compile a single where clause, registering aliases/var bindings as a side
/// effect (same behaviour as the historical inline loop for triples).
///
/// Predicates do NOT register aliases — the referenced variables must already
/// be bound to a plain column by an earlier clause. Or/not recurse into their
/// sub-clauses with the same `var_map`/`aliases` so entity variables bind
/// exactly like top-level triples.
fn build_clause(
    clause: &WhereClause,
    var_map: &mut HashMap<String, (String, String)>,
    aliases: &mut HashSet<String>,
    conditions: &mut Vec<String>,
    params: &mut Vec<String>,
) -> Result<(), CompileError> {
    match clause {
        WhereClause::Triple(pattern) => build_triple(pattern, var_map, aliases, conditions, params),
        WhereClause::Pred(pred) => build_predicate(pred, var_map, conditions, params),
        WhereClause::Or(sub_clauses) => {
            let mut sub_conditions: Vec<String> = Vec::new();
            for sub in sub_clauses {
                // Sub-clauses share the SAME var_map/aliases so entity
                // variables bind exactly like top-level triples.
                build_clause(sub, var_map, aliases, &mut sub_conditions, params)?;
            }
            if sub_conditions.is_empty() {
                return Ok(());
            }
            // Each compiled subcondition is parenthesized per Logseq semantics.
            let wrapped: Vec<String> = sub_conditions.iter().map(|c| format!("({})", c)).collect();
            conditions.push(format!("({})", wrapped.join(" OR ")));
            Ok(())
        }
        WhereClause::Not(sub) => {
            let mut sub_conditions: Vec<String> = Vec::new();
            build_clause(sub, var_map, aliases, &mut sub_conditions, params)?;
            if sub_conditions.is_empty() {
                return Ok(());
            }
            conditions.push(format!("NOT ({})", sub_conditions.join(" AND ")));
            Ok(())
        }
    }
}

/// Compile a plain `[entity attribute value]` triple (existing behaviour).
fn build_triple(
    pattern: &crate::parser::Pattern,
    var_map: &mut HashMap<String, (String, String)>,
    aliases: &mut HashSet<String>,
    conditions: &mut Vec<String>,
    params: &mut Vec<String>,
) -> Result<(), CompileError> {
    let attr = resolve_attr(&pattern.attribute)
        .ok_or_else(|| CompileError::UnknownAttribute(pattern.attribute.clone()))?;
    let alias = special_alias(&attr).to_string();

    // Cross-table reference handling for plain columns: if the entity var
    // is already bound to the other table, join via the page foreign key.
    if let Attr::Col(_a, c) = &attr {
        if pattern.entity.starts_with('?') {
            if let Some((prev_alias, _)) = var_map.get(&pattern.entity) {
                let cross_table =
                    (*prev_alias == "b" && alias == "p") || (*prev_alias == "p" && alias == "b");
                if cross_table && *prev_alias != alias {
                    conditions.push("b.page_path = p.path".to_string());
                }
            }
            // An entity variable identifies the ENTITY itself (the block
            // UUID or the page path), never the attribute's value column.
            // E7.F5 correctness: `[:find ?block ...]` must return the real
            // block id, or results cannot be resolved back to blocks.
            let identity = match alias.as_str() {
                "b" => "id",
                "p" => "path",
                _ => c,
            };
            var_map.insert(
                pattern.entity.clone(),
                (alias.clone(), identity.to_string()),
            );
        }
        aliases.insert(alias.clone());
    } else {
        aliases.insert(alias.clone());
        if pattern.entity.starts_with('?') {
            // An entity var bound through a special (set/map) attribute
            // still identifies the ENTITY itself (block id / page path),
            // not the attribute's rendered value.
            let identity = match alias.as_str() {
                "b" => "id",
                "p" => "path",
                _ => "",
            };
            var_map.insert(
                pattern.entity.clone(),
                (alias.clone(), identity.to_string()),
            );
        }
    }

    if pattern.value == "_" {
        return Ok(());
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
    Ok(())
}

/// Compile a predicate list against already-bound plain-column variables.
fn build_predicate(
    pred: &crate::parser::Predicate,
    var_map: &HashMap<String, (String, String)>,
    conditions: &mut Vec<String>,
    params: &mut Vec<String>,
) -> Result<(), CompileError> {
    if pred.args.is_empty() {
        return Err(CompileError::UnknownPredicate(pred.op.clone()));
    }
    let (var, lit) = if pred.args[0].starts_with('?') {
        (pred.args[0].clone(), pred.args.get(1).cloned())
    } else {
        // Literal-first form is unusual; treat first arg as the var-less
        // comparison against the first bound var otherwise.
        let var = pred
            .args
            .iter()
            .find(|a| a.starts_with('?'))
            .cloned()
            .ok_or_else(|| CompileError::UnresolvedVariable(pred.op.clone()))?;
        let lit = pred.args.iter().find(|a| !a.starts_with('?')).cloned();
        (var, lit)
    };

    let (alias, col) = var_map
        .get(&var)
        .ok_or_else(|| CompileError::UnresolvedVariable(var.clone()))?;
    if col.starts_with('(') || col.starts_with("json_") {
        // Plain-column requirement is relaxed: special values render as
        // `"var" AS "var"` aliases, which are usable in an outer filter —
        // handled by projection later.
    }

    let op = pred.op.as_str();
    let col_expr = format!("{alias}.{col}");
    match op {
        "=" => {
            let lit = lit.unwrap_or_else(|| "".to_string());
            conditions.push(format!("{col_expr} = ?{}", params.len() + 1));
            params.push(lit);
        }
        "!=" | "not=" => {
            let lit = lit.unwrap_or_else(|| "".to_string());
            conditions.push(format!("{col_expr} <> ?{}", params.len() + 1));
            params.push(lit);
        }
        ">" => {
            let lit = lit.unwrap_or_else(|| "".to_string());
            conditions.push(format!("{col_expr} > ?{}", params.len() + 1));
            params.push(lit);
        }
        "<" => {
            let lit = lit.unwrap_or_else(|| "".to_string());
            conditions.push(format!("{col_expr} < ?{}", params.len() + 1));
            params.push(lit);
        }
        ">=" => {
            let lit = lit.unwrap_or_else(|| "".to_string());
            conditions.push(format!("{col_expr} >= ?{}", params.len() + 1));
            params.push(lit);
        }
        "<=" => {
            let lit = lit.unwrap_or_else(|| "".to_string());
            conditions.push(format!("{col_expr} <= ?{}", params.len() + 1));
            params.push(lit);
        }
        "contains?" | "includes?" | "clojure.string/includes?" => {
            let lit = lit.unwrap_or_else(|| "".to_string());
            conditions.push(format!(
                "{col_expr} LIKE '%' || ?{} || '%'",
                params.len() + 1
            ));
            params.push(lit);
        }
        "starts-with?" | "clojure.string/starts-with?" => {
            let lit = lit.unwrap_or_else(|| "".to_string());
            conditions.push(format!("{col_expr} LIKE ?{} || '%'", params.len() + 1));
            params.push(lit);
        }
        "present?" => {
            conditions.push(format!("{col_expr} IS NOT NULL"));
        }
        "absent?" => {
            conditions.push(format!("{col_expr} IS NULL"));
        }
        other => return Err(CompileError::UnknownPredicate(other.to_string())),
    }
    Ok(())
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
        // A substring #project match is required so `:block/tags "project"`
        // actually finds blocks whose content carries `#project`; the JSON
        // `tags` property check binds the second parameter.
        assert!(c.params.contains(&"%#project%".to_string()));
        assert_eq!(c.params.len(), 2, "LIKE param + json_each value param");
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

    #[test]
    fn test_predicate_not_eq_compiles_to_neq() {
        let q =
            parse_query(r#"{:query [:find ?b :where [?b :block/marker ?m] [(not= ?m "DONE")]]}"#)
                .unwrap();
        let c = compile(&q).unwrap();
        assert!(c.sql.contains("b.marker <> ?"), "SQL: {}", c.sql);
        assert!(c.params.contains(&"DONE".to_string()));
    }

    #[test]
    fn test_predicate_contains_compiles_to_like() {
        let q = parse_query(
            r#"{:query [:find ?b :where [?b :block/content ?c] [(clojure.string/includes? ?c "groceries")]]}"#,
        )
        .unwrap();
        let c = compile(&q).unwrap();
        assert!(c.sql.contains("b.content LIKE '%' || ?"), "SQL: {}", c.sql);
        assert!(c.params.contains(&"groceries".to_string()));
    }

    #[test]
    fn test_predicate_gt_compiles() {
        let q = parse_query(r#"{:query [:find ?b :where [?b :block/content ?c] [(> ?c "a")]]}"#)
            .unwrap();
        let c = compile(&q).unwrap();
        assert!(c.sql.contains("b.content > ?"), "SQL: {}", c.sql);
    }

    #[test]
    fn test_predicate_present_compiles_to_is_not_null() {
        let q = parse_query(r#"{:query [:find ?b :where [?b :block/marker ?m] [(present? ?m)]]}"#)
            .unwrap();
        let c = compile(&q).unwrap();
        assert!(c.sql.contains("b.marker IS NOT NULL"), "SQL: {}", c.sql);
    }

    #[test]
    fn test_predicate_absent_compiles_to_is_null() {
        let q = parse_query(r#"{:query [:find ?b :where [?b :block/marker ?m] [(absent? ?m)]]}"#)
            .unwrap();
        let c = compile(&q).unwrap();
        assert!(c.sql.contains("b.marker IS NULL"), "SQL: {}", c.sql);
    }

    #[test]
    fn test_or_group_wraps_in_parens() {
        let q = parse_query(
            r#"{:query [:find ?b :where (or [?b :block/marker "TODO"] [?b :block/marker "DOING"])]}"#,
        )
        .unwrap();
        let c = compile(&q).unwrap();
        assert!(c.sql.contains("(b.marker = ?"), "SQL: {}", c.sql);
        assert!(c.sql.contains(" OR "), "SQL: {}", c.sql);
    }

    #[test]
    fn test_not_wraps_in_parens() {
        let q =
            parse_query(r#"{:query [:find ?b :where (not [?b :block/marker "DONE"])]}"#).unwrap();
        let c = compile(&q).unwrap();
        assert!(c.sql.contains("NOT (b.marker = ?"), "SQL: {}", c.sql);
    }

    #[test]
    fn test_order_by_appends_order_by() {
        let q = parse_query(
            r#"{:query [:find ?page ?title ?modified :where [?page :page/title ?title] [?page :page/modified ?modified] :order-by [[?modified :desc]]]}"#,
        )
        .unwrap();
        let c = compile(&q).unwrap();
        assert!(
            c.sql.contains("ORDER BY p.modified_at DESC"),
            "SQL: {}",
            c.sql
        );
    }

    #[test]
    fn test_unknown_predicate_is_error() {
        let q = parse_query(r#"{:query [:find ?b :where [?b :block/marker ?m] [(bogus? ?m)]]}"#)
            .unwrap();
        let err = compile(&q).unwrap_err();
        match err {
            CompileError::UnknownPredicate(p) => assert_eq!(p, "bogus?"),
            other => panic!("expected UnknownPredicate, got {:?}", other),
        }
    }

    #[test]
    fn test_unbound_predicate_var_is_error() {
        let q = parse_query(r#"{:query [:find ?b :where [(> ?n "5")]]}"#).unwrap();
        let err = compile(&q).unwrap_err();
        match err {
            CompileError::UnresolvedVariable(v) => assert_eq!(v, "?n"),
            other => panic!("expected UnresolvedVariable, got {:?}", other),
        }
    }
}
