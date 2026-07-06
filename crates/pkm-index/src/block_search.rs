//! Block-level Tantivy search index.
//!
//! Indexes individual blocks (not whole pages) for granular full-text search.

use pkm_block::Block;
use pkm_core::{PkmError, PkmResult};
use std::path::Path;
use std::sync::Arc;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::{doc, Index, IndexWriter, ReloadPolicy, TantivyDocument};
use tracing::info;

#[derive(Debug, Clone)]
pub struct BlockSearchResult {
    pub block_id: String,
    pub content: String,
    pub page_path: String,
    pub snippet: String,
    pub score: f32,
}

pub struct BlockIndex {
    index: Index,
    schema: Arc<Schema>,
    writer: Option<IndexWriter>,
}

impl BlockIndex {
    pub fn create(path: &Path) -> PkmResult<Self> {
        let mut schema_builder = Schema::builder();

        schema_builder.add_text_field("id", STRING | STORED);
        schema_builder.add_text_field("content", TEXT);
        schema_builder.add_text_field("page_path", STRING | STORED);
        schema_builder.add_text_field("marker", STRING);
        schema_builder.add_text_field("priority", STRING);
        schema_builder.add_text_field("properties", TEXT);

        let schema = Arc::new(schema_builder.build());

        let dir = path.join("blocks");
        let index = if dir.join("meta.json").exists() {
            info!("Opening existing block index at {:?}", dir);
            Index::open_in_dir(&dir)
                .map_err(|e| PkmError::Index(format!("Failed to open block index: {}", e)))?
        } else {
            info!("Creating new block index at {:?}", dir);
            std::fs::create_dir_all(&dir).map_err(PkmError::Io)?;
            Index::create_in_dir(&dir, (*schema).clone())
                .map_err(|e| PkmError::Index(format!("Failed to create block index: {}", e)))?
        };

        let writer = index
            .writer(50_000_000)
            .map_err(|e| PkmError::Index(format!("Failed to create block writer: {}", e)))?;

        Ok(Self {
            index,
            schema,
            writer: Some(writer),
        })
    }

    fn field(&self, name: &str) -> Field {
        self.schema
            .get_field(name)
            .unwrap_or_else(|_| panic!("Field '{}' not found in block schema", name))
    }

    pub fn index_block(&mut self, block: &Block, page_path: &str) -> PkmResult<()> {
        let id_str = block.id.to_string();
        let marker_str = block
            .marker
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();
        let priority_str = block
            .priority
            .map(|p| p.as_str().to_string())
            .unwrap_or_default();
        let properties_str = block
            .properties
            .iter()
            .map(|(k, v)| format!("{}:{}", k, v))
            .collect::<Vec<_>>()
            .join(" ");

        let id_field = self.field("id");
        let content_field = self.field("content");
        let page_field = self.field("page_path");
        let marker_field = self.field("marker");
        let priority_field = self.field("priority");
        let props_field = self.field("properties");

        // Delete existing doc for this block ID
        self.delete_block(block.id)?;

        let document = doc!(
            id_field => id_str.clone(),
            content_field => block.content.clone(),
            page_field => page_path.to_string(),
            marker_field => marker_str,
            priority_field => priority_str,
            props_field => properties_str,
        );

        let writer = self
            .writer
            .as_mut()
            .ok_or_else(|| PkmError::Index("Block writer not available".to_string()))?;

        writer
            .add_document(document)
            .map_err(|e| PkmError::Index(format!("Failed to add block: {}", e)))?;

        Ok(())
    }

    pub fn delete_block(&mut self, block_id: uuid::Uuid) -> PkmResult<()> {
        let id_field = self.field("id");
        let id_str = block_id.to_string();

        let writer = self
            .writer
            .as_mut()
            .ok_or_else(|| PkmError::Index("Block writer not available".to_string()))?;

        let term = tantivy::Term::from_field_text(id_field, &id_str);
        writer.delete_term(term);

        Ok(())
    }

    pub fn search(&self, query_str: &str, limit: usize) -> PkmResult<Vec<BlockSearchResult>> {
        let reader = self
            .index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .map_err(|e| PkmError::Index(format!("Failed to create reader: {}", e)))?;

        let searcher = reader.searcher();

        let content_field = self.field("content");
        let props_field = self.field("properties");

        let query_parser = QueryParser::for_index(&self.index, vec![content_field, props_field]);

        let query = query_parser
            .parse_query(query_str)
            .map_err(|e| PkmError::Search(format!("Failed to parse query: {}", e)))?;

        let top_docs = searcher
            .search(&query, &TopDocs::with_limit(limit))
            .map_err(|e| PkmError::Search(format!("Search failed: {}", e)))?;

        let id_field = self.field("id");
        let content_f = self.field("content");
        let page_field = self.field("page_path");

        let mut results = Vec::new();
        for (score, doc_address) in top_docs {
            let doc: TantivyDocument = searcher
                .doc(doc_address)
                .map_err(|e| PkmError::Search(format!("Failed to retrieve doc: {}", e)))?;

            let block_id = doc
                .get_first(id_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let content = doc
                .get_first(content_f)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let page_path = doc
                .get_first(page_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let snippet = make_snippet(&content, query_str, 120);

            results.push(BlockSearchResult {
                block_id,
                content,
                page_path,
                snippet,
                score,
            });
        }

        Ok(results)
    }

    pub fn search_by_marker(
        &self,
        marker: &str,
        limit: usize,
    ) -> PkmResult<Vec<BlockSearchResult>> {
        self.search(&format!("marker:{}", marker), limit)
    }

    pub fn delete_blocks_by_page(&mut self, page_path: &str) -> PkmResult<()> {
        let page_field = self.field("page_path");
        let writer = self
            .writer
            .as_mut()
            .ok_or_else(|| PkmError::Index("Block writer not available".to_string()))?;

        let term = tantivy::Term::from_field_text(page_field, page_path);
        writer.delete_term(term);

        Ok(())
    }

    pub fn flush(&mut self) -> PkmResult<()> {
        if let Some(ref mut writer) = self.writer {
            writer
                .commit()
                .map_err(|e| PkmError::Index(format!("Failed to commit block index: {}", e)))?;
        }
        Ok(())
    }
}

impl Drop for BlockIndex {
    fn drop(&mut self) {
        if let Some(writer) = self.writer.take() {
            if let Err(e) = writer.wait_merging_threads() {
                tracing::error!("Failed to wait for block index merge: {}", e);
            }
        }
    }
}

fn make_snippet(content: &str, query: &str, max_len: usize) -> String {
    let lower_content = content.to_lowercase();
    let lower_query = query.to_lowercase();

    if let Some(pos) = lower_content.find(&lower_query) {
        let start = pos.saturating_sub(40);
        let end = (pos + lower_query.len() + max_len - 40).min(content.len());
        let mut snippet = content[start..end].to_string();
        if start > 0 {
            snippet = format!("...{}", snippet);
        }
        if end < content.len() {
            snippet.push_str("...");
        }
        snippet
    } else if content.len() > max_len {
        let mut s = content[..max_len].to_string();
        s.push_str("...");
        s
    } else {
        content.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pkm_block::Block;
    use tempfile::TempDir;
    use uuid::Uuid;

    #[test]
    fn test_create_block_index() {
        let tmp = TempDir::new().unwrap();
        let idx = BlockIndex::create(tmp.path()).unwrap();
        drop(idx);
    }

    #[test]
    fn test_index_and_search_block() {
        let tmp = TempDir::new().unwrap();
        let mut idx = BlockIndex::create(tmp.path()).unwrap();

        let id = Uuid::new_v4();
        let block = Block::new(id, "Hello world this is a test block".into());
        idx.index_block(&block, "pages/test.md").unwrap();
        idx.flush().unwrap();

        let results = idx.search("test block", 10).unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].block_id, id.to_string());
        assert_eq!(results[0].page_path, "pages/test.md");
    }

    #[test]
    fn test_search_with_marker() {
        let tmp = TempDir::new().unwrap();
        let mut idx = BlockIndex::create(tmp.path()).unwrap();

        let id = Uuid::new_v4();
        let block = Block::new(id, "Do the dishes".into()).with_marker(pkm_block::TaskMarker::Todo);
        idx.index_block(&block, "journals/2026-01-01.md").unwrap();
        idx.flush().unwrap();

        let results = idx.search("TODO dishes", 10).unwrap();
        assert!(!results.is_empty());
    }

    #[test]
    fn test_delete_block() {
        let tmp = TempDir::new().unwrap();
        let mut idx = BlockIndex::create(tmp.path()).unwrap();

        let id = Uuid::new_v4();
        let block = Block::new(id, "Delete me".into());
        idx.index_block(&block, "pages/test.md").unwrap();
        idx.flush().unwrap();

        assert!(!idx.search("Delete me", 10).unwrap().is_empty());

        idx.delete_block(id).unwrap();
        idx.flush().unwrap();

        assert!(idx.search("Delete me", 10).unwrap().is_empty());
    }

    #[test]
    fn test_search_with_properties() {
        let tmp = TempDir::new().unwrap();
        let mut idx = BlockIndex::create(tmp.path()).unwrap();

        let id = Uuid::new_v4();
        let block = Block::new(id, "Meeting notes".into())
            .with_property("type", "meeting")
            .with_property("attendees", "alice,bob");
        idx.index_block(&block, "pages/notes.md").unwrap();
        idx.flush().unwrap();

        let results = idx.search("meeting", 10).unwrap();
        assert!(!results.is_empty());
    }

    #[test]
    fn test_search_no_results() {
        let tmp = TempDir::new().unwrap();
        let idx = BlockIndex::create(tmp.path()).unwrap();

        let results = idx.search("nonexistent", 10).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_make_snippet() {
        let content = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore.";
        let snippet = make_snippet(content, "consectetur", 80);
        assert!(snippet.contains("consectetur"));
        assert!(snippet.len() <= 120); // 40 before + query + 40 after + ...
    }
}
