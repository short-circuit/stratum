//! Implementation of the block store: block CRUD.

use super::BlockStore;
use super::StoreResult;
use crate::block::{Block, BlockId, BlockMeta, Priority, TaskMarker};
use chrono::{DateTime, Utc};
use pkm_core::PkmError;
use rusqlite::{params, params_from_iter};
use std::collections::{BTreeMap, HashMap};
use uuid::Uuid;

impl BlockStore {
    pub fn insert_block(&self, block: &Block, page_path: &str) -> StoreResult<()> {
        let id = block.id.to_string();
        let properties = serde_json::to_string(&block.properties)?;
        let marker = block.marker.map(|m| m.as_str().to_string());
        let priority = block.priority.map(|p| p.as_str().to_string());
        let parent_id = block.parent_id.map(|p| p.to_string());
        let left_id = block.left_id.map(|l| l.to_string());
        let created_at = block.created_at.to_rfc3339();
        let modified_at = block.modified_at.to_rfc3339();

        self.conn
            .execute(
                "INSERT OR REPLACE INTO blocks (id, page_path, content, parent_id, left_id,
             properties, marker, priority, collapsed, heading_level, created_at, modified_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    id,
                    page_path,
                    block.content,
                    parent_id,
                    left_id,
                    properties,
                    marker,
                    priority,
                    block.meta.collapsed as i32,
                    block.meta.heading_level,
                    created_at,
                    modified_at,
                ],
            )
            .map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?;
        Ok(())
    }

    pub fn get_block(&self, id: BlockId) -> StoreResult<Block> {
        let id_str = id.to_string();
        self.conn
            .query_row(
                "SELECT id, content, parent_id, left_id, properties, marker, priority,
                 collapsed, heading_level, created_at, modified_at
                 FROM blocks WHERE id = ?1",
                params![id_str],
                |row| {
                    let id: String = row.get(0)?;
                    let content: String = row.get(1)?;
                    let parent_id: Option<String> = row.get(2)?;
                    let left_id: Option<String> = row.get(3)?;
                    let properties_str: String = row.get(4)?;
                    let marker: Option<String> = row.get(5)?;
                    let priority: Option<String> = row.get(6)?;
                    let collapsed: bool = row.get::<_, i32>(7)? != 0;
                    let heading_level: Option<u8> = row.get(8)?;
                    let created_at: String = row.get(9)?;
                    let modified_at: String = row.get(10)?;

                    let id = Uuid::parse_str(&id).unwrap_or_else(|_| Uuid::nil());
                    let properties: BTreeMap<String, String> =
                        serde_json::from_str(&properties_str).unwrap_or_default();
                    let marker = marker.and_then(|m| TaskMarker::parse(&m));
                    let priority = priority.and_then(|p| Priority::parse(&p));

                    Ok(Block {
                        id,
                        content,
                        parent_id: parent_id.and_then(|s| Uuid::parse_str(&s).ok()),
                        left_id: left_id.and_then(|s| Uuid::parse_str(&s).ok()),
                        properties,
                        marker,
                        priority,
                        meta: BlockMeta {
                            collapsed,
                            heading_level,
                        },
                        created_at: DateTime::parse_from_rfc3339(&created_at)
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now()),
                        modified_at: DateTime::parse_from_rfc3339(&modified_at)
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now()),
                    })
                },
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => PkmError::BlockNotFound(format!("{}", id)),
                other => PkmError::Internal(format!("SQLite error: {other}")),
            })
    }

    /// Get a block by ID along with its page_path.
    /// Returns (Block, page_path) or an error if the block is not found.
    pub fn get_block_with_page_path(&self, id: BlockId) -> StoreResult<(Block, String)> {
        let id_str = id.to_string();
        self.conn
            .query_row(
                "SELECT id, content, parent_id, left_id, properties, marker, priority,
                 collapsed, heading_level, created_at, modified_at, page_path
                 FROM blocks WHERE id = ?1",
                params![id_str],
                |row| {
                    let id: String = row.get(0)?;
                    let content: String = row.get(1)?;
                    let parent_id: Option<String> = row.get(2)?;
                    let left_id: Option<String> = row.get(3)?;
                    let properties_str: String = row.get(4)?;
                    let marker: Option<String> = row.get(5)?;
                    let priority: Option<String> = row.get(6)?;
                    let collapsed: bool = row.get::<_, i32>(7)? != 0;
                    let heading_level: Option<u8> = row.get(8)?;
                    let created_at: String = row.get(9)?;
                    let modified_at: String = row.get(10)?;
                    let page_path: String = row.get(11)?;

                    let id = Uuid::parse_str(&id).unwrap_or_else(|_| Uuid::nil());
                    let properties: BTreeMap<String, String> =
                        serde_json::from_str(&properties_str).unwrap_or_default();
                    let marker = marker.and_then(|m| TaskMarker::parse(&m));
                    let priority = priority.and_then(|p| Priority::parse(&p));

                    Ok((
                        Block {
                            id,
                            content,
                            parent_id: parent_id.and_then(|s| Uuid::parse_str(&s).ok()),
                            left_id: left_id.and_then(|s| Uuid::parse_str(&s).ok()),
                            properties,
                            marker,
                            priority,
                            meta: BlockMeta {
                                collapsed,
                                heading_level,
                            },
                            created_at: DateTime::parse_from_rfc3339(&created_at)
                                .map(|dt| dt.with_timezone(&Utc))
                                .unwrap_or_else(|_| Utc::now()),
                            modified_at: DateTime::parse_from_rfc3339(&modified_at)
                                .map(|dt| dt.with_timezone(&Utc))
                                .unwrap_or_else(|_| Utc::now()),
                        },
                        page_path,
                    ))
                },
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => PkmError::BlockNotFound(format!("{}", id)),
                other => PkmError::Internal(format!("SQLite error: {other}")),
            })
    }

    #[allow(clippy::type_complexity)]
    pub fn get_blocks_by_page(&self, page_path: &str) -> StoreResult<Vec<Block>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, content, parent_id, left_id, properties, marker, priority,
                 collapsed, heading_level, created_at, modified_at
                 FROM blocks WHERE page_path = ?1 ORDER BY rowid",
            )
            .map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?;
        let blocks: Vec<Block> = stmt
            .query_map(params![page_path], |row| {
                let id: String = row.get(0)?;
                let content: String = row.get(1)?;
                let parent_id: Option<String> = row.get(2)?;
                let left_id: Option<String> = row.get(3)?;
                let properties_str: String = row.get(4)?;
                let marker: Option<String> = row.get(5)?;
                let priority: Option<String> = row.get(6)?;
                let collapsed: bool = row.get::<_, i32>(7)? != 0;
                let heading_level: Option<u8> = row.get(8)?;
                let created_at: String = row.get(9)?;
                let modified_at: String = row.get(10)?;

                let id = Uuid::parse_str(&id).unwrap_or_else(|_| Uuid::nil());
                let properties: BTreeMap<String, String> =
                    serde_json::from_str(&properties_str).unwrap_or_default();
                let marker = marker.and_then(|m| TaskMarker::parse(&m));
                let priority = priority.and_then(|p| Priority::parse(&p));

                Ok(Block {
                    id,
                    content,
                    parent_id: parent_id.and_then(|s| Uuid::parse_str(&s).ok()),
                    left_id: left_id.and_then(|s| Uuid::parse_str(&s).ok()),
                    properties,
                    marker,
                    priority,
                    meta: BlockMeta {
                        collapsed,
                        heading_level,
                    },
                    created_at: DateTime::parse_from_rfc3339(&created_at)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    modified_at: DateTime::parse_from_rfc3339(&modified_at)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            })
            .map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(blocks)
    }

    /// Batch version of `get_blocks_by_page`. Returns a map from page_path to its
    /// blocks for all requested pages in a single query.
    pub fn get_blocks_by_pages(
        &self,
        page_paths: &[String],
    ) -> StoreResult<HashMap<String, Vec<Block>>> {
        if page_paths.is_empty() {
            return Ok(HashMap::new());
        }

        let placeholders: Vec<String> = page_paths.iter().map(|_| "?".to_string()).collect();
        let sql = format!(
            "SELECT id, content, parent_id, left_id, properties, marker, priority, \
             collapsed, heading_level, created_at, modified_at, page_path \
             FROM blocks WHERE page_path IN ({}) ORDER BY page_path, rowid",
            placeholders.join(", ")
        );

        let mut stmt = self
            .conn
            .prepare(&sql)
            .map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?;

        let rows = stmt
            .query_map(params_from_iter(page_paths.iter()), |row| {
                let id: String = row.get(0)?;
                let content: String = row.get(1)?;
                let parent_id: Option<String> = row.get(2)?;
                let left_id: Option<String> = row.get(3)?;
                let properties_str: String = row.get(4)?;
                let marker: Option<String> = row.get(5)?;
                let priority: Option<String> = row.get(6)?;
                let collapsed: bool = row.get::<_, i32>(7)? != 0;
                let heading_level: Option<u8> = row.get(8)?;
                let created_at: String = row.get(9)?;
                let modified_at: String = row.get(10)?;
                let page_path: String = row.get(11)?;

                let id = Uuid::parse_str(&id).unwrap_or_else(|_| Uuid::nil());
                let properties: BTreeMap<String, String> =
                    serde_json::from_str(&properties_str).unwrap_or_default();
                let marker = marker.and_then(|m| TaskMarker::parse(&m));
                let priority = priority.and_then(|p| Priority::parse(&p));

                Ok((
                    page_path,
                    Block {
                        id,
                        content,
                        parent_id: parent_id.and_then(|s| Uuid::parse_str(&s).ok()),
                        left_id: left_id.and_then(|s| Uuid::parse_str(&s).ok()),
                        properties,
                        marker,
                        priority,
                        meta: BlockMeta {
                            collapsed,
                            heading_level,
                        },
                        created_at: DateTime::parse_from_rfc3339(&created_at)
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now()),
                        modified_at: DateTime::parse_from_rfc3339(&modified_at)
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now()),
                    },
                ))
            })
            .map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?;

        let mut result: HashMap<String, Vec<Block>> = HashMap::new();
        for row in rows {
            let (path, block) =
                row.map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?;
            result.entry(path).or_default().push(block);
        }
        Ok(result)
    }

    pub fn update_block(&self, block: &Block) -> StoreResult<()> {
        let id = block.id.to_string();
        let properties = serde_json::to_string(&block.properties)?;
        let marker = block.marker.map(|m| m.as_str().to_string());
        let priority = block.priority.map(|p| p.as_str().to_string());
        let parent_id = block.parent_id.map(|p| p.to_string());
        let left_id = block.left_id.map(|l| l.to_string());
        let modified_at = block.modified_at.to_rfc3339();

        self.conn
            .execute(
                "UPDATE blocks SET content = ?2, parent_id = ?3, left_id = ?4,
             properties = ?5, marker = ?6, priority = ?7, collapsed = ?8,
             heading_level = ?9, modified_at = ?10
             WHERE id = ?1",
                params![
                    id,
                    block.content,
                    parent_id,
                    left_id,
                    properties,
                    marker,
                    priority,
                    block.meta.collapsed as i32,
                    block.meta.heading_level,
                    modified_at,
                ],
            )
            .map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?;
        Ok(())
    }

    pub fn delete_block(&self, id: BlockId) -> StoreResult<()> {
        let id_str = id.to_string();
        let affected = self
            .conn
            .execute("DELETE FROM blocks WHERE id = ?1", params![id_str])
            .map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?;
        if affected == 0 {
            return Err(PkmError::BlockNotFound(format!("{}", id)));
        }
        Ok(())
    }

    pub fn delete_blocks_by_page(&self, page_path: &str) -> StoreResult<usize> {
        let count = self
            .conn
            .execute(
                "DELETE FROM blocks WHERE page_path = ?1",
                params![page_path],
            )
            .map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?;
        Ok(count)
    }
}
