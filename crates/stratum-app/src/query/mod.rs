//! Datalog query commands: execute a query, manage saved queries, port the
//! visual filter builder, and format saved-query timestamps.
//!
//! The command methods mirror the original IPC surface. [`builder`] owns the
//! pure visual-builder port (field whitelist, EDN emission and round-trip
//! parsing); [`display`] owns timestamp formatting.

pub mod builder;
pub mod display;

use crate::state::StratumService;
use pkm_query::engine::QueryEngine;
use pkm_query::saved_queries::{self, SavedQuery};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QueryResultDto {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

impl StratumService {
    /// Execute a Datalog query against the vault database.
    pub async fn run_query(&self, datalog: String) -> Result<QueryResultDto, String> {
        let state = self.lock()?;
        let db_path = state.db_path.to_string_lossy().to_string();
        let engine = QueryEngine::new(&db_path).map_err(|e| e.to_string())?;
        let results = engine.execute(&datalog).map_err(|e| e.to_string())?;

        if results.is_empty() {
            return Ok(QueryResultDto {
                columns: Vec::new(),
                rows: Vec::new(),
            });
        }

        let columns: Vec<String> = results[0]
            .columns
            .iter()
            .enumerate()
            .map(|(i, _)| format!("col{}", i))
            .collect();

        let rows: Vec<Vec<String>> = results
            .iter()
            .map(|r| {
                r.values
                    .iter()
                    .map(|v| match v {
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    })
                    .collect()
            })
            .collect();

        Ok(QueryResultDto { columns, rows })
    }

    /// List all saved Datalog queries.
    ///
    /// A missing or corrupt `.pkm/saved_queries.json` is treated as an empty list.
    pub async fn list_saved_queries(&self) -> Result<Vec<SavedQuery>, String> {
        let state = self.lock()?;
        saved_queries::load_saved_queries(&state.vault_path).map_err(|e| e.to_string())
    }

    /// Save (create or overwrite) a named Datalog query. The change is staged via
    /// the auto-commit engine so it syncs with the vault.
    pub async fn save_saved_query(
        &self,
        name: String,
        query: String,
    ) -> Result<SavedQuery, String> {
        let mut state = self.lock()?;
        let saved = saved_queries::save_saved_query(&state.vault_path, &name, &query)
            .map_err(|e| e.to_string())?;
        state.record_change(saved_queries::SAVED_QUERIES_REL_PATH);
        Ok(saved)
    }

    /// Rename an existing saved query.
    pub async fn rename_saved_query(
        &self,
        old_name: String,
        new_name: String,
    ) -> Result<SavedQuery, String> {
        let mut state = self.lock()?;
        let saved = saved_queries::rename_saved_query(&state.vault_path, &old_name, &new_name)
            .map_err(|e| e.to_string())?;
        state.record_change(saved_queries::SAVED_QUERIES_REL_PATH);
        Ok(saved)
    }

    /// Delete a saved query by name.
    pub async fn delete_saved_query(&self, name: String) -> Result<(), String> {
        let mut state = self.lock()?;
        saved_queries::delete_saved_query(&state.vault_path, &name).map_err(|e| e.to_string())?;
        state.record_change(saved_queries::SAVED_QUERIES_REL_PATH);
        Ok(())
    }
}
