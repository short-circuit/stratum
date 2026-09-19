//! Tool definitions (contract §5).
//!
//! Each tool declares its exact `input_schema` (from the NORMATIVE contract in
//! `docs/advanced/mcp.md` §5) and the scope it requires (§6). The schemas are
//! the **binding** contract: the server MUST reject inputs that fail schema
//! validation with `InvalidArgs`, and `tools/list` MUST return exactly these
//! input schemas.

/// A registered tool definition.
#[derive(Debug, Clone)]
pub struct ToolDef {
    /// Contract tool name (`kb_*`).
    pub name: &'static str,
    /// Short human description (from the schema `title`).
    pub title: &'static str,
    /// Required scope (§6 mapping table).
    pub scope: &'static str,
    /// Binding input JSON Schema (§5).
    pub input_schema: serde_json::Value,
    /// Tool definition for rmcp.
    pub tool: rmcp::model::Tool,
}

/// Build an rmcp `Tool` from a name, description and input schema.
fn mk_tool(
    name: &'static str,
    title: &'static str,
    schema: serde_json::Value,
) -> rmcp::model::Tool {
    // The rmcp `Tool` struct is `#[non_exhaustive]`, so we build it through the
    // public constructor. We must convert the binding JSON Schema into the
    // `JsonObject` (serde_json::Map) the framework expects; a non-object
    // schema is normalized to an empty object so clients still receive a
    // usable (unconstrained) input schema.
    let input_schema: serde_json::Map<String, serde_json::Value> =
        serde_json::from_value(schema).unwrap_or_default();
    let mut tool =
        rmcp::model::Tool::new(name, title.to_string(), std::sync::Arc::new(input_schema));
    tool.title = Some(title.to_string());
    tool
}

pub const SCOPE_READ: &str = "kb:read";
pub const SCOPE_WRITE: &str = "kb:write";
pub const SCOPE_INDEX: &str = "kb:index";
pub const SCOPE_SEARCH: &str = "kb:search";
pub const SCOPE_LINK: &str = "kb:link";
pub const SCOPE_ORGANIZE: &str = "kb:organize";
pub const SCOPE_ADMIN: &str = "kb:admin";

/// All registered tools with their binding schemas and scopes.
#[allow(clippy::vec_init_then_push)] // progressive pushes keep each tool def readable
pub fn all_tools() -> Vec<ToolDef> {
    let mut v = Vec::new();

    v.push(ToolDef {
        name: "kb_get_page",
        title: "Read a note by its vault-relative path",
        scope: SCOPE_READ,
        input_schema: schema_get_page(),
        tool: mk_tool(
            "kb_get_page",
            "Read a note by its vault-relative path",
            schema_get_page(),
        ),
    });

    v.push(ToolDef {
        name: "kb_list_pages",
        title: "List notes in the knowledge base with pagination",
        scope: SCOPE_READ,
        input_schema: schema_list_pages(),
        tool: mk_tool(
            "kb_list_pages",
            "List notes in the knowledge base with pagination",
            schema_list_pages(),
        ),
    });

    v.push(ToolDef {
        name: "kb_write_page",
        title: "Write a note atomically",
        scope: SCOPE_WRITE,
        input_schema: schema_write_page(),
        tool: mk_tool(
            "kb_write_page",
            "Write a note atomically",
            schema_write_page(),
        ),
    });

    v.push(ToolDef {
        name: "kb_delete_page",
        title: "Delete a note and its index entries",
        scope: SCOPE_WRITE,
        input_schema: schema_delete_page(),
        tool: mk_tool(
            "kb_delete_page",
            "Delete a note and its index entries",
            schema_delete_page(),
        ),
    });

    v.push(ToolDef {
        name: "kb_reindex",
        title: "Reindex the knowledge base (rebuild or incremental)",
        scope: SCOPE_INDEX,
        input_schema: schema_reindex(),
        tool: mk_tool(
            "kb_reindex",
            "Reindex the knowledge base (rebuild or incremental)",
            schema_reindex(),
        ),
    });

    v.push(ToolDef {
        name: "kb_index_status",
        title: "Return index freshness and coverage",
        scope: SCOPE_READ,
        input_schema: schema_index_status(),
        tool: mk_tool(
            "kb_index_status",
            "Return index freshness and coverage",
            schema_index_status(),
        ),
    });

    v.push(ToolDef {
        name: "kb_search",
        title: "Full-text search across blocks",
        scope: SCOPE_SEARCH,
        input_schema: schema_search(),
        tool: mk_tool(
            "kb_search",
            "Full-text search across blocks",
            schema_search(),
        ),
    });

    v.push(ToolDef {
        name: "kb_search_by_tag",
        title: "Search notes and blocks by tag",
        scope: SCOPE_SEARCH,
        input_schema: schema_search_by_tag(),
        tool: mk_tool(
            "kb_search_by_tag",
            "Search notes and blocks by tag",
            schema_search_by_tag(),
        ),
    });

    v.push(ToolDef {
        name: "kb_autocomplete",
        title: "Suggest pages, tags, or backlink targets matching a query",
        scope: SCOPE_SEARCH,
        input_schema: schema_autocomplete(),
        tool: mk_tool(
            "kb_autocomplete",
            "Suggest pages, tags, or backlink targets matching a query",
            schema_autocomplete(),
        ),
    });

    v.push(ToolDef {
        name: "kb_backlinks",
        title: "Find backlinks (linked + unlinked mentions) for a note",
        scope: SCOPE_READ,
        input_schema: schema_backlinks(),
        tool: mk_tool(
            "kb_backlinks",
            "Find backlinks (linked + unlinked mentions) for a note",
            schema_backlinks(),
        ),
    });

    v.push(ToolDef {
        name: "kb_graph",
        title: "Return graph data for a note (or the whole graph)",
        scope: SCOPE_READ,
        input_schema: schema_graph(),
        tool: mk_tool(
            "kb_graph",
            "Return graph data for a note (or the whole graph)",
            schema_graph(),
        ),
    });

    v.push(ToolDef {
        name: "kb_resolve_link",
        title: "Resolve a [[wiki-link]] target to a note path",
        scope: SCOPE_READ,
        input_schema: schema_resolve_link(),
        tool: mk_tool(
            "kb_resolve_link",
            "Resolve a [[wiki-link]] target to a note path",
            schema_resolve_link(),
        ),
    });

    v.push(ToolDef {
        name: "kb_add_tag",
        title: "Add a tag to a note",
        scope: SCOPE_ORGANIZE,
        input_schema: schema_tag_mutation(),
        tool: mk_tool("kb_add_tag", "Add a tag to a note", schema_tag_mutation()),
    });

    v.push(ToolDef {
        name: "kb_remove_tag",
        title: "Remove a tag from a note",
        scope: SCOPE_ORGANIZE,
        input_schema: schema_tag_mutation(),
        tool: mk_tool(
            "kb_remove_tag",
            "Remove a tag from a note",
            schema_tag_mutation(),
        ),
    });

    v.push(ToolDef {
        name: "kb_vault_info",
        title: "Return knowledge base metadata and health",
        scope: SCOPE_READ,
        input_schema: schema_vault_info(),
        tool: mk_tool(
            "kb_vault_info",
            "Return knowledge base metadata and health",
            schema_vault_info(),
        ),
    });

    v
}

/// Look up a tool by name.
pub fn find_tool(name: &str) -> Option<ToolDef> {
    all_tools().into_iter().find(|t| t.name == name)
}

fn schema_base(
    title: &'static str,
    props: serde_json::Value,
    required: Vec<&str>,
) -> serde_json::Value {
    let mut required_arr = serde_json::Value::Array(Vec::new());
    if !required.is_empty() {
        required_arr = serde_json::Value::Array(
            required
                .iter()
                .map(|r| serde_json::Value::String(r.to_string()))
                .collect(),
        );
    }
    serde_json::json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": title,
        "type": "object",
        "properties": props,
        "required": required_arr,
        "additionalProperties": false
    })
}

fn schema_get_page() -> serde_json::Value {
    schema_base(
        "Read a note by its vault-relative path",
        serde_json::json!({
            "path": { "type": "string", "description": "Vault-relative markdown path, e.g. 'projects/example.md'." }
        }),
        vec!["path"],
    )
}

fn schema_list_pages() -> serde_json::Value {
    schema_base(
        "List notes in the knowledge base with pagination",
        serde_json::json!({
            "limit": { "type": "integer", "minimum": 1, "maximum": 1000, "default": 100 },
            "cursor": { "type": "string", "description": "Opaque continuation token from a previous response." }
        }),
        vec![],
    )
}

fn schema_write_page() -> serde_json::Value {
    schema_base(
        "Write a note atomically",
        serde_json::json!({
            "path": { "type": "string", "description": "Vault-relative markdown path, e.g. 'projects/example.md'." },
            "content": { "type": "string", "description": "Full markdown body (including any frontmatter to preserve/insert)." },
            "expected_modified": { "type": ["string", "null"], "description": "RFC 3339 timestamp; if provided and the on-disk note has a newer modified time, the write fails with CONFLICT (-32002)." }
        }),
        vec!["path", "content"],
    )
}

fn schema_delete_page() -> serde_json::Value {
    schema_base(
        "Delete a note and its index entries",
        serde_json::json!({
            "path": { "type": "string", "description": "Vault-relative markdown path to delete." },
            "expected_modified": { "type": ["string", "null"], "description": "Optional precondition guard; fails with CONFLICT if the note changed since this timestamp." }
        }),
        vec!["path"],
    )
}

fn schema_reindex() -> serde_json::Value {
    schema_base(
        "Reindex the knowledge base (rebuild or incremental)",
        serde_json::json!({
            "mode": { "type": "string", "enum": ["incremental", "rebuild"], "default": "incremental", "description": "incremental refreshes changed pages; rebuild re-indexes the entire vault (exclusive under advisory lock)." }
        }),
        vec![],
    )
}

fn schema_index_status() -> serde_json::Value {
    schema_base(
        "Return index freshness and coverage",
        serde_json::json!({}),
        vec![],
    )
}

fn schema_search() -> serde_json::Value {
    schema_base(
        "Full-text search across blocks",
        serde_json::json!({
            "query": { "type": "string", "minLength": 1 },
            "limit": { "type": "integer", "minimum": 1, "maximum": 1000, "default": 20 },
            "offset": { "type": "integer", "minimum": 0, "default": 0 }
        }),
        vec!["query"],
    )
}

fn schema_search_by_tag() -> serde_json::Value {
    schema_base(
        "Search notes and blocks by tag",
        serde_json::json!({
            "tag": { "type": "string", "minLength": 1 },
            "limit": { "type": "integer", "minimum": 1, "maximum": 1000, "default": 50 }
        }),
        vec!["tag"],
    )
}

fn schema_autocomplete() -> serde_json::Value {
    schema_base(
        "Suggest pages, tags, or backlink targets matching a query",
        serde_json::json!({
            "query": { "type": "string", "minLength": 1 },
            "kind": { "type": "string", "enum": ["page", "tag", "backlink"], "default": "page" },
            "limit": { "type": "integer", "minimum": 1, "maximum": 100, "default": 10 }
        }),
        vec!["query"],
    )
}

fn schema_backlinks() -> serde_json::Value {
    schema_base(
        "Find backlinks (linked + unlinked mentions) for a note",
        serde_json::json!({
            "path": { "type": "string" },
            "include_unlinked": { "type": "boolean", "default": true }
        }),
        vec!["path"],
    )
}

fn schema_graph() -> serde_json::Value {
    schema_base(
        "Return graph data for a note (or the whole graph)",
        serde_json::json!({
            "path": { "type": ["string", "null"], "description": "When provided, only the subgraph of nodes within 'depth' hops of this note. When null, the whole graph." },
            "depth": { "type": "integer", "minimum": 1, "maximum": 5, "default": 2 }
        }),
        vec![],
    )
}

fn schema_resolve_link() -> serde_json::Value {
    schema_base(
        "Resolve a [[wiki-link]] target to a note path",
        serde_json::json!({
            "target": { "type": "string", "minLength": 1 }
        }),
        vec!["target"],
    )
}

fn schema_tag_mutation() -> serde_json::Value {
    schema_base(
        "Add or remove a tag on a note",
        serde_json::json!({
            "path": { "type": "string" },
            "tag": { "type": "string", "minLength": 1, "pattern": "^[a-zA-Z0-9_.\\-/#]+$" }
        }),
        vec!["path", "tag"],
    )
}

fn schema_vault_info() -> serde_json::Value {
    schema_base(
        "Return knowledge base metadata and health",
        serde_json::json!({}),
        vec![],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_count_matches_contract() {
        assert_eq!(all_tools().len(), 15);
    }

    #[test]
    fn test_scope_mapping_matches_contract() {
        let tools = all_tools();
        let by_name: std::collections::HashMap<_, _> =
            tools.iter().map(|t| (t.name, t.scope)).collect();
        assert_eq!(by_name["kb_get_page"], "kb:read");
        assert_eq!(by_name["kb_list_pages"], "kb:read");
        assert_eq!(by_name["kb_write_page"], "kb:write");
        assert_eq!(by_name["kb_delete_page"], "kb:write");
        assert_eq!(by_name["kb_reindex"], "kb:index");
        assert_eq!(by_name["kb_index_status"], "kb:read");
        assert_eq!(by_name["kb_search"], "kb:search");
        assert_eq!(by_name["kb_search_by_tag"], "kb:search");
        assert_eq!(by_name["kb_autocomplete"], "kb:search");
        assert_eq!(by_name["kb_backlinks"], "kb:read");
        assert_eq!(by_name["kb_graph"], "kb:read");
        assert_eq!(by_name["kb_resolve_link"], "kb:read");
        assert_eq!(by_name["kb_add_tag"], "kb:organize");
        assert_eq!(by_name["kb_remove_tag"], "kb:organize");
        assert_eq!(by_name["kb_vault_info"], "kb:read");
    }

    #[test]
    fn test_each_schema_parses_as_json_schema() {
        for t in all_tools() {
            let v = serde_json::to_value(&t.input_schema).unwrap();
            assert!(v.is_object(), "{} schema must be an object", t.name);
            assert!(v.get("type").is_some());
            assert!(v.get("additionalProperties") == Some(&serde_json::Value::Bool(false)));
        }
    }

    #[test]
    fn test_schema_enum_and_required() {
        let reindex = schema_reindex();
        assert_eq!(reindex["properties"]["mode"]["enum"][0], "incremental");
        assert_eq!(reindex["properties"]["mode"]["enum"][1], "rebuild");
        let get_page = schema_get_page();
        assert!(get_page["required"]
            .as_array()
            .unwrap()
            .contains(&serde_json::Value::String("path".into())));
    }

    #[test]
    fn test_tool_names_unique() {
        let names: Vec<&str> = all_tools().iter().map(|t| t.name).collect();
        let mut uniq = names.clone();
        uniq.dedup();
        assert_eq!(names.len(), uniq.len());
    }
}
