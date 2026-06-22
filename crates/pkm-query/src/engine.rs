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
        let query = parse_query(datalog)
            .map_err(|e| QueryError::Parse(e.to_string()))?;
        let compiled = compile(&query)
            .map_err(|e| QueryError::Compile(e.to_string()))?;
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

        let mut stmt = self
            .conn
            .prepare(&compiled.sql)
            .map_err(|e| QueryError::Execute(format!("Prepare failed: {} — SQL: {}", e, compiled.sql)))?;

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
}
