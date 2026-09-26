//! Datalog query commands.

use crate::commands::vault::AppState;
use pkm_query::engine::QueryEngine;
use pkm_query::saved_queries::{self, SavedQuery};
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

/// List all saved Datalog queries.
///
/// A missing or corrupt `.pkm/saved_queries.json` is treated as an empty list.
#[tauri::command]
pub async fn list_saved_queries(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<SavedQuery>, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    saved_queries::load_saved_queries(&state.vault_path).map_err(|e| e.to_string())
}

/// Save (create or overwrite) a named Datalog query. The change is staged via
/// the auto-commit engine so it syncs with the vault.
#[tauri::command]
pub async fn save_saved_query(
    name: String,
    query: String,
    state: tauri::State<'_, AppState>,
) -> Result<SavedQuery, String> {
    let mut state = state.lock().map_err(|e| e.to_string())?;
    let saved = saved_queries::save_saved_query(&state.vault_path, &name, &query)
        .map_err(|e| e.to_string())?;
    state.record_change(saved_queries::SAVED_QUERIES_REL_PATH);
    Ok(saved)
}

/// Rename an existing saved query.
#[tauri::command]
pub async fn rename_saved_query(
    old_name: String,
    new_name: String,
    state: tauri::State<'_, AppState>,
) -> Result<SavedQuery, String> {
    let mut state = state.lock().map_err(|e| e.to_string())?;
    let saved = saved_queries::rename_saved_query(&state.vault_path, &old_name, &new_name)
        .map_err(|e| e.to_string())?;
    state.record_change(saved_queries::SAVED_QUERIES_REL_PATH);
    Ok(saved)
}

/// Delete a saved query by name.
#[tauri::command]
pub async fn delete_saved_query(
    name: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let mut state = state.lock().map_err(|e| e.to_string())?;
    saved_queries::delete_saved_query(&state.vault_path, &name).map_err(|e| e.to_string())?;
    state.record_change(saved_queries::SAVED_QUERIES_REL_PATH);
    Ok(())
}
