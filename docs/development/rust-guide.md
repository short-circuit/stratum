# Rust Backend Development Guide

This document covers the Rust workspace architecture, coding standards, module organization, error handling, testing patterns, and common pitfalls for the Stratum PKM backend.

---

## Workspace Structure

Stratum uses a Cargo workspace with **11 crates** (10 library crates + 1 binary crate for Tauri):

```
stratum/
├── Cargo.toml                 # Workspace root
├── crates/
│   ├── pkm-core/              # Core types, config, error types
│   ├── pkm-block/             # Block model, tree, SQLite BlockStore
│   ├── pkm-markdown/          # Block-based markdown parser + serializer
│   ├── pkm-index/             # Backlinks, graph engine, Tantivy search
│   ├── pkm-query/             # Datalog query engine
│   ├── pkm-sync/              # Git sync engine (git2 wrapper)
│   ├── pkm-watcher/           # File system watcher (notify)
│   ├── pkm-ai/                # Embeddings, RAG, LLM provider interface
│   ├── pkm-plugin/            # WASM plugin runtime (wasmtime)
│   └── pkm-cli/               # CLI binary
└── src-tauri/                 # Tauri v2 desktop app (binary + command handlers)
```

### Crate Purposes

| Crate | Responsibility | Key Types |
|-------|---------------|-----------|
| `pkm-core` | Foundation types, `PkmError`, config, frontmatter | `PkmError`, `Config`, `Frontmatter`, `SyncMode` |
| `pkm-block` | Block data model, SQLite `BlockStore`, tree operations | `Block`, `BlockStore`, `Page`, `TaskMarker`, `Priority` |
| `pkm-markdown` | `.md` parser/serializer, wiki-link extraction | `parse_document()`, `serialize_blocks()`, `extract_links()` |
| `pkm-index` | Graph building, Tantivy search, backlinks, tag index | `IndexEngine`, `BlockIndex`, `build_graph_from_store()` |
| `pkm-query` | Datalog query execution against block store | `run_query()` |
| `pkm-sync` | Git operations via git2 | `GitEngine`, `AutoCommitEngine`, `SyncScheduler` |
| `pkm-watcher` | Filesystem change detection | `FileWatcher` |
| `pkm-ai` | LLM provider interface, RAG, embeddings | `AiProvider`, `EmbeddingEngine` |
| `pkm-plugin` | WASM plugin sandbox | `PluginRuntime` |
| `pkm-cli` | Terminal interface | `main()` with clap args |

### Dependency Graph

```
pkm-core       -> (standalone foundation)
pkm-block      -> pkm-core
pkm-markdown   -> pkm-core, pkm-block
pkm-index      -> pkm-core, pkm-markdown, pkm-block
pkm-query      -> pkm-block
pkm-sync       -> pkm-core, pkm-markdown
pkm-watcher    -> pkm-core, pkm-markdown, pkm-index
pkm-ai         -> pkm-core, pkm-index
pkm-plugin     -> pkm-core
pkm-cli        -> pkm-core, pkm-markdown, pkm-index, pkm-sync, pkm-watcher, pkm-ai, pkm-plugin
src-tauri      -> pkm-core, pkm-block, pkm-markdown, pkm-index, pkm-query, pkm-sync, pkm-watcher, pkm-ai
```

Note: `src-tauri` does NOT depend on `pkm-plugin`. The CLI does NOT depend on `pkm-block` or `pkm-query`.

---

## Error Handling

### Single Error Type: `PkmError`

All crate errors use a single unified type defined in `pkm-core`. Never define new error types in individual crates. Extend `PkmError` variants instead.

```rust
// crates/pkm-core/src/error.rs
use thiserror::Error;

/// Unified error type for the Stratum PKM system.
#[derive(Error, Debug)]
pub enum PkmError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Markdown parse error: {0}")]
    MarkdownParse(String),

    #[error("Block not found: {0}")]
    BlockNotFound(String),

    #[error("Page not found: {0}")]
    PageNotFound(String),

    #[error("Index error: {0}")]
    Index(String),

    #[error("Git error: {0}")]
    Git(String),

    #[error("AI provider error: {0}")]
    Ai(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Already exists: {0}")]
    AlreadyExists(String),

    #[error("Unsupported: {0}")]
    Unsupported(String),
    // ... more variants as needed
}

/// Convenience alias used across all crates.
pub type PkmResult<T> = Result<T, PkmError>;
```

### Using `#[from]` Derives

Use `#[from]` for standard library conversions:

```rust
// crates/pkm-core/src/error.rs
#[error("IO error: {0}")]
Io(#[from] std::io::Error),
```

For external crate errors, implement `From` manually:

```rust
impl From<rusqlite::Error> for StoreError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sqlite(e)
    }
}
```

However, note that `pkm-block/src/store.rs` defines its own `StoreError` type. This is a known area for refactoring. The `StoreError` should ideally be converted to `PkmError` at the crate boundary. Command handlers in `src-tauri` catch these and convert them to `String` anyway:

```rust
// src-tauri/src/commands/vault.rs
let store = pkm_block::BlockStore::open(&state.db_path).map_err(|e| e.to_string())?;
```

### Adding a New Error Variant

To add a new error variant:

1. Add the variant to `PkmError` in `crates/pkm-core/src/error.rs`
2. Add an `#[error("...")]` display message
3. If converting from an external error, add a `From` impl
4. Update the `test_all_variants_display` test to include the new variant

### What NOT to Do

Do NOT define new error enums in individual crates:

```rust
// ❌ WRONG — do not define crate-level error types
#[derive(Error, Debug)]
pub enum MyCrateError {
    #[error("Something went wrong")]
    Something(String),
}

// ✅ CORRECT — extend PkmError
// In pkm-core/src/error.rs:
#[error("My crate error: {0}")]
MyCrate(String),
```

---

## Comment Standards

Every Rust source file must follow these conventions:

### Module-Level Docs (`//!`)

Every file starts with a `//!` doc explaining what the module provides and how to use it.

```rust
//! Block management commands.
//!
//! Thin Tauri command handlers that delegate to pkm_block and pkm_markdown.
//! DTOs in this module mirror the TypeScript types in src/lib/types.ts.
```
— `src-tauri/src/commands/block.rs`

```rust
//! SQLite-backed storage for blocks and pages.
//!
//! Primary storage layer for the hybrid SQLite+.md architecture.
//! All CRUD operations happen against SQLite first; .md files are
//! written asynchronously for portability and git sync.
```
— `crates/pkm-block/src/store.rs`

### Public Function Docs (`///`)

Every public function gets a doc comment covering: what it does, arguments, return value, errors.

```rust
/// Open an existing repo at `path`, or initialise a new one if none exists.
pub fn init<P: AsRef<Path>>(path: P) -> PkmResult<Self> {
```
— `crates/pkm-sync/src/git.rs`

```rust
/// Strip Stratum block property lines from content that was previously saved in block format.
/// Removes `.id:`, `.marker:`, `.priority:` lines. For `.heading-level: N` lines, rewrites
/// the preceding block line with ATX heading markers (e.g., `## `) to preserve heading level.
fn strip_block_properties(content: &str) -> String {
```
— `src-tauri/src/commands/page.rs`

### Inline Comments

Use inline comments for:

**Complex algorithms** — explain state invariants and why the approach works:

```rust
// Build edges by scanning blocks for [[wiki-links]]
let mut outgoing: HashMap<String, Vec<GraphEdgeDto>> = HashMap::new();
let mut degree: HashMap<String, usize> = HashMap::new();

for (slug, page_path) in &meta.slug_to_path {
    if let Ok(blocks) = store.get_blocks_by_page(page_path) {
        for block in &blocks {
            let links = pkm_markdown::linker::extract_links(&block.content);
            for link in links {
                let target_slug = meta.resolve_slug(&link.target);
                if let Some(target) = target_slug {
                    // Degree: source always +1, target +1 only if different
                    // (avoid double-count self-links)
                    *degree.entry(slug.clone()).or_default() += 1;
                    if target != *slug {
                        *degree.entry(target.clone()).or_default() += 1;
                    }
                }
            }
        }
    }
}
```
— `src-tauri/src/commands/graph.rs`

**Non-obvious transformations** — explain why this mapping exists:

```rust
// Strip block property lines before conversion to avoid .id: uuid
// appearing as paragraph blocks
let cleaned = strip_block_properties(&content);
let (fm, _, blocks) = pkm_markdown::block_parser::parse_document_as_plain_markdown(&cleaned);
```
— `src-tauri/src/commands/page.rs`

**Workarounds** — reference the issue or limitation being worked around:

```rust
// Ollama response: { "models": [ { "name": "model-name", ... }, ... ] }
let models: Vec<String> = if let Some(data) = body.get("data").and_then(|d| d.as_array()) {
    // OpenAI-compatible response format
    data.iter()
        .filter_map(|m| m.get("id").and_then(|id| id.as_str()))
        .map(|s| s.to_string())
        .collect()
} else if let Some(data) = body.get("models").and_then(|d| d.as_array()) {
    // Ollama response format
```
— `src-tauri/src/commands/settings.rs`

---

## Module Organization

### Command Handlers Are Thin Glue

Files in `src-tauri/src/commands/` must be **thin glue layers**. Their job is:

1. Accept Tauri state and arguments
2. Lock the `Mutex<VaultState>`
3. Call a business logic crate function
4. Map the result to a DTO
5. Return `Result<Dto, String>`

| Command Handler | Business Logic Lives In |
|----------------|------------------------|
| `graph.rs` | `pkm-block` (graph building), `pkm-markdown` (link extraction) |
| `settings.rs` | `pkm-core::Config` (DTO mapping) |
| `search.rs` | `pkm-index` (search, backlinks) |
| `sync.rs` | `pkm-sync` (git operations) |
| `page.rs` | `pkm-block` (BlockStore), `pkm-markdown` (parsing) |
| `block.rs` | `pkm-block` (BlockStore), `pkm-markdown` (serialization) |

### What Belongs Where

**In the command handler (thin layer):**

```rust
// src-tauri/src/commands/search.rs
#[tauri::command]
pub async fn search_blocks(
    query: String,
    limit: Option<usize>,
    state: tauri::State<'_, AppState>,
) -> Result<SearchResultsDto, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let limit = limit.unwrap_or(20);

    let index_path = state.vault_path.join(".pkm").join("search");
    let block_index =
        pkm_index::block_search::BlockIndex::create(&index_path).map_err(|e| e.to_string())?;

    let results = block_index
        .search(&query, limit)
        .map_err(|e| e.to_string())?;

    let dtos: Vec<SearchResultDto> = results
        .into_iter()
        .map(|r| SearchResultDto {
            block_id: r.block_id,
            content: r.content,
            page_path: r.page_path,
            snippet: r.snippet,
            score: r.score,
        })
        .collect();

    Ok(SearchResultsDto { results: dtos })
}
```

**In the business logic crate (where the real work happens):**

```rust
// crates/pkm-index/src/block_search.rs
impl BlockIndex {
    pub fn create(path: &Path) -> PkmResult<Self> {
        let mut schema_builder = Schema::builder();
        schema_builder.add_text_field("id", STRING | STORED);
        schema_builder.add_text_field("content", TEXT);
        schema_builder.add_text_field("page_path", STRING | STORED);
        // ...
        let index = Index::create_in_dir(&dir, (*schema).clone())
            .map_err(|e| PkmError::Index(format!("Failed to create block index: {}", e)))?;
        Ok(Self { index, schema, writer: Some(writer) })
    }

    pub fn search(&self, query: &str, limit: usize) -> PkmResult<Vec<BlockSearchResult>> {
        // Tantivy search implementation lives here, not in the command handler
    }
}
```

### Registering Commands

All commands are registered in `src-tauri/src/lib.rs`:

```rust
// src-tauri/src/lib.rs
.invoke_handler(tauri::generate_handler![
    commands::vault::get_vault_info,
    commands::vault::set_vault_path,
    commands::page::list_pages,
    commands::page::save_page,
    commands::search::search_blocks,
    commands::graph::get_graph_data,
    commands::graph::get_connected_components,
    commands::sync::sync_vault,
    // ... 70+ commands
])
```

The corresponding TypeScript wrappers live in `src/lib/commands.ts`. When you add a new Rust command, you must add a matching wrapper there too.

---

## Key Patterns

### The `AppState = Mutex<VaultState>` Pattern

Shared state is managed as `Mutex<VaultState>`. It wraps the vault path, database path, index engine, sync scheduler, and optional passphrase.

```rust
// src-tauri/src/commands/vault.rs

/// Application state holding the active vault.
pub struct VaultState {
    pub vault_path: PathBuf,
    pub db_path: PathBuf,
    pub index_engine: Option<IndexEngine>,
    pub sync_scheduler: Option<pkm_sync::SyncScheduler>,
    pub auto_commit_engine: Option<pkm_sync::AutoCommitEngine>,
    pub passphrase: Option<String>,
}

pub type AppState = Mutex<VaultState>;
```

Initialized in `lib.rs` during Tauri setup:

```rust
// src-tauri/src/lib.rs
app.manage(Mutex::new(VaultState::new(vault_path.clone())) as AppState);
```

Usage in commands:

```rust
#[tauri::command]
pub async fn get_vault_info(state: tauri::State<'_, AppState>) -> Result<VaultInfo, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    // ... use state.vault_path, state.db_path, etc.
}
```

### Command Handler Pattern

Every Tauri command follows this shape:

```rust
#[tauri::command]
pub async fn command_name(
    // 1. Arguments from the frontend
    arg1: String,
    arg2: Option<usize>,
    // 2. State (always last)
    state: tauri::State<'_, AppState>,
) -> Result<SomeDto, String> {   // 3. Return Result<Dto, String>
    // 4. Lock the mutex
    let state = state.lock().map_err(|e| e.to_string())?;

    // 5. Delegate to business logic crate
    let result = pkm_some_crate::do_thing(&state.db_path)
        .map_err(|e| e.to_string())?;

    // 6. Return DTO
    Ok(SomeDto { field: result })
}
```

### DTO Pattern

All DTOs use `#[derive(Debug, Serialize, Deserialize, Clone)]` and mirror the TypeScript types in `src/lib/types.ts`.

```rust
// src-tauri/src/commands/search.rs
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResultDto {
    pub block_id: String,
    pub content: String,
    pub page_path: String,
    pub snippet: String,
    pub score: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResultsDto {
    pub results: Vec<SearchResultDto>,
}
```

Nested DTOs for complex responses:

```rust
// src-tauri/src/commands/graph.rs
#[derive(Debug, Clone, Serialize)]
pub struct GraphDataDto {
    pub nodes: Vec<GraphNodeDto>,
    pub edges: Vec<GraphEdgeDto>,
    pub node_count: usize,
    pub edge_count: usize,
    pub vault_path: String,
}
```

### The `PageMetaIndex` Pattern

The `PageMetaIndex` struct in `graph.rs` eliminates duplicated map-building across all graph-related operations. Before this pattern, `build_graph_data`, `get_connected_components`, `get_orphaned_notes`, and `resolve_link_target` each built their own `slug -> path` and `slug -> title` maps from `store.list_pages()`.

```rust
// src-tauri/src/commands/graph.rs

/// Pre-built index of all pages in the vault for fast slug/title/path resolution.
/// Eliminates duplicated map-building across graph, connected components,
/// orphans, and link resolution.
struct PageMetaIndex {
    slug_to_path: HashMap<String, String>,
    slug_to_title: HashMap<String, String>,
    slug_to_tags: HashMap<String, Vec<String>>,
    title_to_slug: HashMap<String, String>,
}

impl PageMetaIndex {
    fn from_store(store: &pkm_block::BlockStore) -> Result<Self, String> {
        let paths = store.list_pages().map_err(|e| e.to_string())?;
        let mut slug_to_path = HashMap::new();
        let mut slug_to_title = HashMap::new();
        let mut slug_to_tags = HashMap::new();
        let mut title_to_slug = HashMap::new();

        for path in &paths {
            let slug = slug_from_path(path);
            slug_to_path.insert(slug.clone(), path.clone());

            let fm = store.get_page(path).ok().flatten();
            let title = fm.as_ref()
                .and_then(|f| f.title.clone())
                .unwrap_or_else(|| slug.replace('-', " "));
            slug_to_title.insert(slug.clone(), title.clone());
            title_to_slug.insert(title.to_lowercase(), slug.clone());

            let tags = fm.map(|f| f.tags).unwrap_or_default();
            slug_to_tags.insert(slug, tags);
        }

        Ok(Self { slug_to_path, slug_to_title, slug_to_tags, title_to_slug })
    }

    fn resolve_slug(&self, target: &str) -> Option<String> {
        let slugified = target.replace(' ', "-").to_lowercase();
        if self.slug_to_path.contains_key(&slugified) {
            return Some(slugified);
        }
        if self.slug_to_path.contains_key(target) {
            return Some(target.to_string());
        }
        let lower = target.to_lowercase();
        if let Some(slug) = self.title_to_slug.get(&lower) {
            return Some(slug.clone());
        }
        None
    }

    fn get_node(&self, slug: &str) -> GraphNodeDto {
        GraphNodeDto {
            id: slug.to_string(),
            title: self.slug_to_title.get(slug).cloned().unwrap_or_default(),
            path: self.slug_to_path.get(slug).cloned().unwrap_or_default(),
            tags: self.slug_to_tags.get(slug).cloned().unwrap_or_default(),
            degree: 0, // caller sets degree
        }
    }
}
```

Used in every graph function:

```rust
fn build_graph_data_from_store(
    store: &pkm_block::BlockStore,
    vault_path: &str,
) -> Result<GraphDataDto, String> {
    let meta = PageMetaIndex::from_store(store)?;
    // ... use meta.slug_to_path, meta.resolve_slug(), meta.get_node()
}
```

When building a new command that needs page metadata, reuse or extend `PageMetaIndex` rather than duplicating the map-building loop.

---

## Testing

### Unit Tests Alongside Implementation

Tests go in `#[cfg(test)]` modules at the bottom of the same file, co-located with the code they test:

```rust
// src-tauri/src/commands/graph.rs

#[cfg(test)]
mod tests {
    use super::*;
    use pkm_block::{Block, BlockStore, Page};
    use std::path::{Path, PathBuf};
    use uuid::Uuid;

    fn insert_test_page(store: &BlockStore, vault_root: &Path, rel_path: &str) {
        let full_path = vault_root.join(rel_path);
        let page = Page::new(full_path, vault_root);
        store.upsert_page(&page).unwrap();
    }

    #[test]
    fn test_self_link_included_in_edges() {
        let store = BlockStore::open_in_memory().unwrap();
        let vault_root = PathBuf::from("/tmp/test-vault");
        let slug = "test-self-link";
        let rel_path = format!("pages/{}.md", slug);

        insert_test_page(&store, &vault_root, &rel_path);

        let block = Block::new(Uuid::new_v4(), format!("Self reference [[{}]]", slug));
        store.insert_block(&block, &rel_path).unwrap();

        let data = build_graph_data_from_store(&store, "/tmp/test-vault").unwrap();

        assert_eq!(data.node_count, 1, "should have 1 node");
        assert_eq!(data.edge_count, 1, "should have 1 self-link edge");
        assert_eq!(data.edges[0].source, slug, "edge source should be the slug");
        assert_eq!(data.nodes[0].degree, 1,
            "self-link degree should be 1 (not double-counted)");
    }

    #[test]
    fn test_self_link_does_not_affect_connected_components() {
        let store = BlockStore::open_in_memory().unwrap();
        let vault_root = PathBuf::from("/tmp/test-vault");

        insert_test_page(&store, &vault_root, "pages/page-a.md");
        insert_test_page(&store, &vault_root, "pages/page-b.md");

        let block_a = Block::new(Uuid::new_v4(), "[[page-a]]".into());
        store.insert_block(&block_a, "pages/page-a.md").unwrap();

        let block_b = Block::new(Uuid::new_v4(), "[[page-a]]".into());
        store.insert_block(&block_b, "pages/page-b.md").unwrap();

        let components = get_connected_components_from_store(&store).unwrap();
        assert_eq!(components.len(), 1, "should have 1 connected component");
        assert_eq!(components[0].size, 2, "component should contain both pages");
    }
}
```

### In-Memory SQLite for Testing

Use `BlockStore::open_in_memory()` for tests that don't need filesystem I/O:

```rust
// crates/pkm-block/src/store.rs
#[test]
fn test_find_blocks_by_markers() {
    let store = BlockStore::open_in_memory().unwrap();
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();

    store.insert_block(
        &Block::new(a, "Task 1".into()).with_marker(TaskMarker::Todo),
        "pages/tasks.md",
    ).unwrap();
    store.insert_block(
        &Block::new(b, "Task 2".into()).with_marker(TaskMarker::Done),
        "pages/tasks.md",
    ).unwrap();

    let todos = store.find_blocks_by_marker("TODO").unwrap();
    assert_eq!(todos.len(), 1);
    assert_eq!(todos[0].id, a);
}
```

### Using `tempfile` for Filesystem Tests

When tests need real filesystem access, use the `tempfile` crate:

```rust
use tempfile::TempDir;

#[test]
fn test_page_creation() {
    let dir = TempDir::new().unwrap();
    let vault_path = dir.path().join("vault");
    std::fs::create_dir_all(&vault_path).unwrap();

    // Test with real filesystem paths
    let db_path = vault_path.join(".pkm").join("blocks.db");
    let store = BlockStore::open(&db_path).unwrap();
    // ...
}
```

### Test Target

Every public function should have at least one test. Focus on:

- **Edge cases**: self-links, empty queries, missing pages
- **Error paths**: invalid paths, corrupted data
- **Round-trips**: serialize -> deserialize -> compare
- **Boundary conditions**: large inputs, special characters

### Running Tests

```bash
# All workspace tests
cargo test --workspace

# Single crate
cargo test -p pkm-block

# Single test
cargo test -p pkm-block -- test_find_blocks_by_markers

# With output
cargo test -- --nocapture
```

---

## Avoiding Common Mistakes

### 1. Don't Hold `MutexGuard` Across `.await` Points

The `Mutex` from `std::sync::Mutex` is not `Send`. Holding the lock guard across an `.await` will cause a compile error or a runtime deadlock.

```rust
// ❌ WRONG — MutexGuard held across .await
#[tauri::command]
pub async fn fetch_models(state: tauri::State<'_, AppState>) -> Result<Vec<String>, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    // ... uses config from state ...
    let response = reqwest::get(&url).await.map_err(|e| e.to_string())?;
    // ^^^ ERROR: MutexGuard is not Send, cannot hold across .await
}

// ✅ CORRECT — extract data before the async call
#[tauri::command]
pub async fn fetch_models(state: tauri::State<'_, AppState>) -> Result<Vec<String>, String> {
    // Extract config data before async operation (MutexGuard is not Send)
    let (endpoint, api_key) = {
        let state = state.lock().map_err(|e| e.to_string())?;
        let config_path = state.vault_path.join(".pkm").join("config.toml");
        let config = pkm_core::Config::load(&config_path).map_err(|e| e.to_string())?;
        (config.ai.endpoint.unwrap_or_default(), config.ai.api_key)
    };
    // MutexGuard dropped here — safe to .await

    let response = reqwest::get(&format!("{}/v1/models", endpoint))
        .await
        .map_err(|e| e.to_string())?;
    // ...
}
```
— `src-tauri/src/commands/settings.rs`

### 2. Don't Define New Error Types

Always extend `PkmError` rather than creating crate-level error enums.

```rust
// ❌ WRONG — crate-level StoreError in pkm-block/src/store.rs (existing tech debt)
pub enum StoreError {
    Sqlite(rusqlite::Error),
    NotFound(String),
    Serialization(String),
}

// ✅ CORRECT — add variants to PkmError in pkm-core
// pkm-core/src/error.rs:
#[error("Store error: {0}")]
Store(String),

// Then in pkm-block:
impl From<rusqlite::Error> for PkmError { ... }
```

### 3. Don't Put Business Logic in Command Handlers

Command handlers should not contain complex algorithms, SQL queries, or parsing logic.

```rust
// ❌ WRONG — graph building logic in the command handler
#[tauri::command]
pub async fn get_graph_data(state: ...) -> Result<..., String> {
    let state = state.lock()...;
    // 100 lines of graph building here — BAD

// ✅ CORRECT — delegate to a crate function
#[tauri::command]
pub async fn get_graph_data(state: ...) -> Result<..., String> {
    let state = state.lock()...;
    let store = BlockStore::open(&state.db_path)...;
    let data = build_graph_data_from_store(&store, &vault_path_str)?;
    Ok(data)
}
```

### 4. Don't Duplicate `slug_from_path`

The utility to derive a slug from a file path appears inline in multiple places. Use the shared `PageMetaIndex` or extract a helper.

```rust
// ❌ WRONG — duplicated in page.rs, search.rs, graph.rs
let slug = std::path::Path::new(&path)
    .file_stem()
    .and_then(|s| s.to_str())
    .unwrap_or("untitled")
    .to_string();

// ✅ CORRECT — use slug_from_path in graph.rs (or extract to shared utility)
fn slug_from_path(path: &str) -> String {
    std::path::Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("untitled")
        .to_string()
}
```

---

## Code Style

### Edition and Toolchain

- **Edition 2021** — set in workspace `Cargo.toml` and `rust-toolchain.toml`
- **Formatter**: `cargo fmt` (runs on save in rust-analyzer)
- **Linter**: `cargo clippy -- -D warnings` (deny all warnings in CI)

```toml
# rust-toolchain.toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy", "rust-analyzer"]
```

### Import Organization

Group imports by: standard library, external crates, internal crates.

```rust
// Standard library
use std::collections::HashMap;
use std::path::PathBuf;

// External crates
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Internal crates
use crate::commands::vault::AppState;
use pkm_block::BlockStore;
```

### Naming Conventions

| Construct | Convention | Example |
|-----------|-----------|---------|
| Types/Enums | PascalCase | `VaultState`, `PkmError`, `BlockStore` |
| Functions | snake_case | `build_graph_data_from_store`, `slug_from_path` |
| DTOs | PascalCase + `Dto` suffix | `GraphNodeDto`, `SearchResultDto`, `PageDto` |
| Modules | snake_case | `block_search`, `auto_commit` |
| Error variants | PascalCase | `NoteNotFound`, `Index`, `Validation` |

### Function Length

Keep command handler functions under 30 lines. If a command handler grows beyond that, extract the business logic into a private function or move it to the appropriate crate.

---

## Dependencies

### AGPL-3.0 Compatibility

Every new dependency (Rust crate or npm package) must be license-compatible with AGPL-3.0. Always acceptable licenses:

- MIT, Apache-2.0, BSD-2/3-Clause, ISC, Zlib, Unlicense, CC0-1.0, BSL-1.0
- MPL-2.0 (AGPL-compatible per MPL Section 3.3)
- Apache-2.0 WITH LLVM-exception
- Unicode-3.0
- Dual-licensed dependencies where at least one option is in the above list

**Reject** any dependency that is:

- GPL-2.0-only without a permissive dual-license alternative
- A proprietary or non-OSI-approved license

### Adding a New Dependency

1. Add the dependency to `[workspace.dependencies]` in the root `Cargo.toml`
2. Reference it in the specific crate's `Cargo.toml` using `workspace = true`
3. Verify license compatibility
4. Update `Cargo.lock` with `cargo build --workspace`

---

## Performance Targets

| Metric | Target |
|--------|--------|
| Cold start (empty vault) | < 500ms |
| Vault with 10k notes — index rebuild | < 30s |
| Full-text search (10k notes) | < 100ms |
| Graph load (10k notes) | < 2s |
| Note save latency | < 50ms (excluding git commit) |
| Memory (idle, desktop) | < 80MB |
| Memory (10k notes, desktop) | < 200MB |

When profiling, use `RUST_LOG=info` or add `eprintln!("[stratum:module] ...")` markers for timing. For example, the graph command logs timing information:

```rust
eprintln!(
    "[stratum:graph] Found {} nodes, {} edges",
    data.node_count, data.edge_count
);
```
