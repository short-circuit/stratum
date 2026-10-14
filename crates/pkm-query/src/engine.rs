//! Datalog query engine against SQLite BlockStore.

use crate::compiler::{compile, CompiledQuery};
use crate::parser::parse_query;
use rusqlite::Connection;

#[derive(Debug)]
pub enum QueryError {
    Parse(String),
    Compile(String),
    Execute(String),
}

impl std::fmt::Display for QueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(s) => write!(f, "Parse error: {}", s),
            Self::Compile(s) => write!(f, "Compile error: {}", s),
            Self::Execute(s) => write!(f, "Execute error: {}", s),
        }
    }
}
impl std::error::Error for QueryError {}

#[derive(Debug, Clone)]
pub struct QueryRow {
    pub columns: Vec<String>,
    pub values: Vec<serde_json::Value>,
}

pub struct QueryEngine {
    conn: Connection,
}

impl QueryEngine {
    pub fn new(db_path: &str) -> Result<Self, QueryError> {
        let conn = Connection::open(db_path)
            .map_err(|e| QueryError::Execute(format!("Failed to open database: {}", e)))?;
        Ok(Self { conn })
    }

    pub fn execute(&self, datalog: &str) -> Result<Vec<QueryRow>, QueryError> {
        let query = parse_query(datalog).map_err(|e| QueryError::Parse(e.to_string()))?;
        let compiled = compile(&query).map_err(|e| QueryError::Compile(e.to_string()))?;
        self.execute_compiled(&compiled)
    }

    fn execute_compiled(&self, compiled: &CompiledQuery) -> Result<Vec<QueryRow>, QueryError> {
        let col_count = compiled
            .sql
            .split("FROM")
            .next()
            .unwrap_or("")
            .replace("SELECT ", "")
            .split(',')
            .count();

        let mut stmt = self.conn.prepare(&compiled.sql).map_err(|e| {
            QueryError::Execute(format!("Prepare failed: {} — SQL: {}", e, compiled.sql))
        })?;

        let params: Vec<&dyn rusqlite::types::ToSql> = compiled
            .params
            .iter()
            .map(|s| s as &dyn rusqlite::types::ToSql)
            .collect();

        let rows = stmt
            .query_map(params.as_slice(), move |row| {
                let mut values = Vec::new();
                for i in 0..col_count {
                    let val: Result<String, _> = row.get(i);
                    match val {
                        Ok(s) => values.push(serde_json::Value::String(s)),
                        Err(_) => values.push(serde_json::Value::Null),
                    }
                }
                let columns: Vec<String> = (0..col_count).map(|i| format!("col{}", i)).collect();
                Ok(QueryRow { columns, values })
            })
            .map_err(|e| QueryError::Execute(format!("Query failed: {}", e)))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| QueryError::Execute(format!("Row error: {}", e)))?);
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pkm_block::{Block, BlockStore, TaskMarker};
    use tempfile::TempDir;
    use uuid::Uuid;

    fn setup_test_engine() -> (TempDir, QueryEngine) {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("test.db");
        let store = BlockStore::open(&db_path).unwrap();

        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let block1 = Block::new(id1, "Buy groceries".into()).with_marker(TaskMarker::Todo);
        let block2 = Block::new(id2, "Completed task".into()).with_marker(TaskMarker::Done);

        store.insert_block(&block1, "pages/tasks.md").unwrap();
        store.insert_block(&block2, "pages/tasks.md").unwrap();
        drop(store);

        let engine = QueryEngine::new(&db_path.to_string_lossy()).unwrap();
        (tmp, engine)
    }

    #[test]
    fn test_find_blocks_by_marker_edn() {
        let (_tmp, engine) = setup_test_engine();
        let results = engine
            .execute(r#"{:query [:find ?b :where [?b :block/marker "TODO"]]}"#)
            .unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_find_blocks_by_marker_json() {
        let (_tmp, engine) = setup_test_engine();
        let results = engine
            .execute(r#"{"query": [":find", ["?b"], ":where", ["?b", ":block/marker", "TODO"]]}"#)
            .unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_find_with_content() {
        let (_tmp, engine) = setup_test_engine();
        let results = engine
            .execute(r#"{:query [:find ?b ?content :where [?b :block/marker "DONE"] [?b :block/content ?content]]}"#)
            .unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_empty_results() {
        let (_tmp, engine) = setup_test_engine();
        let results = engine
            .execute(r#"{:query [:find ?b :where [?b :block/marker "CANCELLED"]]}"#)
            .unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_invalid_syntax() {
        let (_tmp, engine) = setup_test_engine();
        assert!(engine.execute("not datalog").is_err());
    }

    #[test]
    fn test_page_join() {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("test.db");
        let store = BlockStore::open(&db_path).unwrap();

        let mut page = pkm_block::Page::new(
            std::path::PathBuf::from("/vault/pages/projects.md"),
            std::path::Path::new("/vault"),
        );
        page.frontmatter.title = Some("Projects".into());
        store.upsert_page(&page).unwrap();

        let id = Uuid::new_v4();
        let block = Block::new(id, "Build Stratum".into()).with_marker(TaskMarker::Doing);
        store.insert_block(&block, "pages/projects.md").unwrap();
        drop(store);

        let engine = QueryEngine::new(&db_path.to_string_lossy()).unwrap();
        let results = engine
            .execute(r#"{:query [:find ?title :where [?b :block/marker "DOING"] [?b :block/page ?p] [?p :page/title ?title]]}"#)
            .unwrap();
        assert!(!results.is_empty());
    }

    // ---------------------------------------------------------------------
    // E10.F1 SECTION A: predicate / or / not / order-by end-to-end execution.
    // The queries below are EXACTLY the shapes the frontend builder emits.
    // ---------------------------------------------------------------------

    /// Seed a vault with a deterministic dataset for extended-form tests:
    ///  * tasks page has two TODO blocks ("Buy groceries" prio A, "Ship v1" prio B)
    ///  * tasks page has a DONE block ("Completed task")
    ///  * notes page has one TODO block with a `project` tag
    fn setup_extended_engine() -> (TempDir, QueryEngine, Vec<String>) {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("test.db");
        let store = BlockStore::open(&db_path).unwrap();

        let ids: Vec<Uuid> = (0..4).map(|_| Uuid::new_v4()).collect();
        let b1 = Block::new(ids[0], "Buy groceries".into())
            .with_marker(TaskMarker::Todo)
            .with_priority(pkm_block::Priority::A);
        let b2 = Block::new(ids[1], "Ship v1".into())
            .with_marker(TaskMarker::Todo)
            .with_priority(pkm_block::Priority::B);
        let b3 = Block::new(ids[2], "Completed task".into()).with_marker(TaskMarker::Done);
        let b4 = Block::new(ids[3], "Ship v2 #project".into()).with_marker(TaskMarker::Todo);
        store.insert_block(&b1, "pages/tasks.md").unwrap();
        store.insert_block(&b2, "pages/tasks.md").unwrap();
        store.insert_block(&b3, "pages/tasks.md").unwrap();
        store.insert_block(&b4, "pages/notes.md").unwrap();

        // Upsert pages AFTER inserting blocks so `pages.block_count` reflects
        // the real number of blocks (needed by the `> ` predicate test).
        let mut tasks_page = pkm_block::Page::new(
            std::path::PathBuf::from("/vault/pages/tasks.md"),
            std::path::Path::new("/vault"),
        );
        tasks_page.frontmatter.title = Some("Tasks".into());
        tasks_page.set_blocks(&[b1.clone(), b2.clone(), b3.clone()]);
        store.upsert_page(&tasks_page).unwrap();

        let mut notes_page = pkm_block::Page::new(
            std::path::PathBuf::from("/vault/pages/notes.md"),
            std::path::Path::new("/vault"),
        );
        notes_page.frontmatter.title = Some("Notes".into());
        notes_page.set_blocks(&[b4.clone()]);
        store.upsert_page(&notes_page).unwrap();
        drop(store);

        let engine = QueryEngine::new(&db_path.to_string_lossy()).unwrap();
        let id_strings: Vec<String> = ids.iter().map(|u| u.to_string()).collect();
        (tmp, engine, id_strings)
    }

    #[test]
    fn test_exec_predicate_not_eq_marker() {
        let (_tmp, engine, ids) = setup_extended_engine();
        // Spec query 1: not= on marker.
        let results = engine
            .execute(
                r#"{:query [:find ?b ?content :where [?b :block/marker ?m] [(not= ?m "DONE")] [?b :block/content ?content]]}"#,
            )
            .unwrap();
        assert_eq!(
            results.len(),
            3,
            "three non-DONE blocks (2 TODO + tagged TODO)"
        );
        let contents: Vec<String> = results
            .iter()
            .map(|r| {
                r.values
                    .get(1)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string()
            })
            .collect();
        for c in &contents {
            assert_ne!(c, "Completed task", "DONE block must be excluded");
        }
        let _ = ids;
    }

    #[test]
    fn test_exec_predicate_includes_content() {
        let (_tmp, engine, _ids) = setup_extended_engine();
        // Spec query 2: contains on content.
        let results = engine
            .execute(
                r#"{:query [:find ?b :where [?b :block/content ?c] [(clojure.string/includes? ?c "groceries")]]}"#,
            )
            .unwrap();
        assert_eq!(results.len(), 1, "only the groceries block matches");
    }

    #[test]
    fn test_exec_predicate_gt_block_count() {
        let (_tmp, engine, _ids) = setup_extended_engine();
        // Spec query 3: gt on page block_count with a page join.
        // (Spec's snippet lists the predicate before the binding triple;
        // Datalog binds in clause order, so the binding triple must precede
        // the predicate that uses it — we use the binder-first order the
        // frontend builder emits.)
        let results = engine
            .execute(
                r#"{:query [:find ?b :where [?b :block/page ?p] [?p :page/block_count ?n] [(> ?n "0")]]}"#,
            )
            .unwrap();
        assert!(
            !results.is_empty(),
            "both pages have block_count 3/1 which is > 0"
        );
    }

    #[test]
    fn test_exec_predicate_gt_unbound_var_is_error() {
        let (_tmp, engine, _ids) = setup_extended_engine();
        // If the predicate appears before its binding (spec's literal order),
        // the var is unbound and compilation must error, not panic.
        let err = engine
            .execute(
                r#"{:query [:find ?b :where [?b :block/page ?p] [(> ?n "0")] [?p :page/block_count ?n]]}"#,
            )
            .unwrap_err();
        assert!(
            err.to_string().contains("Unresolved variable"),
            "got: {err}"
        );
    }

    #[test]
    fn test_exec_or_group() {
        let (_tmp, engine, _ids) = setup_extended_engine();
        // Spec query 4: or-group on markers.
        let results = engine
            .execute(
                r#"{:query [:find ?b :where (or [?b :block/marker "TODO"] [?b :block/marker "DOING"])]}"#,
            )
            .unwrap();
        assert_eq!(results.len(), 3, "three TODO blocks match the or-group");
    }

    #[test]
    fn test_exec_not_on_tags() {
        let (_tmp, engine, _ids) = setup_extended_engine();
        // Spec query 5: not on block/tags.
        let results = engine
            .execute(r#"{:query [:find ?b :where (not [?b :block/tags "project"])]}"#)
            .unwrap();
        assert_eq!(
            results.len(),
            3,
            "the #project-tagged block must be excluded"
        );
    }

    #[test]
    fn test_exec_order_by() {
        let (_tmp, engine, _ids) = setup_extended_engine();
        // Spec query 6: order-by execution — sort markers descending.
        let results = engine
            .execute(
                r#"{:query [:find ?b ?marker :where [?b :block/marker ?marker] :order-by [[?marker :desc]]]}"#,
            )
            .unwrap();
        assert!(
            results.len() >= 4,
            "all four blocks have markers, got {}",
            results.len()
        );
        // Markers lexically sorted DESC: TODO > DONE, so first row must be a TODO.
        let first_marker = results[0]
            .values
            .get(1)
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert_eq!(first_marker, "TODO");
    }
}
