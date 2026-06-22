//! Datalog query commands.

use crate::commands::vault::AppState;
use pkm_query::engine::QueryEngine;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QueryResultDto {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

#[tauri::command]
pub async fn run_query(
    datalog: String,
    state: tauri::State<'_, AppState>,
) -> Result<QueryResultDto, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
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
